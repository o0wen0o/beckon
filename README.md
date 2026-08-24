# Beckon

English | [简体中文](./README.zh-CN.md)

Press a hotkey. Whatever text you had selected goes to an LLM with a prompt you wrote, and the
answer streams into a popover next to your cursor. Then it gets out of the way.

Beckon lives in the tray, starts with the machine, and stores nothing. The name comes from
"beckoning" — you wave, it comes.

**Windows and macOS.** Bring your own key: any OpenAI-compatible endpoint works, DeepSeek by
default, always the vendor's own host and never an aggregator.

<!-- screenshot: the Popover mid-stream, over a browser -->

## Install

Grab the latest [release](https://github.com/o0wen0o/beckon/releases):

| Platform | File | Note |
| --- | --- | --- |
| Windows | `Beckon_x.y.z_x64-setup.exe` | Recommended — this one self-updates |
| Windows | `Beckon_x.y.z_x64_en-US.msi` | Installs fine, but cannot update itself |
| macOS | `Beckon_x.y.z_universal.dmg` | Intel and Apple Silicon |

Nothing is code-signed, so the **first** install meets SmartScreen or Gatekeeper. Every update
after it is verified against Beckon's own signing key instead.

On macOS, grant **Accessibility** permission — without it, grabbing the selection silently returns
nothing. Settings reads the permission and links you to the pane.

## First run

Settings opens by itself, because there is no API key yet.

1. **Providers** — the default row is DeepSeek. Paste a key, hit **Test connection**. A local
   endpoint (Ollama, LM Studio) needs no key at all.
2. **Actions** — two examples are written for you on first launch. Edit them, or add your own.
3. Press `Ctrl+Shift+Space` (`Cmd+Shift+Space` on macOS) to open the Launcher.

## Using it

An **Action** is a saved prompt: a system prompt, a model, and how it gets its input. You pick one
from the Launcher, or bind it to its own hotkey and skip the Launcher entirely.

- **Launcher** — global hotkey, type to filter, Enter to run.
- **Direct Hotkey** — per-Action, straight to the answer with zero interaction.
- **Selection** — the platform copy shortcut is simulated to grab it, and your clipboard is put
  back afterwards. Nothing selected is not an error: the Popover just gives you a box to type in.
- **Popover** — takes focus, streams, accepts follow-up turns. Drag any edge or corner to resize;
  the size sticks for next time. `Esc` cancels a running request, then closes.
- **Screenshot** — the button in the Popover runs your OS snip tool and attaches the result, up to
  four per turn. It is attached, then sent — never captured-and-sent. Whether the model reads
  images is between you and your endpoint.

Beckon is one process. Launching it again while it is already resident opens Settings instead of
starting a second copy. On macOS it lives in the menu bar with no Dock tile, so there is nothing to
double-click in the first place — reopening the app hands back the copy that is already running.

Everything is bilingual — English and Simplified Chinese, all three windows plus the tray menu,
switched in Settings. Default is `en`, and there is no "follow the system" arm: a wrong locale
guess would replace every word in the product, including the words explaining how to change it
back. Your own Actions are never translated in either direction.

## Configuration

Settings covers all of it, but the files are yours to edit — a watcher reloads them on change.

```
%APPDATA%\Beckon\                        # Windows
~/Library/Application Support/Beckon/    # macOS
├── config.toml
├── models.json                          # Beckon's, not yours
└── actions/
    ├── translate.toml
    └── ask.toml
```

`config.toml` and `actions/` are the two the watcher reads and the two Settings writes. `models.json`
is neither: it is the last model list each endpoint told Beckon it serves, so that a fresh launch has
something to offer before you ask again. Deleting it costs one fetch. Nothing in it is a prompt, an
answer or a screenshot.

The whole directory copies between platforms unchanged: `Ctrl`, `Alt`, `Shift` and `Cmd`/`Super`
all parse on both, and only the defaults differ.

**API keys are not in these files.** They live in the OS credential store — Windows Credential
Manager, or the login Keychain — under service `Beckon`, one account per endpoint. There is no
plaintext secret anywhere on disk.

### config.toml

```toml
launcher_hotkey = "Ctrl+Shift+Space" # "Cmd+Shift+Space" is the macOS default
autostart = true
update_check = true             # the once-per-launch check
theme = "light"                 # light | dark | system
language = "en"                 # en | zh

[defaults]
provider = "deepseek"           # what an Action that names no provider gets

[popover]
width = 620.0                   # written by dragging the window, not by hand
height = 500.0

# One table per endpoint. Keep these last: an array of tables swallows any
# header after it. `id` is both what an Action names and the credential account.
[[api.providers]]
id = "deepseek"
label = "DeepSeek"              # display only
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
thinking = false                # DeepSeek thinks by default; off saves seconds
reasoning = "deepseek"          # deepseek | qwen | openai | none — how this
                                # endpoint is told *not* to think. A property of
                                # the endpoint, not the model; prefilled by its
                                # preset.
temperature = 1.3               # optional; omitted means let the endpoint decide
key_page = "https://platform.deepseek.com/api_keys"

[[api.providers]]
id = "ollama"
label = "Ollama (local)"
base_url = "http://localhost:11434/v1"
model = "qwen3:8b"
thinking = false
reasoning = "qwen"
# No key_page, and no key needed: a local endpoint gets no Authorization header.
```

Two endpoints can be live at once — Translate on a fast hosted model, Summarise on one that never
leaves the machine, one hotkey apart rather than a settings trip apart. Models are picked from a
list per endpoint, not typed, and no endpoint ships one: the list is the endpoint's own
`/v1/models`, remembered between launches, and otherwise what your config already names — flagged
rather than rewritten. A pre-provider config file still works;
it folds into one row on load.

### An Action

An Action's **identity is its filename**. `name` is display only.

```toml
name = "Translate"
description = "Chinese <-> English"
input_source = "auto"           # auto | prompt
hotkey = "Ctrl+Shift+T"         # optional; Launcher-only without it

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese. Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
Translate only anything after "Input:".
"""
user = "Input: {{input}}"        # may be omitted; it defaults to "{{input}}"

# [model] may be omitted entirely. Each key absent means "inherit":
#   provider  the [defaults] provider row
#   model     that row's model
#   thinking  that row's thinking
# Overriding `provider` therefore moves what the other two inherit.
[model]
provider = "ollama"
thinking = true
```

## Updates

Beckon checks once per launch, thirty seconds in, and says nothing unless there is something. The
tray menu is the loud path: `Check for Updates…` normally, `Update to 0.2.0…` once there is one.
The automatic check is a switch in Settings → Triggering; the tray item asks whenever you click it.

An update is refused while a Popover is open, and says why — installing ends the process, and an
Exchange is never on disk to come back to.

## Not doing

| | Why |
| --- | --- |
| History, search, saved conversations | An Exchange dies with the window, on purpose ([ADR-0004](./docs/adr/0004-exchanges-are-never-persisted.md)) |
| Action categories or tags | At a dozen Actions, fuzzy search is enough; subdirectories can do it later for free |
| Fill-in-the-blank Actions | Breaks "press it and wait"; the system prompt can decide instead |
| Replacing the selected text in place | [ADR-0002](./docs/adr/0002-selection-via-simulated-ctrl-c.md) |
| A floating icon after you select text | Needs global selection polling — power-hungry, easy to misfire, and it fights the copy-shortcut grab |
| Token usage display | A flash translation costs a fraction of a cent; the number changes nothing |
| Linux | The stubs in `platform/fallback.rs` keep the crate compiling, and are not a promise |
| Code signing and notarization | Not set up on either platform ([ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md)) |

## Building it yourself

Tauri v2 — Rust backend, React webviews. Needs Node and a Rust toolchain.

```bash
npm install
npm run tauri dev             # the real app, tray and all
npm run tauri build           # installers; needs an updater signing key
```

`npx tauri signer generate` makes a throwaway key that bundles fine and signs nothing anyone else
will accept. See [CLAUDE.md](./CLAUDE.md) for the four gates CI enforces.

## Reading further

- [CONTEXT.md](./CONTEXT.md) — the vocabulary, one name per concept, English and Chinese.
- [docs/adr/](./docs/adr/) — 22 decisions, each with what was rejected and why.
- [docs/macos-testing.md](./docs/macos-testing.md) — the behaviour no compiler checks.
