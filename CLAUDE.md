# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Beckon is a Windows-only, tray-resident LLM shortcut built on Tauri v2 (Rust backend + three Svelte 5 webview surfaces). Press a global hotkey → the Selection is grabbed by simulating Ctrl+C → a preset Action's prompt is sent to a DeepSeek/OpenAI-compatible API → the answer streams into a Popover next to the cursor.

Read these before non-trivial changes:

- [README.md](README.md) — MVP scope, out-of-scope list with reasons, decided behavior, config/Action TOML schema.
- [CONTEXT.md](CONTEXT.md) — the ubiquitous language (Action, Input Source, Selection, Launcher, Direct Hotkey, Popover, Exchange) **and the words to avoid**. Naming in code and UI follows it.
- [docs/adr/](docs/adr/) — ADR-0001…0007. The code cites them by number in module docs; a change that contradicts one needs a new ADR, not a quiet edit.
- [docs/PLAN.md](docs/PLAN.md) — build phases plus an "Implementation notes / Still needs a human" section (manual selection-grab checklist, autostart-from-installer, end-to-end key test).

## Commands

```powershell
npm install                     # once
npm run tauri dev               # run the app (spawns vite on :1420, then cargo)
npm run tauri build             # MSI + NSIS bundle
npm run dev                     # vite only, no Rust (webviews will fail on invoke)
npm run check                   # svelte-check + tsc over the frontend

cargo test                      # workspace tests (all in src-tauri)
cargo test action::tests::slugs_display_names        # one test
cargo test registry::                                # one module
cargo clippy --all-targets
cargo fmt                       # rustfmt.toml: max_width = 100
```

Tests are `#[cfg(test)]` modules beside the code. Everything platform- or network-touching is deliberately untestable and covered by the manual checklist in docs/PLAN.md instead; `platform::place_near_cursor` and `llm::sse` exist as pure functions precisely so they *can* be tested.

## Architecture

### Rust owns state; the windows are views

[src-tauri/src/state.rs](src-tauri/src/state.rs) holds every authoritative value (`Config`, `Registry`, `ExchangeManager`, `PopoverView`, hotkey bindings, pending Selection, previous foreground HWND). Frontend types in [src/lib/types.ts](src/lib/types.ts) mirror the serde shapes and are never a second source of truth. Notably `PopoverPhase` — the resolution of `input_source` against the grab — is decided in Rust so the rule does not live in two places.

`AppState` is built in `load_state()` **before** `tauri::Builder`, because Tauri creates the configured windows during `build()` and a webview can invoke a command before `setup` runs.

### The three surfaces

`launcher.html`, `popover.html`, `settings.html` at the repo root are the Vite entry points ([vite.config.ts](vite.config.ts)) and the Tauri window URLs. Launcher and Popover are declared in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json), created hidden at startup, and only shown/hidden (ADR-0007) — WebView creation is too slow to pay per keypress. Settings is built on first use in `trigger::show_settings`. Window `close` is intercepted and turned into hide; the app quits only from the tray.

Window permissions live in [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) — a new `window.*` API call from the frontend needs its permission added there.

### Trigger flow ([src-tauri/src/trigger.rs](src-tauri/src/trigger.rs))

`hotkey → grab → resolve input_source → show window`. Order is load-bearing:

- The grab happens **before any Beckon window is shown** (ADR-0006). Once the Launcher has focus, Beckon is the foreground window and a Ctrl+C would copy from the wrong process.
- The hotkey handler spawns a thread: the grab polls the clipboard for up to ~300 ms and must not block the event pump.
- `remember_foreground` skips Beckon's own HWNDs, and focus is handed back only once neither Launcher nor Popover is visible.
- The Launcher hotkey grabs eagerly into `pending_selection`; `pick_from_launcher` consumes it. Hiding the Launcher drops it.
- An empty grab is **not an error**: `selection` → `EmptySelection` hint and no request, `auto` → `NeedsInput`, `prompt` ignores the grab entirely.

