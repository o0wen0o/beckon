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
npm run icons                   # re-rasterize src-tauri/icons from assets/*.svg

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

Every reveal is announced **before** the window is shown: `launcher:opened`, `popover:view`, `settings:opened`. The windows are reused and re-read their state asynchronously, so revealing first paints the *previous* trigger's contents for a few frames. `hide_popover` emits too, for the same reason.

### The design system ([src/app.css](src/app.css))

One file, four layers: brand primitives → semantic palette (light base, `[data-theme="dark"]` override) → theme-invariant scales (space, type, radius, motion, z) → element base. Components name **no** colour, size or duration of their own; a new hardcoded `12px` or hex is a bug.

- The palette is derived from [assets/logo.svg](assets/logo.svg). Brand cyan is 1.4:1 on white, so light mode uses the same hue darkened until it passes AA, and full-strength brand is reserved for dark surfaces and non-text use. Every ratio in the comments is measured — re-measure when changing one.
- `--border` is decorative and exempt from contrast rules; `--border-strong` bounds interactive controls and must clear 3:1 (WCAG 1.4.11).
- The gradient is a light source, not a paint bucket: rails, the streaming caret, the waiting rail, one primary button. Never behind prose, never on the focus ring, never on a semantic colour.
- `.surface` owns the frameless card and sets `--surface-radius`; children that meet its edge read that variable instead of repeating a number. It carries no `box-shadow` — the card fills the window rect, so the shadow you see is DWM's.
- Motion is `transform`/`opacity` only, and one `prefers-reduced-motion` block disables the lot. The Popover's looping indicators opt back in with a *static* form: a frozen travelling rail reads as a stalled request.

Icons are hand-rolled inline SVG in [src/lib/icons/](src/lib/icons/) — 24×24 grid, `currentColor`, `aria-hidden` always, so the accessible name lives on the control. Shared controls are in [src/lib/ui/](src/lib/ui/).

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

`sse.rs` is a pure frame parser, `deepseek.rs` is the only place provider quirks live, `models.rs` is the model catalog, `client.rs` does the request. **No HTTP timeout, on purpose** (README): a dead network must error immediately rather than spin, and a long thinking pause must not look like a hang. `thinking` is mapped explicitly and an unknown model is a hard error — omitting the field would silently leave DeepSeek thinking on. `LlmError::kind()` is the stable discriminant the frontend switches on.

`models::CATALOG` is read by **both** `deepseek::thinking_wire` and the Settings model dropdown (`get_models`), so the set of models offered and the set Beckon knows how to send cannot drift. Adding a model means adding a row there and nothing else. `get_models` prefers the endpoint's own `/v1/models` list and **never fails**: no credential, a rejected key, an offline machine or an empty list all fall back to the documented catalog and report the cause by kind, because an empty dropdown would be worse than the failure it reports. Whatever the config already names is always among the options — an unrecognised model is surfaced, never rewritten. Retired ids (`deepseek-chat`, `deepseek-reasoner`) stay in the catalog so an old config keeps working, but are not offered.

### Filesystem is the source of truth ([src-tauri/src/action/](src-tauri/src/action/), [reload.rs](src-tauri/src/reload.rs))

`%APPDATA%\Beckon\config.toml` + `actions\*.toml` (ADR-0003). Every mutation path — watcher event, Settings edit, startup — funnels through `reload::reload_config` / `reload_actions`, which re-read disk, re-derive hotkeys, and broadcast `config-changed` / `actions-changed`. Windows re-render from the snapshot; they never patch their own copy.

Rules that break subtly if ignored:

- An Action's **identity is its filename stem**; `name` is display only. Renaming `name` must not move the file.
- Mark `state.self_writes.mark(&path)` *before* any write, or the watcher echoes your own write back as an external change.
- Writes go through `atomic::write_atomic`; the watcher ignores dotfiles/temp files and reloads the whole directory rather than interpreting event kinds.
- A file that fails to parse is skipped and reported (`Registry::errors`), never fatal; the raw text stays editable from the Launcher's list via `read_action_raw` / `write_action_raw`.
- Hotkey registration is **derived state**: `hotkey::apply` unregisters everything and rebuilds from config + registry. Conflicts resolve by filename order, losers land in `hotkey_errors` and are flagged red. Failures are never silent — tray error icon + one-time balloon.
- The API key is only in the Windows Credential Manager (service `Beckon`, ADR-0005). "No credential", "read error" and "key rejected" must stay three distinguishable outcomes all the way to the UI.
- A missing config file or missing field is a default, never an error; a *corrupt* config is reported, never overwritten.

### Editing surfaces are editors, not owners ([src/settings/](src/settings/), [src/launcher/](src/launcher/))

There is no Save button and there must never be one (ADR-0003). The work is split by *what* is edited, not by window chrome: Settings owns the global config (credential, Launcher hotkey, theme, model defaults); the **Launcher** owns the Actions, because that is where their list already is. Both drive [src/lib/saveSlot.svelte.ts](src/lib/saveSlot.svelte.ts) from their own store — [settings/store.svelte.ts](src/settings/store.svelte.ts), [launcher/actions.svelte.ts](src/launcher/actions.svelte.ts) — and components receive values and callbacks, never calling `saveConfig` / `saveAction` themselves.

