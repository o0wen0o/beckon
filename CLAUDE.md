# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Beckon is

A background-resident LLM shortcut for Windows and macOS on Tauri v2 (Rust backend + React
webviews). A global hotkey grabs the current Selection, resolves an Action (a preset prompt stored
as a TOML file), and streams the response into a popover near the cursor. The endpoint is any
OpenAI-compatible one — DeepSeek by default — kept as a table and chosen **per Action**
(ADR-0021), always at the vendor's own host and never through an aggregator.

Read before non-trivial work:

- [README.md](./README.md) — the user-facing surface: what Beckon does, install, first run, the
  config-file and Action-file layout, and what it deliberately does not do.
- [CONTEXT.md](./CONTEXT.md) — the vocabulary. Action, Provider, Input Source, Selection, Capture,
  Launcher, Direct Hotkey, Popover, Exchange each have one name, a list of banned synonyms, and one
  Chinese form. Use those words in code, comments, UI strings and commit messages.
- [docs/adr/](./docs/adr/) — 27 accepted ADRs. Comments cite them by number; a comment saying
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
npm run build:signed          # the same build, with the updater key set for that process only
npm run icons                 # regenerate icons/ from assets (PowerShell, Windows)
```

`tauri build` needs an updater signing key since ADR-0022 — `createUpdaterArtifacts` is on, so an
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
   empty grab is a phase, never an error — and since ADR-0020 there are two arms (`auto`, `prompt`)
   and two phases (`NeedsInput`, `Running`), so an empty grab always lands in the composer. The
   Action's `[model]` table is merged over the Provider row it resolves to in the same step
   (`model_params`), so `ModelParams` carries a provider **id** onward — the row itself is re-read at
   request time (ADR-0021).
3. Emit the view event **before** revealing the window. Windows are created hidden at startup and
   reused (ADR-0007), so revealing first paints the previous Exchange for a few frames.

A Capture (ADR-0016, ADR-0017) rides the same flow sideways: `start_capture` hides the Popover *window* —
never `hide_popover`, which would discard the Exchange — runs the OS snip tool on a thread, and
emits `popover:capture` rather than `popover:view`, because re-reading the view is the new-trigger
path and would remount the composer over the note the screenshot was taken for.

The size that step 3 reveals at is the user's, unconditionally (ADR-0018): `config.popover`, with no
phase overriding it since ADR-0020 removed the short hint window. The Popover is undecorated, so the
eight grips that resize it are markup (`ResizeGrips`) handing the drag to `startResizeDragging`, and
the window reports the result back debounced. Rust drops any report matching `popover_asked_size` —
the size it last asked for — because every `set_size` reports itself, and a report echoing what Rust
just asked for is not a drag.

One process, always (ADR-0023). `tauri-plugin-single-instance` is the **first** plugin registered in
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
| `exchange/` | In-memory Exchanges (ADR-0004: no storage layer anywhere), `spawn_turn`, streaming events to the Popover. |
| `llm/` | OpenAI-compatible client: `sse` a pure frame parser, `wire` the shapes, `request` the only home for a divergence between endpoints, `models` the DeepSeek catalog. Knows nothing about windows. |
| `platform/` | The facade: Win32 under `windows/`, AppKit/CoreGraphics under `macos/`, stubs in `fallback.rs`. Put every new platform divergence here so business logic stays `#[cfg]`-free. Pure geometry (`place_near_cursor`) and Capture normalisation (`capture.rs`) live here as the unit-testable parts; `snip` is the per-platform half of ADR-0016. |
| `hotkey.rs` | Parsing (`Ctrl`/`Alt`/`Shift`/`Cmd` parse on both platforms) and registration; failures surface in `ApplyReport`, never silently. |
| `secrets.rs` | One API key **per Provider** via `keyring`, account `provider:{id}`, never plaintext on disk (ADR-0005, ADR-0021). "First run" means "no key readable" for the default row, never a file check — and never for a local row, which wants no header at all. |
| `i18n.rs` | Only what Rust writes: the tray menu, the balloon, derived diagnostics. |
| `update.rs` | The self-update channel (ADR-0022): check, verify, install, restart. Reaches `i18n`, `tray`, `state` and one constant from `trigger` and nothing else — an available update is neither config nor an Action, so it never touches the reload path, and no window renders it. |

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

