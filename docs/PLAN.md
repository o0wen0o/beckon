# Beckon implementation plan

Derived from [ADR-0001..0005](./adr/) and the MVP scope in [README.md](../README.md).

> **Status: implemented.** Phases 0–7 are built; the deferred ADRs are written
> ([ADR-0006](./adr/0006-grab-the-selection-eagerly-at-hotkey-press.md),
> [ADR-0007](./adr/0007-windows-are-created-hidden-at-startup-and-reused.md)). See
> [Implementation notes](#implementation-notes) at the bottom for what was resolved differently and
> what still needs a human at a keyboard.

## Stack decisions this plan assumes

These are not yet ADRs; they are the smallest choices needed to start. Anything marked
**needs ADR** should be written up before or alongside the phase that depends on it.

| Choice | Decision | Reason |
| --- | --- | --- |
| Frontend | Svelte 5 + Vite + TypeScript | Three small surfaces (Launcher / Popover / Settings); no vdom, tiny bundle, matches ADR-0001's footprint motivation. React is a fine substitute; nothing in the plan depends on it. |
| Window strategy | All three windows created hidden at startup, then shown/hidden — never created per trigger | WebView creation costs 100ms+; a hotkey tool cannot pay that. ADR-0004 requires the *Exchange* be destroyed, not the window — clearing state on hide satisfies it. **needs ADR-0007** |
| Selection timing | Grab the Selection eagerly at hotkey press, *before* showing any window | Once the Launcher has focus, the foreground window is Beckon, so a later Ctrl+C would go to the wrong process. **needs ADR-0006** |
| Clipboard access | `clipboard-win` crate | Needs `GetClipboardSequenceNumber`, which ADR-0002 requires for change polling; the Tauri clipboard plugin does not expose it. |
| Win32 access | `windows` crate, confined to `platform/windows/` | ADR-0001 consequence: platform code concentrated in a handful of modules. |
| HTTP | `reqwest` (stream feature) + manual SSE frame parsing | Small surface; one adapter module isolates OpenAI-compat quirks. |

## Layout

```
beckon/
├── Cargo.toml                 # workspace
├── src-tauri/
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs            # setup: windows, tray, plugins, state
│       ├── state.rs           # AppState
│       ├── commands.rs        # #[tauri::command] surface, thin
│       ├── config.rs          # config.toml load/save/merge
│       ├── action/
│       │   ├── mod.rs         # Action struct, TOML parse, validation
│       │   ├── registry.rs    # load dir, per-file error state, lookups
│       │   └── watcher.rs     # notify + debounce
│       ├── secrets.rs         # keyring: read/write/delete, error kinds
│       ├── llm/
│       │   ├── client.rs      # request build, streaming
│       │   ├── sse.rs         # frame parser (pure, unit-tested)
│       │   └── deepseek.rs    # model/thinking/temperature mapping
│       ├── exchange.rs        # in-memory turns, cancellation handles
│       ├── hotkey.rs          # parse, register/unregister, conflicts
│       ├── tray.rs            # icon states, menu, balloon
│       └── platform/
│           ├── mod.rs         # trait-ish facade, cfg-gated
│           └── windows/
│               ├── selection.rs   # Ctrl+C grab + clipboard restore
│               ├── focus.rs       # foreground HWND save/restore
│               └── cursor.rs      # cursor pos, monitor work area
└── src/                       # frontend
    ├── launcher/
    ├── popover/
    ├── settings/
    └── lib/ipc.ts             # typed invoke/event wrappers
```

## Phase 0 — Toolchain and skeleton

Machine has Node 24 and npm; **Rust is not installed** (`rustc`/`cargo` absent from both
shells). Blocking prerequisite.

1. Install Rust (`rustup`, `x86_64-pc-windows-msvc`), MSVC build tools, WebView2 runtime
   (present on Win11 by default), `cargo install tauri-cli`.
2. Scaffold Tauri v2 + Svelte + TS. Add plugins: `global-shortcut`, `autostart`,
   `opener` (settings links), `notification`.
3. Hidden-by-default windows in `tauri.conf.json`: `launcher`, `popover` (both
   `decorations: false`, `alwaysOnTop`, `skipTaskbar`, `transparent` as needed),
   `settings` (normal chrome).
4. `.gitignore`, `rustfmt.toml`, `clippy` clean, one CI-less `cargo test` placeholder.

**Done when** `cargo tauri dev` starts, tray icon appears, no window visible.

## Phase 1 — Config and Actions (pure Rust, no UI)

Implements ADR-0003. Highest-value phase to get right; fully unit-testable.

- `Config`: `launcher_hotkey`, `autostart`, `api.base_url`, `defaults.{model, thinking,
  temperature}`. Missing file ⇒ write defaults. Missing field ⇒ default, not error.
- `Action`: `name`, `description?`, `input_source` (`selection|prompt|auto`), `hotkey?`,
  `prompt.system`, `prompt.user?` (default `"{{input}}"`), `[model]` overrides.
  **Identity is the filename stem** (README) — `id` is derived, never stored in the file.
- Effective model params = action `[model]` over `config.defaults`. One `merge` fn, tested.
- `Registry::load(dir)` returns *both* good Actions and per-file parse errors. A broken
  file is skipped and reported, never fatal (ADR-0003).
- Writes are atomic: temp file in the same directory + rename, so a crash mid-save cannot
  truncate an Action.
- Watcher: `notify` + `notify-debouncer-full` (~300ms). Must treat delete+create as a
  modify (editors' atomic save, ADR-0003). Ignore our own writes via a short
  self-write suppression window keyed by path.
- On reload, emit one `actions-changed` event carrying the whole registry snapshot;
  frontend never keeps authoritative state (ADR-0003 consequence).

**Tests**: valid parse; multi-line system prompt round-trip; unknown field tolerated;
bad TOML surfaces as file-level error while siblings still load; merge precedence;
`{{input}}` default substitution.

**Done when** editing a TOML by hand in Notepad shows up in a `println!` reload within
~300ms, and one corrupt file does not stop the others.

## Phase 2 — Secrets and Settings window

Implements ADR-0005.

- `secrets.rs` over `keyring` v3, service `Beckon`. Three distinguishable outcomes:
  key present / **no credential** (guide reconfiguration) / **read error** (Credential
  Manager failure). ADR-0005 requires the first two never be conflated with
  "key is invalid".
- First-run condition is **"no key readable"**, never a file check (ADR-0005).
  First run ⇒ show Settings.
- Settings UI, editing files directly (ADR-0003): API key (echo masked, last 4 chars
  only), `base_url`, global hotkey recorder, global model defaults, autostart toggle,
  Action list with create / edit / delete / red badge for parse errors.
- **Test connection**: minimal request against current key + `base_url`, reporting
  auth failure separately from network failure.
- New Action file naming: slug the display `name` into a filename, de-duplicate with a
  numeric suffix. Renaming the display name later does **not** rename the file
  (identity is the filename). Surface the filename in the editor so this is visible.
- Every field commits to disk on change (debounced), and `actions-changed` /
  `config-changed` events re-render the form. No local authoritative copy.

**Done when** a fresh profile opens Settings, accepts a key, "Test connection" passes,
and hand-editing `config.toml` externally updates the open Settings window.

## Phase 3 — LLM client and streaming

- `sse.rs`: pure parser over a byte stream ⇒ `Vec<Event>`; handles split frames,
  `data: [DONE]`, comments, CRLF. Unit-tested against fixtures — no network.
- `client.rs`: OpenAI-compatible `POST {base_url}/v1/chat/completions`, `stream: true`.
  **No timeout** (README): a dead network surfaces as an immediate HTTP error.
- `deepseek.rs`: maps `model`, `temperature`, and `thinking`.
  **Open question** — the exact wire representation for disabling thinking on
  `deepseek-v4-*` is unverified in these docs. Keep it behind this one function, verify
  against live API in this phase, and fail loudly rather than silently sending nothing.
- Events to the frontend: `exchange:first-token`, `exchange:delta`, `exchange:done`,
  `exchange:error`, `exchange:interrupted`. `delta` is **coalesced on a ~16ms tick** —
  per-token IPC floods the WebView.
- `exchange.rs`: `HashMap<ExchangeId, CancellationToken>`. Esc, window hide, and a new
  trigger all cancel. Full turn history resent per follow-up, no truncation (ADR-0004).
- Distinguish, in the state machine the UI consumes: `waiting-first-token` vs
  `streaming` vs `interrupted` vs `error` (README requires the first distinction).

**Tests**: SSE fixtures incl. mid-frame splits; cancellation drops the task; error
mapping (401 vs connection refused vs mid-stream disconnect ⇒ `interrupted`, keeping
partial text).

## Phase 4 — Platform layer (Windows)

Implements ADR-0002. The riskiest code; keep it in `platform/windows/` per ADR-0001.

`grab_selection()` sequence, in order:

1. `GetForegroundWindow()` and store it (needed for focus return, README).
2. **Release physically-held modifiers** via `SendInput` key-ups (Ctrl/Alt/Shift/Win).
   The trigger hotkey is still down — without this the target app receives
   `Ctrl+Alt+C`, not `Ctrl+C`. Easy to miss, breaks everything downstream.
3. Back up the clipboard, **plain text only** (ADR-0002 accepts losing rich text/images).
4. Read `GetClipboardSequenceNumber()`, `SendInput` Ctrl down / C / C up / Ctrl up.
5. **Poll the sequence number** (~5ms interval, ~300ms cap) — never a fixed sleep
   (ADR-0002).
6. Read the text, restore the backup, **drop the backup from memory immediately**
   (ADR-0002: no extra retained copy of the user's clipboard).
7. Timeout or empty result ⇒ `Ok(None)`, **not an error** (ADR-0002, README): UAC-elevated
   windows fail silently and must be handled by `input_source`.

Also here: `focus.rs` (`SetForegroundWindow` restore on close) and `cursor.rs`
(`GetCursorPos` + monitor work area, so the Popover is clamped on-screen at any DPI).

**Manual test checklist** (this layer cannot be unit-tested): Notepad, Chrome, VS Code
(Electron), Word, Windows Terminal, an elevated console, and empty-selection in each.
Verify the clipboard contents are byte-identical afterwards for plain text.

## Phase 5 — Triggering: hotkeys, tray, Launcher

- `hotkey.rs`: parse the README's `"Ctrl+Alt+Space"` form to plugin accelerators.
  Register the global hotkey plus one per Action that declares `hotkey`.
- **Conflicts**: two Actions claiming the same accelerator ⇒ first by filename wins,
  loser flagged red in Settings (consistent with ADR-0003's "skip and flag" treatment).
- In Settings, a recorded hotkey is **registered immediately**; failure ⇒ red on the
  spot, save refused (README).
- Startup registration failure is never silent: tray switches to error icon **plus a
  one-time balloon** that opens Settings on click (README).
- Tray: normal / error icon, menu (Settings, quit), `autostart` via plugin.
- Trigger flow, both paths, sharing one grab:
  - Direct Hotkey ⇒ grab ⇒ resolve `input_source` ⇒ show Popover.
  - Global hotkey ⇒ grab, cache as pending selection ⇒ show Launcher ⇒ on pick, hand the
    cached selection to the Action (the reason for eager grab, ADR-0006).
- `input_source` resolution: `selection` with empty grab ⇒ hint, no request;
  `prompt` ⇒ ignore grab, show input box; `auto` ⇒ selection if non-empty, else input box.
- Launcher: fuzzy search over `name` + `description`, keyboard-only navigation,
  Esc hides, re-pressing the global hotkey toggles.
- **Undefined in the docs, decided here**: only one Popover exists; a new trigger while
  one is open cancels the in-flight request and replaces its contents.

**Done when** a Direct Hotkey over a Chrome selection reaches a real streamed result,
and Esc returns focus to Chrome.

## Phase 6 — Popover UI

- Positioned at the cursor, clamped to the work area; takes focus; Esc closes.
- States rendered distinctly: waiting-first-token (not a generic spinner), streaming,
  done, interrupted (keep partial text, mark "interrupted" beneath), error (inline
  message + retry button, **no system notification** — README).
- Follow-up input after a turn completes; each turn appended to the in-memory Exchange.
- **Copy** is the only export path (ADR-0002, ADR-0004) — so it must be prominent,
  keyboard-reachable, and confirm visually. Copy is a user-requested clipboard write and
  is therefore *not* restored.
- No "replace original text" affordance anywhere (ADR-0002).
- On hide: cancel in-flight request, drop the Exchange, restore foreground window.

## Phase 7 — First-run, packaging, polish

- Seed two example Actions (`translate.toml` = `selection`, `ask.toml` = `prompt`) only
  when `actions/` does not exist; **never regenerate after deletion** (README).
- Markdown rendering decision for streamed output (plain text is acceptable for MVP;
  if rendered, sanitize).
- MSI/NSIS bundle, autostart verified after install, resident-memory check against
  ADR-0001's ~30MB expectation.
- Write the deferred ADRs: **0006** eager selection grab, **0007** persistent hidden
  windows. Both contradict a naive reading of the existing docs, so they need recording.

## Open questions to resolve while building

1. `thinking = false` wire format for `deepseek-v4-*` — unverified (Phase 3).
2. Model names `deepseek-v4-flash` / `-pro` are config strings; nothing hardcodes them.
3. Direct Hotkey pressed while a Popover is open — decided above (replace), not in docs.
4. Whether the Launcher should also appear at the cursor or centered — README says only
   the Popover is cursor-adjacent; assuming centered on the active monitor.
5. Eager grab means the clipboard round-trips even when the user picks a `prompt`-only
   Action. Restoration makes this invisible, but it is a real cost — the alternative
   (restore focus, then grab) reintroduces a focus race, so it is rejected.

## Risk ranking

1. **Phase 4 selection grab** — held-modifier release, seqnum race, per-app variance.
   Prototype it standalone before wiring it into the trigger flow.
2. **Phase 1 watcher** — atomic-save patterns and self-write echo cause reload loops.
3. **Phase 3 thinking-mode mapping** — silently sending a wrong field means every
   translation quietly pays the reasoning latency ADR/README explicitly wanted gone.

## Implementation notes

What the build settled, and where it diverged from the plan above.

### Open questions, resolved

1. **`thinking` wire format — resolved from the DeepSeek API reference**, not left to a live probe:
   `deepseek-v4-*` take `"thinking": {"type": "enabled" | "disabled"}` on
   `POST /chat/completions`. It lives in one function, `llm/deepseek.rs::thinking_wire`, which
   **refuses** rather than omits: a model whose mapping we do not know returns an error instead of
   quietly leaving thinking on. Locked down by unit tests.
2. Model names are config strings throughout; nothing hardcodes `-flash` / `-pro`.
3. A trigger while a Popover is open cancels the in-flight request and replaces the contents
   (recorded in ADR-0007).
4. The Launcher is centred on the monitor under the cursor, a third of the way down. Only the
   Popover is cursor-adjacent.
5. The eager grab's cost is accepted and recorded in ADR-0006.

### Divergences from the layout

* `trigger.rs`, `reload.rs`, `seeds.rs` and `atomic.rs` exist beyond the planned tree. They keep
  `main.rs` to setup and `commands.rs` to validate-and-delegate, as the plan asked: the trigger flow,
  the reload-and-broadcast path, first-run seeding and atomic writes each needed a home that was not
  a command handler.
* `GetClipboardSequenceNumber` is called through the `windows` crate rather than `clipboard-win`;
  `clipboard-win` handles the text get/set. Both are inside `platform/windows/`, so ADR-0001's
  isolation holds either way.
* `notify` is used through `notify-debouncer-full`'s re-export instead of as a direct dependency, so
  the two cannot drift onto incompatible versions.
* Streamed output renders as plain text with preserved whitespace (the MVP option the plan allowed).
  Nothing is parsed as Markdown, so nothing needs sanitising.
* The tray error balloon cannot itself be clicked through to Settings — Windows toasts give no
  callback to a desktop app. The balloon says so, and left-clicking the tray icon opens Settings.
* Only the Launcher and Popover are created at startup. Settings is built the first time it is
  opened, because it is the one surface with no latency requirement and a WebView is the most
  expensive thing a resident tool holds (see the measurement in ADR-0007).

### Still needs a human

* **The Phase 4 manual checklist.** `grab_selection` cannot be unit-tested: run it against Notepad,
  Chrome, VS Code, Word, Windows Terminal, an elevated console, and empty selections in each, and
  confirm the clipboard is byte-identical for plain text afterwards.
* **Resident memory in the steady state** (Launcher + Popover only, which needs a stored API key so
  that first run does not open Settings). The three-surface figure is already measured and recorded
  in ADR-0007: ~285MB private bytes for the tree, against ADR-0001's ~30MB — that ADR was describing
  the Tauri process, and the process is indeed 30MB.
* **Autostart after install** from the MSI/NSIS bundle, rather than from a dev build.
* A real API key in the Credential Manager, then "Test connection" and one streamed Direct Hotkey
  result end to end.