- **Saving echoes back at the window that saved.** `save_config` → `reload_config` → `config-changed` (and `save_action` → `actions-changed`), broadcast to every window including the one that caused it. So the events being defended against are mostly our own writes arriving mid-keystroke, not the file watcher.
- A snapshot is refused while a text field in the pane has focus **or** a write is pending, and is then *held* and applied when both clear — dropping it would leave the form permanently stale after an external edit.
- Focus is read from the DOM when the event arrives, not tracked in per-field flags. The flags this replaced covered two of eleven inputs, so every field added later silently opted out.
- Settings' navigation column sits **outside** the pane element, so changing section fires `focusout` and flushes the slot. No route change can strand an unwritten edit.
- The model `<select>` uses `value=` + `onchange`, **never `bind:`**, and refuses to write `""` where no inherit option exists. Both halves stop a configured model being silently rewritten before the catalog lands.
- `save_action` re-probes the Direct Hotkey and refuses the *whole* write if it cannot be registered — so while an outside app holds an Action's hotkey, even renaming it fails. The editor says so and offers to clear the hotkey.
- Both windows are reused (ADR-0007): `settings:opened` clears the last visit's typed API key and test result, `launcher:opened` closes whatever the last summon left in the editor. `onMount` cannot do it, and Settings' first open misses the event because the window is still being built — so that component always loads itself too.
- An Action's `[model]` overrides render as `OverrideField`: opening the row *is* the override, and it collapses again when focus leaves. The collapse is presentation only — the write already happened.
- A field's explanation lives behind the info icon (`InfoHint`), never as a permanent line: only warnings and errors earn one, because those are conditions to act on.

The Launcher hosting a form has one Rust consequence: `WindowEvent::Focused(false)` normally hides it, and `state.launcher_modal` (set by `set_launcher_modal`, cleared by `hide_launcher`) is what suspends that while the editor is open.

### Platform isolation ([src-tauri/src/platform/](src-tauri/src/platform/))

All Win32 lives under `platform/windows/`, re-exported through `platform/mod.rs` with non-Windows stubs so the crate still compiles elsewhere (ADR-0001). Do not scatter `#[cfg]` into business logic. `selection.rs` documents the grab's step order — release physically-held modifiers, back up the clipboard, poll `GetClipboardSequenceNumber`, restore, drop the backup — and each step there fixes a specific failure.

### Icons are generated, not edited ([assets/](assets/))

Every PNG and the ICO in `src-tauri/icons` is output from `npm run icons` ([scripts/gen-icons.ps1](scripts/gen-icons.ps1)); editing a raster by hand is how the old 32px and 256px icons drifted out of alignment with each other. The script rasterizes with `tauri icon`, already a devDependency, so there is no extra toolchain — but the app icon needs two passes, because only the default pass emits `icon.ico` and only a `--png` pass can ask for 256.

There are three sources, not one. `assets/logo.svg` is the app icon; it uses gradients, a glow filter and a drop shadow, which at 32px collapse into a grey smear. So the tray renders from `assets/tray-normal.svg` / `assets/tray-error.svg` — the same silhouette redrawn flat, with fattened strokes and no shadow margin. The two tray sources must stay geometrically identical to each other and differ only in accent colour, or the error state stops reading as the same app.

### Adding an IPC command

`#[tauri::command]` in [src-tauri/src/commands.rs](src-tauri/src/commands.rs) (validate + delegate, keep it thin) → register in `generate_handler!` in [src-tauri/src/main.rs](src-tauri/src/main.rs) → typed wrapper in [src/lib/ipc.ts](src/lib/ipc.ts) → payload type in [src/lib/types.ts](src/lib/types.ts). Any `file_name` arriving over IPC goes through `sanitize_file_name`. Errors are `String` for plain messages, `Failure { kind, message }` when the UI must react by cause.

## GitNexus

This repo is indexed by GitNexus; MCP tools (`impact`, `context`, `query`, `detect_changes`) are available. Their usage rules live in the untracked local `AGENTS.md` / `.claude/`.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (1060 symbols, 2271 relationships, 86 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> Index stale? Run `node .gitnexus/run.cjs analyze` from the project root — it auto-selects an available runner. No `.gitnexus/run.cjs` yet? `npx gitnexus analyze` (npm 11 crash → `npm i -g gitnexus`; #1939).

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows. For regression review, compare against the default branch: `detect_changes({scope: "compare", base_ref: "main"})`.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query({search_query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context({name: "symbolName"})`.
- For security review, `explain({target: "fileOrSymbol"})` lists taint findings (source→sink flows; needs `analyze --pdg`).

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/beckon/context` | Codebase overview, check index freshness |
| `gitnexus://repo/beckon/clusters` | All functional areas |
| `gitnexus://repo/beckon/processes` | All execution flows |
| `gitnexus://repo/beckon/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
