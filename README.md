# Beckon

English | [简体中文](./README.zh-CN.md)

A background-resident LLM shortcut for Windows and macOS: press a hotkey to summon it, send a preset prompt plus your current input to DeepSeek, and get the result streamed back in a popover next to your cursor. The name comes from "beckoning" — you wave, it comes.

For terminology see [CONTEXT.md](./CONTEXT.md); for architectural decisions see [docs/adr/](./docs/adr/).

## MVP scope

**In scope**

- Lives in the tray (the menu bar on macOS), starts with the machine
- Global hotkey summons the Launcher (a searchable list of Actions)
- An Action can also be bound to a Direct Hotkey — press it and go straight to the result with zero interaction
- The platform's copy shortcut is simulated to grab the Selection — Ctrl+C, or Cmd+C on macOS — with the clipboard restored afterwards
- Popover near the cursor: takes focus, streams output, supports follow-up turns, closes on Esc
- A screenshot button in the Popover: it runs the platform's own snip tool, attaches the result as a Capture, and sends it with whatever is typed beside it ([ADR-0016](./docs/adr/0016-captures-from-the-os-snip-tool-via-the-clipboard.md)). The image goes to whichever model the Action names; whether that model reads images is the endpoint's answer to give
- Full settings window: API key, global hotkey, theme, language, global model defaults, and the Actions themselves — one Actions section listing every Action, each opening into its own editor
- English and Simplified Chinese, switched in Settings ([ADR-0015](./docs/adr/0015-english-and-chinese-from-one-config-field.md))
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
| Linux | [ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md) ports the platform layer to macOS; nobody has asked for a third. The stubs in `platform/fallback.rs` keep the crate compiling elsewhere, and are not a promise |
| Code signing and notarization | Not set up on either platform. On macOS this is the difference between "runs on the machine that built it" and "runs on anyone else's" — see ADR-0013 |

## Decided behavior

**Triggering and grabbing text**

- The global hotkey defaults to `Ctrl+Shift+Space` on Windows and `Cmd+Shift+Space` on macOS: Space with two modifiers on both, and only the first differs, because the platform's own launcher does — Spotlight is `Cmd+Space`. Both avoid the IME chords (`Ctrl+Space` Chinese/English on Microsoft Pinyin and "previous input source" on macOS, `Shift+Space` full-width/half-width, `Win+Space` switch IME), `Alt+Space` (system window menu, and commonly taken by PowerToys Run / uTools), and any `Ctrl+Alt` combination, which is `AltGr` on an ISO keyboard and so composes characters as well as the modifier state the grab must release before it can copy.
- If the grab comes back empty, handle it according to the Action's `input_source` — this is **not an error**: `selection` shows a hint and sends no request, `auto` falls through to the input box.
- The Popover always takes focus. Remember what was in front before showing it — the window on Windows, the application on macOS — and hand focus back on close.
- On macOS the grab needs Accessibility permission, and the OS refuses it **silently**: the Selection just comes back empty. Settings reads the permission directly and says so, with a link to the pane; the hotkey itself still fires either way.

**Failure and waiting**

- No timeout. When the network is down the HTTP layer errors immediately, and the Popover shows the error rather than spinning.
- The UI must distinguish "waiting for the first token" from "currently streaming" — otherwise there is no way to tell whether the request is still alive.
- Request failed → show the error inline in the Popover with a retry button; no system notification.
- Stream cut off midway → keep whatever was already output and mark it "interrupted" underneath.
- Esc cancels the request at any time.
- A Capture is attached, then sent — never captured-and-sent. A cancelled snip is **not an error**: nothing was captured, nothing was sent, and the Popover says so. A screenshot that cannot be sent — over Beckon's 8 MB ceiling, or bytes no decoder reads — is named as its own cause, distinct from a cancel.

**Configuration**

- When recording a hotkey in the settings window, **attempt to register it immediately**; if it is taken, flag it red on the spot and refuse to save a hotkey that cannot be registered.
- A hotkey registration failure at startup is never silent: the tray icon switches to an error state plus a one-time balloon notification that opens settings when clicked.
- The model is **chosen from a list, not typed**. The list is the endpoint's own `/v1/models` response when a key is stored and the request succeeds, and the officially documented DeepSeek models otherwise — no credential, a rejected key or a dead network downgrades the list and says why, but never empties it. A model already named in `config.toml` or in an Action stays selectable even when nothing vouches for it, flagged rather than rewritten.
- On first run (no key readable from the credential store), open the settings window directly, including a "Test connection" button — it sends a minimal request to verify the key and `base_url` on the spot.
- On first run, if `actions/` does not exist, write two example Actions (one `selection` type and one `prompt` type) covering both main paths. Once deleted, they are not regenerated.
- The theme is `light`, `dark`, or `system`, and it applies to all three surfaces at once. **The default is `light`**, including on a machine whose OS appearance is dark: the system preference is read only by `theme = "system"`, which has to be chosen.
- The language is `en` or `zh`, and it applies to all three surfaces *and* the tray menu at once. **The default is `en`**, on a Chinese machine too, and there is no `system` arm: an OS locale is a guess about a reader rather than a setting, and a wrong guess replaces every word in the product — including the words explaining how to change it back ([ADR-0015](./docs/adr/0015-english-and-chinese-from-one-config-field.md)). Your Actions are never translated in either direction: they are your words, in your files.

## Config file layout

```
%APPDATA%\Beckon\                        # Windows
~/Library/Application Support/Beckon/    # macOS
├── config.toml        # global hotkey, autostart, theme, language, base_url, global model defaults
└── actions/
    ├── translate.toml
    └── ask.toml
```

The API key is **not here** — it lives in the OS credential store (the Windows Credential Manager, or the login Keychain on macOS) under the service name `Beckon` ([ADR-0005](./docs/adr/0005-api-key-in-windows-credential-manager.md)). There is no plaintext secret file anywhere on disk.

A config directory copies between the two platforms unchanged: `Ctrl`, `Alt`, `Shift` and `Cmd`/`Super` all parse on both. Only the *defaults* differ, and only where a stock machine would refuse to register them.

An Action's **identity is its filename**; the `name` field is only for display.

### config.toml

```toml
launcher_hotkey = "Ctrl+Shift+Space" # "Cmd+Shift+Space" is the macOS default
autostart = true
theme = "light"                 # light | dark | system
language = "en"                 # en | zh

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

Tauri v2 (Rust + web UI). For the reasoning and the rejected alternatives, see [ADR-0001](./docs/adr/0001-tauri-v2-on-windows-only.md); for the macOS port and what it changed, [ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md).

Both platforms are built on every push by [.github/workflows/ci.yml](./.github/workflows/ci.yml). Half of `src-tauri/src/platform/` cannot be compiled on a Windows machine and half cannot be compiled on a Mac, so a green build on one is not evidence about the other. What a compiler cannot check is in [docs/macos-testing.md](./docs/macos-testing.md), along with the one Windows behaviour the port touched.

DeepSeek is accessed via the OpenAI-compatible format at `https://api.deepseek.com`. Current models are `deepseek-v4-flash` / `deepseek-v4-pro`, 1M context, with **thinking mode on by default** — which is why `thinking = false` exists in the global defaults: leaving thinking on for translation-type Actions adds several seconds of latency and a pile of reasoning tokens for nothing. The legacy names `deepseek-chat` / `deepseek-reasoner` were discontinued on 2026-07-24; Beckon still recognises them so an old config keeps working, but does not offer them.
