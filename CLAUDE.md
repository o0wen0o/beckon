# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Beckon is

A background-resident LLM shortcut for Windows and macOS on Tauri v2 (Rust backend + React
webviews). A global hotkey grabs the current Selection, resolves an Action (a preset prompt stored
as a TOML file), and streams a DeepSeek response into a popover near the cursor.

Read before non-trivial work:

- [README.md](./README.md) — the spec: scope, decided behaviour, config-file layout.
- [CONTEXT.md](./CONTEXT.md) — the vocabulary. Action, Input Source, Selection, Capture, Launcher,
  Direct Hotkey, Popover, Exchange each have one name, a list of banned synonyms, and one Chinese
  form. Use those words in code, comments, UI strings and commit messages.
- [docs/adr/](./docs/adr/) — 18 accepted ADRs. Comments cite them by number; a comment saying
  "(ADR-0007)" means the ADR explains why the code looks wrong-but-isn't.

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
npm run icons                 # regenerate icons/ from assets (PowerShell, Windows)
```

**Not done until** all four gates pass: `tsc --noEmit`, `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test`. [CI](./.github/workflows/ci.yml) runs them on
`windows-latest` and `macos-latest` both, because half of `src-tauri/src/platform/` cannot compile
on the other platform — a green local run is no evidence about the one you are not on. Behaviour no
compiler checks is listed in [docs/macos-testing.md](./docs/macos-testing.md).

## Architecture

### Rust owns state; the webviews render it (ADR-0003)

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

### The trigger flow

[src-tauri/src/trigger/mod.rs](src-tauri/src/trigger/mod.rs) is the flow and nothing else; `window`
sizes and places, `foreground` remembers and hands back the previously focused window. The order is
load-bearing:

1. Remember the foreground window, **then** grab the Selection. The grab sends the platform copy
   shortcut to whatever is in front, so it must land before any Beckon window takes focus
   (ADR-0006, ADR-0002).
2. Resolve `input_source` against the grab in Rust, producing a `PopoverView` + `PopoverPhase`. An
   empty grab is a phase, never an error.
3. Emit the view event **before** revealing the window. Windows are created hidden at startup and
   reused (ADR-0007), so revealing first paints the previous Exchange for a few frames.

A Capture (ADR-0016, ADR-0017) rides the same flow sideways: `start_capture` hides the Popover *window* —
never `hide_popover`, which would discard the Exchange — runs the OS snip tool on a thread, and
emits `popover:capture` rather than `popover:view`, because re-reading the view is the new-trigger
path and would remount the composer over the note the screenshot was taken for.

The size that step 3 reveals at is the user's (ADR-0018): `config.popover`, with the
`empty-selection` hint height as a *ceiling* on it rather than a fixed height. The Popover is
undecorated, so the eight grips that resize it are markup (`ResizeGrips`) handing the drag to
`startResizeDragging`, and the window reports the result back debounced. Rust drops any report
matching `popover_asked_size` — the size it last asked for — because every `set_size` reports itself
and the 220px hint would otherwise become the remembered size.

Spawn a thread for the grab: it blocks up to ~300ms polling the clipboard and must stay off the
event-loop thread. `show_settings` spawns too — `WebviewWindowBuilder::build` deadlocks on the main
thread on Windows.

### Rust modules

| Path | Owns |
| --- | --- |
| `commands/` | The IPC surface, one file per thing commanded, re-exported flat. Thin: validate, delegate, let `reload` broadcast. |
| `action/` | The `Action` model, the `Registry` loaded from `actions/`, the debounced watcher. |
| `exchange/` | In-memory Exchanges (ADR-0004: no storage layer anywhere), `spawn_turn`, streaming events to the Popover. |
| `llm/` | OpenAI-compatible client: `sse` a pure frame parser, `wire` the shapes, `deepseek` the only home for provider quirks, `models` the catalog. Knows nothing about windows or Actions. |
| `platform/` | The facade: Win32 under `windows/`, AppKit/CoreGraphics under `macos/`, stubs in `fallback.rs`. Put every new platform divergence here so business logic stays `#[cfg]`-free. Pure geometry (`place_near_cursor`) and Capture normalisation (`capture.rs`) live here as the unit-testable parts; `snip` is the per-platform half of ADR-0016. |
| `hotkey.rs` | Parsing (`Ctrl`/`Alt`/`Shift`/`Cmd` parse on both platforms) and registration; failures surface in `ApplyReport`, never silently. |
| `secrets.rs` | The API key via `keyring`, never plaintext on disk (ADR-0005). "First run" means "no key readable", never a file check. |
| `i18n.rs` | Only what Rust writes: the tray menu, the balloon, derived diagnostics. |

