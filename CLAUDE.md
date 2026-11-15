# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Beckon is

A background-resident LLM shortcut for Windows and macOS on Tauri v2 (Rust backend + React
webviews). A global hotkey grabs the current Selection, resolves an Action (a preset prompt stored
as a TOML file), and streams the response into a popover near the cursor. The endpoint is any
OpenAI-compatible one — DeepSeek by default — kept as a table and chosen **per Action**,
always at the vendor's own host and never through an aggregator.

Read before non-trivial work:

- [README.md](./README.md) — the user-facing surface: what Beckon does, install, first run, the
  config-file and Action-file layout, and what it deliberately does not do.
- [CONTEXT.md](./CONTEXT.md) — the vocabulary. Action, Provider, Input Source, Selection, Capture,
  Launcher, Direct Hotkey, Popover, Exchange each have one name, a list of banned synonyms, and one
  Chinese form. Use those words in code, comments, UI strings and commit messages.

## Commands

Run `cargo` from `src-tauri/` (workspace root is `Cargo.toml`, members `["src-tauri"]`).

```bash
npm run tauri dev             # the real app: builds the frontend, launches the tray process
npm run dev                   # vite alone on :1420 — markup only, no IPC
npx tsc --noEmit              # the frontend gate; there is no JS test runner in this repo

cargo fmt --check             # max_width = 100 (rustfmt.toml)
cargo clippy --all-targets -- -D warnings
cargo test
cargo test place_near_cursor  # one test by name substring
cargo test platform::         # one module

npm run tauri build           # MSI/NSIS or .app/.dmg; unsigned on both platforms
npm run build:signed          # the same build, with the updater key set for that process only
npm run icons                 # regenerate icons/ from assets (PowerShell, Windows)
```

`tauri build` needs an updater signing key — `createUpdaterArtifacts` is on, so an
unsigned bundle is an error, not an artifact. `npm run build:signed` is that build with
`TAURI_SIGNING_PRIVATE_KEY_PATH` and `..._PASSWORD` set for the one process and nothing written to the
user environment; `pwsh scripts/build-signed.ps1` is the same on macOS. Without the maintainer's key,
`npx tauri signer generate` makes a throwaway that bundles fine and signs nothing anyone will accept.
Nothing else needs it: `tauri dev` does not bundle, and neither do the four gates.

Releasing is a tag and nothing else: bump `version` in `package.json` (the only file that carries it —
`tauri.conf.json` reads it from there), tag `vX.Y.Z`, push. `release.yml` builds both platforms and
leaves a **draft** release; publishing it by hand is what starts serving `latest.json` to installed
copies.

**Not done until** all four gates pass: `tsc --noEmit`, `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test`. [CI](./.github/workflows/ci.yml) runs them on
`windows-latest` and `macos-latest` both, because half of `src-tauri/src/platform/` cannot compile
on the other platform — a green local run is no evidence about the one you are not on. Behaviour no
compiler checks is listed in [docs/macos-testing.md](./docs/macos-testing.md).

## Architecture

### Rust owns state; the webviews render it

The filesystem is the single source of truth for config and Actions. `AppState`
([src-tauri/src/state.rs](src-tauri/src/state.rs)) holds the authoritative copy; the frontend keeps
none and never patches its own snapshot.

Route every change to disk — Settings edit, file watcher, startup — through
[src-tauri/src/reload.rs](src-tauri/src/reload.rs), which re-reads disk into state and broadcasts one
whole snapshot (`config-changed`, `actions-changed`). Three consequences:

- A save is echoed back at the window that caused it, so `src/lib/saveSlot.ts` debounces writes and
  holds off adopting a snapshot while a text field inside the pane has focus.
- Write through `atomic::write_atomic` (temp file + rename in the same directory) and register the
  path in `SelfWrites` so the watcher swallows the echo — `SUPPRESSION` must exceed `DEBOUNCE`.
- Per-Action diagnostics (unparsable TOML, a Direct Hotkey that lost a conflict) are *derived state*
  built by `Registry::load` / `hotkey::apply` in the current language, so a language change re-runs
  both.
- A `config.toml` that does not load is state too: `AppState::config_fault`, set by `load_state` and
  every `reload_config`. While it is set the config in force is `Config::default()`, so nothing
  derived from a *default* is applied as if the user had asked for it (autostart, the startup update
  check), `reload::apply_hotkeys` puts the tray in its error state for it as well as for a hotkey,
  and Settings draws it above every pane. It is not one of `startup_errors` — those are the hotkeys',
  worded for the pane that can fix them.

### The trigger flow

[src-tauri/src/trigger/mod.rs](src-tauri/src/trigger/mod.rs) is the flow and nothing else; `window`
sizes and places, `foreground` remembers and hands back the previously focused window. The order is
load-bearing:

1. Remember the foreground window, **then** grab the Selection. The grab sends the platform copy
   shortcut to whatever is in front, so it must land before any Beckon window takes focus.