### The Provider table (ADR-0021)

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
  endpoint is told *not* to think, `Search` how it is *asked* to search the web (ADR-0026), and
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
the stranded model, `relaysThrough` (`config::BROKERS`, ADR-0025), and `keyProblem` (the four-outcome
credential split `commands::require_api_key` owns) — because they answer what a pane *says* while drawing a list, not what goes on the wire. Add a
rule there only if it can be stated twice without drifting; otherwise it belongs in Rust and reaches
the window as a field. What is *not* there is any invariant: those are `fold_legacy`'s.

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
  0002 mandates deliberately does not apply to a snip the user ran themselves. ADR-0019 and ADR-0020
  each *narrow* what an Action file may say without contradicting why: 0011 and 0012 still describe
  the override rows that remain, and 0020 keeps 0002's "an empty grab is a phase, never an error" —
  it just leaves one phase where there were two. ADR-0021 supersedes nothing and *extends* three:
  0005's three credential outcomes all survive, with one new reading (nothing stored for a local
  endpoint is a working setup, not a fault); 0011's override machinery gained a third row and needed
  no new mechanism for it; and 0019's decision — no temperature *control* — stands, while the 1.3 it
  pinned moved onto the DeepSeek row, which is the argument 0019 itself made about where a DeepSeek
  quirk belongs. ADR-0022 supersedes nothing and adds a channel rather than changing one: 0013's
  "nothing is signed or notarized" still holds for the *installer*, and the signature 0022 makes
  mandatory is a different one over a different artifact; 0004's "the Exchange dies with the window"
  is why an install is refused while a Popover is open; and 0003 is the reason update state lives in
  `AppState` and the tray rather than in `config.toml` and a pane. ADR-0023 supersedes nothing and
  *protects* several: 0003's filesystem-as-truth is why a second writer is a correctness problem and
  not a waste of memory, `hotkey.rs`'s "failures surface, never silently" is what a second copy would
  turn into a lie, and 0022's restart is the one place the lock has to be handed back by hand.
  ADR-0024 *narrows* two and supersedes neither outright: 0004's "no storage layer" gains one sidecar
  that records ids and the URL they came from and no part of an Exchange, and 0021 is superseded **in
  part** — its documented-catalog fallback is gone entirely, while the rest of it, including
  `llm/models.rs` as the single table the request layer reads for `thinking`, stands. 0003 is the
  reason the sidecar is *not* config: the user did not write it, so it is neither watched nor
  broadcast, and a fetched list on the reload path would echo back at the window that caused it.
  ADR-0026 supersedes nothing and *reuses* two: 0021's override chain carries `web_search` with no new
  mechanism — a fourth row that inherits from whichever provider the first one resolves to — and
  0021's own "the dialect is the row's" is what makes `Search` a field rather than a model rule. Its
  one narrowing is deliberate and recorded there: a host whose search is a tool call needing a second
  round trip stays `Search::None`, because 0004's Exchange streams one request per turn.
  ADR-0027 supersedes 0026 **in part**, and only for what a control offers: a model the vendor
  documents as excluded from its endpoint's search field — DashScope's Max tiers — greys the switch
  rather than taking a `true` that reaches nothing, so `Search::supports_model` answers per arm and
  by family and each `ModelOption` carries the answer beside its `thinking` one. Every wire claim
  0026 makes stands: `search_wire` still reads only the row, still sends the field for any model, and
  still never fails. The "no per-model list" it argued survives as "no list of ids" — an id no arm
  recognises is `None`, which is offered rather than greyed, because an arm that said no to what it
  had not heard of would grey each new model on the day it shipped.
  ADR-0025 supersedes 0021 **in part** and in one direction only: its preset rule stops being "the
  request must terminate at the company whose key it carries" and becomes "a row that relays says
  so". Everything else 0021 argues is untouched — no active row, the dialect stated rather than
  guessed, `fold_legacy` owning the invariants — and the ban's own reasoning survives inside the
  disclosure, because the risk it named did not change, only who decides to accept it. The host match
  `Provider::relays` performs is not a hole in 0021's "no host guess": that rule exists because a
  wrong dialect is a `400` on every turn, and a wrong broker match costs a warning nobody needed.
- **Styling**: shadcn/ui (new-york, base colour `neutral`) + Tailwind v4, tokens in
  [src/globals.css](src/globals.css). Components read tokens and name no colour, size or duration.
  The accent is inversion, not a hue; `--brand` has one consumer. That file's header documents each
  deviation from shadcn's generated values with contrast ratios — keep those numbers true.
- An Action's **identity is its filename**; `name` is display only.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (2293 symbols, 5794 relationships, 192 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