Read under a lock, drop the guard, then `await` — plain `std` locks, never held across a suspension
point. `TurnPlan` exists for exactly this.

### Frontend

Three surfaces, three Vite entry points (`launcher.html`, `popover.html`, `settings.html`), each
mounted via `mountSurface` in [src/lib/boot.tsx](src/lib/boot.tsx), which awaits theme and language
before the first paint.

- **State**: plain classes extending `Notifier` ([src/lib/store.ts](src/lib/store.ts)), read through
  `useStore` / `useSyncExternalStore`. Module-level singletons are correct because a window is never
  destroyed (ADR-0007) — which is also why transient Settings state resets on the `settings:opened`
  event rather than on mount.
- **IPC**: cross the boundary only in [src/lib/ipc.ts](src/lib/ipc.ts). Nothing else calls
  `invoke`/`listen`.
- **Errors**: failures arrive as `{kind, message}`; `describeFailure` names the kind in the reader's
  language and quotes the cause verbatim. A rejected key, a missing credential and an unreachable
  API are distinct kinds and stay distinct.
- **Overlays**: Radix portals into the pane, not `document.body`
  ([src/lib/pane.tsx](src/lib/pane.tsx)) — from the body, opening a dropdown reads as "the user left
  the form" to the save protocol.

### i18n (ADR-0015)

One `language` field in `config.toml` (`en` | `zh`, default `en`, no `system` arm) drives all three
surfaces and the tray. Two catalogs typed against each other:
[src/lib/i18n/en.ts](src/lib/i18n/en.ts) fixes the shape, `zh.ts` must satisfy it. Access is
compiler-checked (`t.settings.nav.actions`), never a string-key lookup. A new user-visible string
goes into both catalogs, taking its Chinese term from the CONTEXT.md table. Users' own Actions are
never translated in either direction.

## Conventions

- **Comment the why, and cite the ADR.** Density here is high and deliberate: nearly every
  non-obvious line names the constraint it satisfies. Match it, and update any comment your change
  invalidates.
- **Contradicting an ADR requires a new ADR**, numbered next, naming what it supersedes; edit the
  superseded one to point forward rather than deleting it (0008 → 0012, 0009 → 0010/0014,
  0001/0002/0005 → 0013). ADR-0017 extends 0016 rather than superseding it — the arity of a turn's
  Captures changed, not where they come from. ADR-0018 makes the Popover's 620×500 a *default* rather
  than contradicting the layout arguments made against it (0010, 0014, 0017): those still describe
  the window a user has never dragged. ADR-0016 does not supersede 0002 — it explains why the clipboard restore
  0002 mandates deliberately does not apply to a snip the user ran themselves.
- **Styling**: shadcn/ui (new-york, base colour `neutral`) + Tailwind v4, tokens in
  [src/globals.css](src/globals.css). Components read tokens and name no colour, size or duration.
  The accent is inversion, not a hue; `--brand` has one consumer. That file's header documents each
  deviation from shadcn's generated values with contrast ratios — keep those numbers true.
- An Action's **identity is its filename**; `name` is display only.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (1863 symbols, 4601 relationships, 154 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