2. Resolve `input_source` against the grab in Rust, producing a `PopoverView` + `PopoverPhase`. An
   empty grab is a phase, never an error — there are two arms (`auto`, `prompt`) and two phases (`NeedsInput`, `Running`), so an empty grab always lands in the composer. The
   Action's `[model]` table is merged over the Provider row it resolves to in the same step
   (`model_params`), so `ModelParams` carries a provider **id** onward — the row itself is re-read at
   request time.
3. Emit the view event **before** revealing the window. Windows are created hidden at startup and
   reused, so revealing first paints the previous Exchange for a few frames.

A Capture rides the same flow sideways: `start_capture` hides the Popover *window* —
never `hide_popover`, which would discard the Exchange — runs the OS snip tool on a thread, and
emits `popover:capture` rather than `popover:view`, because re-reading the view is the new-trigger
path and would remount the composer over the note the screenshot was taken for. The snip thread can
outlive the Popover that started it — a cancelled snip on Windows is noticed only when its 45 seconds
run out — so it remembers `popover_generation` and drops its result if a new trigger or a close has
bumped it (`AppState::supersede_popover`): no attach, no event, no reveal.

The size that step 3 reveals at is the user's: `config.popover`, with no phase overriding it
— shrunk only where the monitor under the cursor has no room for
it (`platform::fit_to_area`), for that showing and without rewriting config. Size and position are
both worked out at *that* monitor's scale factor (`platform::cursor::Screen`), never the window's
own, which is the scale of wherever it was last shown. The Popover is undecorated, so the
eight grips that resize it are markup (`ResizeGrips`) handing the drag to `startResizeDragging`, and
the window reports the result back debounced. Rust drops any report matching `popover_asked_size` —
the size it last asked for — because every `set_size` reports itself, and a report echoing what Rust
just asked for is not a drag.

Insert runs the flow backwards, and its order is load-bearing the same way. The clipboard
write is Copy's and is **never restored** — restoring the clipboard after a synthetic paste is a
race, because when the target has read it can only be guessed at. The remembered foreground window is brought back **before** `hide_popover`, while
Beckon is still the process allowed to hand the foreground on and there is still a Popover to report
a refusal in; then, on a thread, `wait_for_foreground` polls until the target really is in front and
`platform::selection::send_paste` sends the one keystroke. Past the hide nothing can be reported, so
a paste that cannot be sent is logged and the answer is still on the clipboard.

One process, always. `tauri-plugin-single-instance` is the **first** plugin registered in
`main`, because plugin setups run in order and `setup` runs after all of them — so a second launch
exits before it can claim a hotkey, add a second tray icon or start a second watcher. It exits by
telling the running copy to open Settings, which is the only surface a *launch* can be asking for.
`update::install` releases the lock by hand before `restart`, since `cleanup_before_exit` does not
reach plugins.

Spawn a thread for the grab: it blocks up to ~300ms polling the clipboard and must stay off the
event-loop thread. `show_settings` spawns too — `WebviewWindowBuilder::build` deadlocks on the main
thread on Windows.

### Rust modules

| Path | Owns |
| --- | --- |
| `commands/` | The IPC surface, one file per thing commanded, re-exported flat. Thin: validate, delegate, let `reload` broadcast. |
| `action/` | The `Action` model, the `Registry` loaded from `actions/`, the debounced watcher. |
| `exchange/` | In-memory Exchanges (no storage layer anywhere), `spawn_turn`, streaming events to the Popover. |
| `llm/` | OpenAI-compatible client: `sse` a pure frame parser, `wire` the shapes, `request` the only home for a divergence between endpoints, `models` the DeepSeek catalog. Knows nothing about windows. |
| `platform/` | The facade: Win32 under `windows/`, AppKit/CoreGraphics under `macos/`, stubs in `fallback.rs`. Put every new platform divergence here so business logic stays `#[cfg]`-free. Pure geometry (`place_near_cursor`) and Capture normalisation (`capture.rs`) live here as the unit-testable parts; `snip` runs the OS snip tool, per platform. |
| `hotkey.rs` | Parsing (`Ctrl`/`Alt`/`Shift`/`Cmd` parse on both platforms) and registration; failures surface in `ApplyReport`, never silently. |
| `secrets.rs` | One API key **per Provider** via `keyring`, account `provider:{id}`, never plaintext on disk. "First run" means "no key readable" for the default row, never a file check — and never for a local row, which wants no header at all. |
| `i18n.rs` | Only what Rust writes: the tray menu, the balloon, derived diagnostics. |
| `update.rs` | The self-update channel: check, verify, install, restart. Reaches `i18n`, `tray`, `state` and one constant from `trigger` and nothing else — an available update is neither config nor an Action, so it never touches the reload path, and no window renders it. |

Read under a lock, drop the guard, then `await` — plain `std` locks, never held across a suspension
point. `TurnPlan` exists for exactly this.

### Frontend

