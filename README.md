# Beckon

A background-resident LLM shortcut for Windows: press a hotkey to summon it, send a preset prompt plus your current input to DeepSeek, and get the result streamed back in a popover next to your cursor. The name comes from "beckoning" — you wave, it comes.

For terminology see [CONTEXT.md](./CONTEXT.md); for architectural decisions see [docs/adr/](./docs/adr/).

## MVP scope

**In scope**

- Lives in the tray, starts with Windows
- Global hotkey summons the Launcher (a searchable list of Actions)
- An Action can also be bound to a Direct Hotkey — press it and go straight to the result with zero interaction
- Simulated Ctrl+C to grab the Selection, with the clipboard restored afterwards
- Popover near the cursor: takes focus, streams output, supports follow-up turns, closes on Esc
- Full settings window: API key, global hotkey, theme, global model defaults, and the Actions themselves — one Actions section listing every Action, each opening into its own editor
- Actions stored as TOML files, with a file watcher that reloads them automatically on external changes
- OpenAI-compatible API, with a configurable `base_url`

**Explicitly out of scope**

| Not doing | Why |
| --- | --- |
| Persisting Exchanges / history / search | [ADR-0004](./docs/adr/0004-exchanges-are-never-persisted.md) |
| Action categories / tags | At a scale of a dozen or so Actions, fuzzy search is already enough; subdirectories can do this later at zero migration cost |
| Parameterized Actions (choosing/filling variables at invocation time) | Breaks "press it and just wait for the result"; bidirectional translation can be left to the model to figure out in the system prompt |
| "Replace the original text" write-back | [ADR-0002](./docs/adr/0002-selection-via-simulated-ctrl-c.md) |
| Auto-popping a small icon after selecting text (PopClip style) | Requires globally polling the selection — power-hungry, easy to trigger by accident, and it fights the Ctrl+C grab approach |
| Token usage display | A flash translation costs a fraction of a cent; seeing the number changes nothing |
| macOS / Linux | [ADR-0001](./docs/adr/0001-tauri-v2-on-windows-only.md), but platform-specific code must stay isolated |

## Decided behavior

**Triggering and grabbing text**

- The global hotkey defaults to `Ctrl+Alt+Space`. This avoids Microsoft Pinyin's `Ctrl+Space` (Chinese/English toggle), `Shift+Space` (full-width/half-width), and `Win+Space` (switch IME), as well as `Alt+Space` (system window menu, and commonly taken by PowerToys Run / uTools).
- If the grab comes back empty, handle it according to the Action's `input_source` — this is **not an error**: `selection` shows a hint and sends no request, `auto` falls through to the input box.
- The Popover always takes focus. Remember the previous foreground window handle before showing it, and hand focus back on close.

**Failure and waiting**

- No timeout. When the network is down the HTTP layer errors immediately, and the Popover shows the error rather than spinning.
- The UI must distinguish "waiting for the first token" from "currently streaming" — otherwise there is no way to tell whether the request is still alive.
- Request failed → show the error inline in the Popover with a retry button; no system notification.
- Stream cut off midway → keep whatever was already output and mark it "interrupted" underneath.
- Esc cancels the request at any time.

**Configuration**

- When recording a hotkey in the settings window, **attempt to register it immediately**; if it is taken, flag it red on the spot and refuse to save a hotkey that cannot be registered.
- A hotkey registration failure at startup is never silent: the tray icon switches to an error state plus a one-time balloon notification that opens settings when clicked.
- The model is **chosen from a list, not typed**. The list is the endpoint's own `/v1/models` response when a key is stored and the request succeeds, and the officially documented DeepSeek models otherwise — no credential, a rejected key or a dead network downgrades the list and says why, but never empties it. A model already named in `config.toml` or in an Action stays selectable even when nothing vouches for it, flagged rather than rewritten.
- On first run (no key readable from the Credential Manager), open the settings window directly, including a "Test connection" button — it sends a minimal request to verify the key and `base_url` on the spot.
- On first run, if `actions/` does not exist, write two example Actions (one `selection` type and one `prompt` type) covering both main paths. Once deleted, they are not regenerated.
- The theme is `light`, `dark`, or `system`, and it applies to all three surfaces at once. **The default is `light`**, including on a machine whose Windows app theme is dark: the system preference is read only by `theme = "system"`, which has to be chosen.

## Config file layout

```
%APPDATA%\Beckon\
├── config.toml        # global hotkey, autostart, theme, base_url, global model defaults
└── actions\
    ├── translate.toml
    └── ask.toml
```

The API key is **not here** — it lives in the Windows Credential Manager under the service name `Beckon` ([ADR-0005](./docs/adr/0005-api-key-in-windows-credential-manager.md)). There is no plaintext secret file anywhere on disk.

An Action's **identity is its filename**; the `name` field is only for display.

### config.toml

```toml
launcher_hotkey = "Ctrl+Alt+Space"
autostart = true
theme = "light"                 # light | dark | system

[api]
base_url = "https://api.deepseek.com"

[defaults]
model = "deepseek-v4-flash"
thinking = false
temperature = 1.3
```

### actions/translate.toml

```toml
name = "Translate"
description = "Chinese <-> English"
input_source = "selection"      # selection | prompt | auto
hotkey = "Ctrl+Alt+T"           # optional; if omitted, only callable from the Launcher

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""
# user may be omitted; it defaults to "{{input}}"

[model]
temperature = 1.3               # all three of these may be omitted, falling back to defaults in config.toml
```

### actions/ask.toml

```toml
name = "Quick ask"
input_source = "prompt"

[prompt]
system = "Answer concisely. Unless asked, do not enumerate bullet points and do not preamble at length."

[model]
thinking = true
```

## Tech stack

Tauri v2 (Rust + web UI). For the reasoning and the rejected alternatives, see [ADR-0001](./docs/adr/0001-tauri-v2-on-windows-only.md).

DeepSeek is accessed via the OpenAI-compatible format at `https://api.deepseek.com`. Current models are `deepseek-v4-flash` / `deepseek-v4-pro`, 1M context, with **thinking mode on by default** — which is why `thinking = false` exists in the global defaults: leaving thinking on for translation-type Actions adds several seconds of latency and a pile of reasoning tokens for nothing. The legacy names `deepseek-chat` / `deepseek-reasoner` were discontinued on 2026-07-24; Beckon still recognises them so an old config keeps working, but does not offer them.