### Exchange = one Popover's conversation ([src-tauri/src/exchange.rs](src-tauri/src/exchange.rs))

In-memory only, never persisted (ADR-0004); `discard_all` on hide or on a replacing trigger. Follow-ups resend the full untruncated history. Each turn installs a fresh `CancellationToken` (a cancelled one stays cancelled). Partial text from an interrupted turn *is* committed to history, since it is what the user can see.

The Popover's state machine is driven by events, not return values: `exchange:first-token` (fires once; thinking text counts, because the UI must distinguish "waiting" from "streaming"), `exchange:delta` coalesced onto a 16 ms tick, then exactly one of `exchange:done` / `exchange:error` / `exchange:interrupted` — or silence on cancel, which the UI already knows about.

### LLM layer ([src-tauri/src/llm/](src-tauri/src/llm/))

`sse.rs` is a pure frame parser, `deepseek.rs` is the only place provider quirks live, `client.rs` does the request. **No HTTP timeout, on purpose** (README): a dead network must error immediately rather than spin, and a long thinking pause must not look like a hang. `thinking` is mapped explicitly and an unknown model is a hard error — omitting the field would silently leave DeepSeek thinking on. `LlmError::kind()` is the stable discriminant the frontend switches on.

### Filesystem is the source of truth ([src-tauri/src/action/](src-tauri/src/action/), [reload.rs](src-tauri/src/reload.rs))

`%APPDATA%\Beckon\config.toml` + `actions\*.toml` (ADR-0003). Every mutation path — watcher event, Settings edit, startup — funnels through `reload::reload_config` / `reload_actions`, which re-read disk, re-derive hotkeys, and broadcast `config-changed` / `actions-changed`. Windows re-render from the snapshot; they never patch their own copy.

Rules that break subtly if ignored:

- An Action's **identity is its filename stem**; `name` is display only. Renaming `name` must not move the file.
- Mark `state.self_writes.mark(&path)` *before* any write, or the watcher echoes your own write back as an external change.
- Writes go through `atomic::write_atomic`; the watcher ignores dotfiles/temp files and reloads the whole directory rather than interpreting event kinds.
- A file that fails to parse is skipped and reported (`Registry::errors`), never fatal; the raw text stays editable in Settings via `read_action_raw` / `write_action_raw`.
- Hotkey registration is **derived state**: `hotkey::apply` unregisters everything and rebuilds from config + registry. Conflicts resolve by filename order, losers land in `hotkey_errors` and are flagged red. Failures are never silent — tray error icon + one-time balloon.
- The API key is only in the Windows Credential Manager (service `Beckon`, ADR-0005). "No credential", "read error" and "key rejected" must stay three distinguishable outcomes all the way to the UI.
- A missing config file or missing field is a default, never an error; a *corrupt* config is reported, never overwritten.

### Platform isolation ([src-tauri/src/platform/](src-tauri/src/platform/))

All Win32 lives under `platform/windows/`, re-exported through `platform/mod.rs` with non-Windows stubs so the crate still compiles elsewhere (ADR-0001). Do not scatter `#[cfg]` into business logic. `selection.rs` documents the grab's step order — release physically-held modifiers, back up the clipboard, poll `GetClipboardSequenceNumber`, restore, drop the backup — and each step there fixes a specific failure.

### Adding an IPC command

`#[tauri::command]` in [src-tauri/src/commands.rs](src-tauri/src/commands.rs) (validate + delegate, keep it thin) → register in `generate_handler!` in [src-tauri/src/main.rs](src-tauri/src/main.rs) → typed wrapper in [src/lib/ipc.ts](src/lib/ipc.ts) → payload type in [src/lib/types.ts](src/lib/types.ts). Any `file_name` arriving over IPC goes through `sanitize_file_name`. Errors are `String` for plain messages, `Failure { kind, message }` when the UI must react by cause.

## GitNexus

This repo is indexed by GitNexus; MCP tools (`impact`, `context`, `query`, `detect_changes`) are available. Their usage rules live in the untracked local `AGENTS.md` / `.claude/`.