Three surfaces, three Vite entry points (`launcher.html`, `popover.html`, `settings.html`), each
mounted via `mountSurface` in [src/lib/boot.tsx](src/lib/boot.tsx), which awaits theme and language
before the first paint.

- **State**: plain classes extending `Notifier` ([src/lib/store.ts](src/lib/store.ts)), read through
  `useStore` / `useSyncExternalStore`. Module-level singletons are correct because a window is never
  destroyed — which is also why transient Settings state resets on the `settings:opened`
  event rather than on mount.
- **IPC**: cross the boundary only in [src/lib/ipc.ts](src/lib/ipc.ts). Nothing else calls
  `invoke`/`listen`.
- **Errors**: failures arrive as `{kind, message}`; `describeFailure` names the kind in the reader's
  language and quotes the cause verbatim. A rejected key, a missing credential and an unreachable
  API are distinct kinds and stay distinct.
- **The answer is Markdown**, drawn by [src/popover/Markdown.tsx](src/popover/Markdown.tsx)
  and nothing else is: the question card and the reasoning stay plain text. Three properties are
  load-bearing there — no raw HTML, no fetched image, and a link that goes to Rust's `open_link`
  rather than navigating the webview — so a new element is a new entry in that file's component
  map, never a `rehype` plugin that widens what an answer can make the window do.
- **Overlays**: Radix portals into the pane, not `document.body`
  ([src/lib/pane.tsx](src/lib/pane.tsx)) — from the body, opening a dropdown reads as "the user left
  the form" to the save protocol.

### The Provider table

`config.api.providers` is the list of endpoints; `config.defaults.provider` names the one an Action
that says nothing inherits. There is no "active" row — several are in use at once, one per Action —
so nothing in the codebase may reintroduce a global switch.

Three rules the whole layer leans on:

- **`Config::fold_legacy` owns the invariants**: the table is never empty, every id is distinct, and
  `defaults.provider` always names a row that exists. It runs on the load path, inside
  `Config::default`, and at the IPC boundary in `save_config` — so what a fresh install has cannot
  drift from what a pre-provider file becomes, and a table arriving from a window cannot be the one
  thing on disk that breaks them. No pane re-checks any of it. `ApiConfig::default` is therefore
  **empty** — an empty table means "the file said nothing", which is the signal to migrate.
- **The wire dialect is a property of the endpoint, never of the model.** `Reasoning` says how an
  endpoint is told *not* to think, `Search` how it is *asked* to search the web, and
  `Reasoning::guess` is the only host guess in the codebase — it runs once, on a file written before
  the table existed. Anywhere else, the row states it and `llm/request.rs` sends nothing it was not
  told to: an unknown field is a `400`, not a courtesy. The two enums are twins with opposite
  polarity: thinking happens unless stopped, so `Reasoning` names off-switches; searching happens only
  when asked, so `Search` names on-switches, `None` means "cannot be asked", and off is silence
  everywhere but xAI. Nothing detects a `Search` arm — a probe would run a real search and be billed
  for it — so a preset states it and a hand-made row is asked.
- **A configured model is surfaced, never rewritten**, and `get_models` gathers `configured` *per
  provider* so a model an Action pinned before its endpoint changed still appears in that endpoint's
  dropdown. Overriding an Action's provider strands its pinned model; the editor says so in red with
  the revert control beside it.

`src/lib/providers.ts` mirrors six small rules from Rust — `isLocal`, `chatUrl`, the Action count,
the stranded model, `relaysThrough` (`config::BROKERS`), and `keyProblem` (the four-outcome
credential split `commands::require_api_key` owns) — because they answer what a pane *says* while drawing a list, not what goes on the wire. Add a
rule there only if it can be stated twice without drifting; otherwise it belongs in Rust and reaches
the window as a field. What is *not* there is any invariant: those are `fold_legacy`'s.

### i18n

One `language` field in `config.toml` (`en` | `zh`, default `en`, no `system` arm) drives all three
surfaces and the tray. Two catalogs typed against each other:
[src/lib/i18n/en.ts](src/lib/i18n/en.ts) fixes the shape, `zh.ts` must satisfy it. Access is
compiler-checked (`t.settings.nav.actions`), never a string-key lookup. A new user-visible string
goes into both catalogs, taking its Chinese term from the CONTEXT.md table. Users' own Actions are
never translated in either direction.

## Conventions

- **Comment the why.** Density here is high and deliberate: nearly every non-obvious line names
  the constraint it satisfies. Match it, and update any comment your change invalidates.
- **Styling**: shadcn/ui (new-york, base colour `neutral`) + Tailwind v4, tokens in
  [src/globals.css](src/globals.css). Components read tokens and name no colour, size or duration.
  The accent is inversion, not a hue; `--brand` has one consumer. That file's header documents each
  deviation from shadcn's generated values with contrast ratios — keep those numbers true.
- An Action's **identity is its filename**; `name` is display only.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (2311 symbols, 5825 relationships, 194 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
