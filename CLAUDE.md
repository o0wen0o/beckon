# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Beckon is a Windows-only, tray-resident LLM shortcut built on Tauri v2 (Rust backend + three webview surfaces — Settings is React on shadcn/ui, the Launcher and Popover are still Svelte 5; ADR-0008). Press a global hotkey → the Selection is grabbed by simulating Ctrl+C → a preset Action's prompt is sent to a DeepSeek/OpenAI-compatible API → the answer streams into a Popover next to the cursor.

Read these before non-trivial changes:

- [README.md](README.md) — MVP scope, out-of-scope list with reasons, decided behavior, config/Action TOML schema.
- [CONTEXT.md](CONTEXT.md) — the ubiquitous language (Action, Input Source, Selection, Launcher, Direct Hotkey, Popover, Exchange) **and the words to avoid**. Naming in code and UI follows it.
- [docs/adr/](docs/adr/) — ADR-0001…0008. The code cites them by number in module docs; a change that contradicts one needs a new ADR, not a quiet edit.
- [docs/PLAN.md](docs/PLAN.md) — build phases plus an "Implementation notes / Still needs a human" section (manual selection-grab checklist, autostart-from-installer, end-to-end key test).

## Commands

```powershell
npm install                     # once
npm run tauri dev               # run the app (spawns vite on :1420, then cargo)
npm run tauri build             # MSI + NSIS bundle
npm run dev                     # vite only, no Rust (webviews will fail on invoke)
npm run check                   # svelte-check + tsc over the frontend (both — one framework each)
npx shadcn@latest add <name>     # pull a shadcn/ui component into src/components/ui
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

Each surface is a shell plus its views, and the state behind them lives in a singleton store — one per window, which is safe precisely because the window is created once (ADR-0007). Two frameworks for the length of the shadcn/ui port (ADR-0008): the Svelte surfaces use `.svelte.ts` runes, Settings uses plain classes over `Notifier` ([lib/store.ts](src/lib/store.ts)) that components subscribe to with `useStore`. Each Vite plugin claims its own extensions, and the surfaces share only `src/lib/*.ts`, which is framework-agnostic.

- **Popover** — [Popover.svelte](src/popover/Popover.svelte) (shell, window keys, the scroller) over [PopoverHeader](src/popover/PopoverHeader.svelte), [TurnCard](src/popover/TurnCard.svelte) and [Composer](src/popover/Composer.svelte), backed by [popover/exchange.svelte.ts](src/popover/exchange.svelte.ts). Every state a turn can be in is decided in the store and rendered in `TurnCard`; a state machine half-implemented in markup is the failure mode this split exists to prevent.
- **Launcher** — [Launcher.svelte](src/launcher/Launcher.svelte) (shell, window keys) over [ActionList](src/launcher/ActionList.svelte), backed by [launcher/actions.svelte.ts](src/launcher/actions.svelte.ts). A picker only: it reads the registry and writes nothing, so it can keep dying with its focus.
- **Settings** (React) — [Settings.tsx](src/settings/Settings.tsx) (shell, subscriptions, the delete dialog, the pane element) over [SettingsNav](src/settings/SettingsNav.tsx) and [sections/](src/settings/sections/), backed by two stores: [settings/store.ts](src/settings/store.ts) for the global config and [settings/actions.ts](src/settings/actions.ts) for the Actions. The Actions section ([sections/actions/](src/settings/sections/actions/)) is one nav entry holding the whole list, and the list and the editor are two views of that one pane.

Where a Svelte shell has to reach into a child's DOM — focusing the query box, focusing or resetting the composer, scrolling the selected row into view — the child exports a function and the shell holds it with `bind:this`. The store never touches the DOM; it calls the `onStream` / `onIdle` / `onReset` hooks the shell installs on mount.

### Two design systems, one shrinking ([src/app.css](src/app.css), [src/globals.css](src/globals.css))

Settings is styled by shadcn/ui and Tailwind utilities (ADR-0008); the Launcher and the Popover are still on the hand-rolled system in `app.css`. Neither surface reads the other's. Both rules are the same rule: a component names **no** colour, size or duration of its own, and a new hardcoded `12px` or hex — in a `.svelte` `<style>` or a `.tsx` `className` — is a bug.

**Settings.** [src/globals.css](src/globals.css) is shadcn/ui's generated file: `@import "tailwindcss"`, the `dark` variant, the `:root` / `.dark` token blocks, the `@theme inline` bridge, one `@layer base` reset. Nothing else belongs in it — the whole point of the port was to stop hand-authoring rules. The base colour is `neutral`, and the surface is monochrome: **the accent is not a hue, it is inversion.** A selected nav item and a selected segment are ink-filled with paper text, that fill is the only fill on the pane, and so "current" cannot be confused with anything else. Brand colour survives in one place, the mark in the corner — which is what `--brand` is, a non-shadcn token with exactly one consumer.

Every value that deviates from the generated set is measured, with the ratio in the comment beside it — that much of app.css's practice does carry over (ADR-0008). Moving the brand out of `--primary` is what lets `--primary` and `--ring` hold the values shadcn generates, so four deviations are left: `--muted-foreground` darkened, because the generated value is 4.34:1 on `--muted` *and* because a field's explanation is now a permanent line rather than a bubble, which makes it body copy; `--input` split from the decorative `--border` and darkened to clear 3:1, because a text field's box is the control (WCAG 1.4.11) — the same split app.css makes as `--border-strong`; `--ring` set to `--primary`, because shadcn draws the ring at `ring/50` and the generated grey lands at 1.6:1 there, on the one affordance a keyboard user has; `--warning` / `--success` added, since the alternative is `text-amber-700 dark:text-amber-400` at every use site; and all three semantic hues pulled off shadcn's chroma — `--destructive` from 0.245 to 0.185 and the other two to match — because on a surface with no hue anywhere else, stock saturation is the loudest thing in the window. It measures better too, 6.47:1 against 4.76:1. `--destructive-foreground` went the other way and is deleted, unconsumed and 2.77:1 on its own fill in dark. `--border` is 0.9 rather than 0.922: the hairline is the ledger's structure, not a decorative edge, so it is drawn where it can be seen. And `--muted-quiet` is a third grey, added because two were not enough: a group head, a row's Input Source column and the standing status line are labels *about* values rather than values, and setting them at the same strength as a field's explanation is what made the pane read as one flat grey. It is 0.56 light and 0.58 dark — quieter is *darker* in dark mode — which is 4.65:1 and 4.62:1; the mock's own third grey is 0.625, 3.57:1, which fails AA as text, so this is as light as a real window can take it.

**The type scale is the pane's, not Tailwind's.** `body` is 14px — Tailwind sets no base size, so inheriting the browser's 16px left everything carrying no class of its own (an Action's name, a bare paragraph) a full step out of the scale, which is what made a deliberately quiet pane read as merely big. Below body there are three steps, because a ledger needs three: `--text-micro` 10.5px for a tracked group head, `--text-meta` 11.5px for metadata — a filename under a name, "from Model defaults", a hotkey chip — and `--text-quiet` 13px for the quiet reading register, which is a row's label and a pane's lede. Tailwind's ladder offers 12 and 14 across that whole span; rounding to it put every line half a step high, and half a step, on every line, is the whole difference between refined and approximate. `--text-title` is 23px. And there are **two** faces, not three: Segoe UI Variable Small was in use on every hint, which changed typeface halfway down each row — a difference you see without being able to name it, which reads as unfinished. There is a fourth step below body, `--text-note` at 12px, and it is a different job from the three: `--text-meta` is metadata *about* a value, while a note is a sentence *about the state* the control is in — the key is stored, the connection failed, the hotkey is taken — which is prose and wants the reading size the mock gives it. `--container-measure` is the pane's one prose measure at 62ch, a name of our own because `max-w-prose` is a static 65ch in Tailwind v4 that no theme variable can re-point; `--container-lede` is 58ch, shorter on purpose, because the line under a display title has to read as a caption to the heading and not as the section's first paragraph. Tracking is in the scale too rather than at each call site — `--tracking-eyebrow` 0.11em, because uppercase at 10.5px closes into a block at default tracking, and `--tracking-title` -0.018em, because Tailwind's `tracking-tight` overshoots to -0.025em and at 23px that reads as letters touching. Mono is set at 0.92em wherever it carries no size class of its own (`code`, `kbd`, `.font-mono`): at equal pixel size the greater x-height and heavier stroke make a filename read as louder than the name above it.

The pane is a **ledger**, and that is the layout in one decision: a fixed right-aligned label column against a value column, every row closed by a hairline, groups headed by a tracked micro-label. The controls all start at the same x, so a pane reads as a column of values rather than a stack of forms — and the air goes *between* `FieldGroup`s, never evenly between fields, because even spacing produces a list with no structure in it. Three components hold it: `Field` is the row, `FieldGroup` is the head plus its rows, `PaneHeader` is the title, the one-line description and the section's create action. A control takes one of two measures — `Field`'s `measure` prop, 340px or 420px for a control sharing its line with buttons — and the measure wraps the control **only**, never the explanation under it, which runs to `--container-measure`. A text field stretched to the pane's width reads as an empty box with a cursor in one corner, and a control shrunk to its own content (shadcn's `SelectTrigger` is `w-fit`) breaks the single line the ledger is drawing. A 24px display title over 14px body is the contrast that stops "quiet" reading as "unfinished".


- Controls come from `npx shadcn@latest add`, into [src/components/ui/](src/components/ui/) — library source, editable, but every edit is a divergence to justify. Five stand: `destructive-outline` added; `success-outline` added, the one chromatic control on the surface and the exception to "the accent is inversion" — see the Save button below; `outline` and `ghost` set to `font-normal` — `font-medium` is right for the one filled button on a pane, and at that weight an outlined button competes with the row labels beside it. `ghost` is muted too, since without a box the weight and the colour are the only things saying it is quieter than the value it sits next to. And `switch` is rebuilt — stock unchecked is `bg-input`, a filled mid-grey pill, but here a fill means "on" and nothing else, so an off switch that is filled says the opposite of what it is; off is paper with a bounded edge and a grey knob. Its geometry went with it, a 13px knob in a 19px track where stock puts a 16px knob in 18.4px and 1.2px of air reads as a lozenge — and the knob *travels*, 200ms `ease-out` on the same curve as the track's fill, where the mock froze it because a static mock cannot show motion. Last, the button base carries `duration-150 ease-out` and `active:scale-[0.98]`: shadcn's base animates `all` at Tailwind's unset duration, which lands the colour change in one frame, and a control that only changes under the pointer gives a click no acknowledgement at all. Beckon's own compositions live one level up in [src/components/](src/components/): `Field`, `FieldGroup`, `PaneHeader`, `InfoHint`, `Segmented`, `Callout`, `ConfirmDialog`, `ModelSelect`, `Temperature`, `HotkeyInput`, `OverrideField`, `StatusBar`, `BrandMark`.
- **Radix portals to `document.body`, and the pane *is* the save protocol.** `select` and `popover` are patched to default their portal container to the pane via [src/lib/pane.tsx](src/lib/pane.tsx); `alert-dialog` is not, because the delete confirmation is hosted by the shell, outside the pane. Adding another portalling component means deciding which of those two it is.
- shadcn's ToggleGroup gives the selected item and the hovered item the same `bg-accent`, and `--accent` equals `--muted`, so the hover fill would be indistinguishable from the group's own ground. `Segmented` cancels it outright: a fill means "selected" and nothing else, hover brightens the label instead. The selected chip is `bg-primary` with `text-primary-foreground` — 17:1 against its own label, and the same rule in both themes, which is the point of inversion. A control where "set" and "under the pointer" look identical is not a styling nit.
- One destructive treatment, and it is a button variant (`destructive-outline`), not a class string at each call site: red text and edge at rest, fill on hover only. Solid `destructive` belongs to the confirmation dialog alone. Danger has to be legible at rest — a keyboard never passes through hover.
- `Callout` is a rule and its text, never a card, and it is not a shadcn `Alert` — the pane is ruled horizontally top to bottom, so an outlined box in the middle of it is the only container on screen and reads as a different kind of thing, backwards, because a callout is *about* the rows underneath it. The tone lives in the rule **alone**: the prose stays the same muted grey as every other explanation, because a paragraph set entirely in red says "all of this is the alarm" when the alarm is the one sentence in `<strong>`. There is no icon for the same reason — the rule is the marker and the words carry the meaning, so nothing here depends on colour alone.
- `HotkeyInput` is a chip beside a button, not one bordered control doing both. A box holding a combination reads as a value someone typed into a field, the thing that says how to change it should be the thing you press, and the chip is then the same weight as the hotkey chips in the Actions list rather than the widest control on the pane.
- **One green button, and it is Save.** The accent on this surface is inversion, and the single exception is the API key's Save — because it is the single exception in behaviour too. Everything else on the pane is written to a TOML file as you type (ADR-0003), so nothing else needs or may have a commit step; the key goes to the Windows Credential Manager, is cleared from the field the moment it lands, and cannot be read back to check. So it carries a colour. It does not carry a fill: Remove sits on the same line, and two solid buttons there would each read as the thing to press — so `success-outline` is the mirror of `destructive-outline` at the other end of the row, colour at rest and the fill on hover only. Green text on the background is 7.12:1 light and 11.92:1 dark; on the hover fill it is the same two ratios the other way round, white light and `--background` dark. The mock has this button as a `ghost` — the colour is a deliberate departure from it, the outline is not.
- **Motion is 150–200ms `ease-out`, and it is the same 150–200ms everywhere.** A row's hover, a nav item's fill, the chevron's nudge, a segment's label, the switch's knob, a button's press, and the pane's own entrance on a section change all run on that pair, so a click reads as one movement from the nav column into the pane rather than as several unrelated repaints. Two rules keep it from becoming decoration: the entrance animates opacity and a 4px offset only — the content is already laid out, so nothing waits on it — and it is a *vertical* offset, because the pane is `overflow-y-auto`, which per spec makes `overflow-x` compute to `auto` as well, so a horizontal transform flashes a scrollbar. Every one of them carries `motion-reduce:` and stops.
- The nav's attention dot is not drawn on the current row. On the inverted fill it is 2.7:1 at best and 1.3:1 in the warning tone, and it is the one row whose problem is already on screen in the pane beside it.
- Icons are `lucide-react`, which is what shadcn's own components import. `BrandMark` is the one exception — it is the identity, not a glyph.

**Launcher and Popover.** One file, four layers: brand primitives → semantic palette (light base, `[data-theme="dark"]` override) → theme-invariant scales (space, type, radius, motion, z) → element base.

- The palette is derived from [assets/logo.svg](assets/logo.svg). Brand cyan is 1.4:1 on white, so light mode uses the same hue darkened until it passes AA, and full-strength brand is reserved for dark surfaces and non-text use. Every ratio in the comments is measured — re-measure when changing one.
- `--border` is decorative and exempt from contrast rules; `--border-strong` bounds interactive controls and must clear 3:1 (WCAG 1.4.11).
- The gradient is a light source, not a paint bucket: rails, the streaming caret, the waiting rail, one primary button. Never behind prose, never on the focus ring, never on a semantic colour.
- `.surface` owns the frameless card and sets `--surface-radius`; children that meet its edge read that variable instead of repeating a number. It carries no `box-shadow` — the card fills the window rect, so the shadow you see is DWM's.
- Motion is `transform`/`opacity` only, and one `prefers-reduced-motion` block disables the lot. The Popover's looping indicators opt back in with a *static* form: a frozen travelling rail reads as a stalled request. This is the one thing with no Tailwind equivalent, and it is why the Popover has not been ported yet.
- Icons are hand-rolled inline SVG in [src/lib/icons/](src/lib/icons/) — 24×24 grid, `currentColor`, `aria-hidden` always, so the accessible name lives on the control.

The theme is stamped twice by [lib/theme.ts](src/lib/theme.ts): `data-theme` for `app.css`, `.dark` for shadcn's variant. One resolution, two stamps. Both it and `src/lib/icons/` go when the last Svelte surface is ported.

### Trigger flow ([src-tauri/src/trigger/](src-tauri/src/trigger/))

`mod.rs` is the flow itself; `window.rs` sizes, places and builds windows; `foreground.rs` remembers whose window was in front. `hotkey → grab → resolve input_source → show window`. Order is load-bearing:

- The grab happens **before any Beckon window is shown** (ADR-0006). Once the Launcher has focus, Beckon is the foreground window and a Ctrl+C would copy from the wrong process.
- The hotkey handler spawns a thread: the grab polls the clipboard for up to ~300 ms and must not block the event pump.
- `remember_foreground` skips Beckon's own HWNDs, and focus is handed back only once neither Launcher nor Popover is visible.
- The Launcher hotkey grabs eagerly into `pending_selection`; `pick_from_launcher` consumes it. Hiding the Launcher drops it.
- An empty grab is **not an error**: `selection` → `EmptySelection` hint and no request, `auto` → `NeedsInput`, `prompt` ignores the grab entirely.

### Exchange = one Popover's conversation ([src-tauri/src/exchange/](src-tauri/src/exchange/))

`mod.rs` is the bookkeeping (`ExchangeManager`, `TurnPlan`), `events.rs` the wire to the Popover, `turn.rs` the spawned task that runs one turn and drives that wire — so the emit calls have one home and the state machine's shape is readable in one file. In-memory only, never persisted (ADR-0004); `discard_all` on hide or on a replacing trigger. Follow-ups resend the full untruncated history. Each turn installs a fresh `CancellationToken` (a cancelled one stays cancelled). Partial text from an interrupted turn *is* committed to history, since it is what the user can see.

The Popover's state machine is driven by events, not return values: `exchange:first-token` (fires once; thinking text counts, because the UI must distinguish "waiting" from "streaming"), `exchange:delta` coalesced onto a 16 ms tick, then exactly one of `exchange:done` / `exchange:error` / `exchange:interrupted` — or silence on cancel, which the UI already knows about.

### LLM layer ([src-tauri/src/llm/](src-tauri/src/llm/))

`sse.rs` is a pure frame parser, `wire.rs` holds every response shape plus the pure functions over them, `error.rs` is the one error type, `deepseek.rs` is the only place provider quirks live, `models.rs` is the model catalog, and `client.rs` is only the requests — so everything but `client.rs` is testable without a network. **No HTTP timeout, on purpose** (README): a dead network must error immediately rather than spin, and a long thinking pause must not look like a hang. `thinking` is mapped explicitly and an unknown model is a hard error — omitting the field would silently leave DeepSeek thinking on. `LlmError::kind()` is the stable discriminant the frontend switches on.

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

### The editing surface is an editor, not an owner ([src/settings/](src/settings/))

There is no Save button and there must never be one (ADR-0003). **Settings is the only place anything is authored** — the global config (credential, Launcher hotkey, theme, model defaults) *and* the Actions; the Launcher is a picker that writes nothing. Two stores, one per kind of file, both driving [src/lib/saveSlot.ts](src/lib/saveSlot.ts): [settings/store.ts](src/settings/store.ts) for `config.toml`, [settings/actions.ts](src/settings/actions.ts) for `actions\*.toml`. Components receive values and callbacks, never calling `saveConfig` / `saveAction` themselves. The Action store reads `defaults` and the model catalog off the config store rather than fetching its own — two copies in one window could only drift.

- **Saving echoes back at the window that saved.** `save_config` → `reload_config` → `config-changed` (and `save_action` → `actions-changed`), broadcast to every window including the one that caused it. So the events being defended against are mostly our own writes arriving mid-keystroke, not the file watcher.
- A snapshot is refused while a text field in the pane has focus **or** a write is pending, and is then *held* and applied when both clear — dropping it would leave the form permanently stale after an external edit.
- Focus is read from the DOM when the event arrives, not tracked in per-field flags. The flags this replaced covered two of eleven inputs, so every field added later silently opted out.
- Settings' navigation column sits **outside** the pane element, so changing section fires the pane's blur and flushes the slot. No route change can strand an unwritten edit.
- Radix would portal an open dropdown or hint bubble to `document.body`, i.e. outside the pane — which reads as "the user left the form". `select` and `popover` portal into the pane instead ([lib/pane.tsx](src/lib/pane.tsx)); the delete dialog stays on the body, where the old native `<dialog>` was. ADR-0008 has the reasoning; a new portalling component has to pick a side.
- `ModelSelect` is controlled — `value=` + `onChange`, **never** a two-way binding — and refuses to write `""` where no inherit option exists. Both halves stop a configured model being silently rewritten before the catalog lands. Radix rejects an item valued `""`, so inherit rides a sentinel mapped at both edges of that one file.
- `save_action` re-probes the Direct Hotkey and refuses the *whole* write if it cannot be registered — so while an outside app holds an Action's hotkey, even renaming it fails. The editor says so and offers to clear the hotkey.
- The window is reused (ADR-0007): `settings:opened` clears the last visit's typed API key and test result and closes whatever Action it left open in the editor. `onMount` cannot do it, and the first open misses the event because the window is still being built — so `Settings.tsx` always loads itself too.
- An Action's `[model]` overrides render as `OverrideField`: opening the row *is* the override, and it collapses again when focus leaves. The collapse is presentation only — the write already happened.
- A field's explanation is a permanent line under the control, not a bubble. Hiding it behind an info icon was right while the label sat directly above the control and the prose pushed the two apart; in the ledger the hint is beside the label rather than between them, and a settings pane nobody can read without hovering is the worse failure. `InfoHint` survives where the room genuinely is not there — `OverrideField`'s collapsed rows — and nowhere else.

Keeping the forms out of the Launcher is what lets `WindowEvent::Focused(false)` hide it unconditionally: there is nothing unwritten inside it to lose, and no dropdown or dialog of its own to survive.

### Platform isolation ([src-tauri/src/platform/](src-tauri/src/platform/))

All Win32 lives under `platform/windows/`, re-exported through `platform/mod.rs` with non-Windows stubs so the crate still compiles elsewhere (ADR-0001). Do not scatter `#[cfg]` into business logic. `selection.rs` documents the grab's step order — release physically-held modifiers, back up the clipboard, poll `GetClipboardSequenceNumber`, restore, drop the backup — and each step there fixes a specific failure.

### Icons are generated, not edited ([assets/](assets/))

Every PNG and the ICO in `src-tauri/icons` is output from `npm run icons` ([scripts/gen-icons.ps1](scripts/gen-icons.ps1)); editing a raster by hand is how the old 32px and 256px icons drifted out of alignment with each other. The script rasterizes with `tauri icon`, already a devDependency, so there is no extra toolchain — but the app icon needs two passes, because only the default pass emits `icon.ico` and only a `--png` pass can ask for 256.

There are three sources, not one. `assets/logo.svg` is the app icon; it uses gradients, a glow filter and a drop shadow, which at 32px collapse into a grey smear. So the tray renders from `assets/tray-normal.svg` / `assets/tray-error.svg` — the same silhouette redrawn flat, with fattened strokes and no shadow margin. The two tray sources must stay geometrically identical to each other and differ only in accent colour, or the error state stops reading as the same app.

### Adding an IPC command

`#[tauri::command]` in the matching file under [src-tauri/src/commands/](src-tauri/src/commands/) — `config`, `actions`, `secrets`, `models`, `windows` (validate + delegate, keep it thin; `commands/mod.rs` re-exports them flat, so the handler list never learns which file it landed in) → register in `generate_handler!` in [src-tauri/src/main.rs](src-tauri/src/main.rs) → typed wrapper in [src/lib/ipc.ts](src/lib/ipc.ts) → payload type in [src/lib/types.ts](src/lib/types.ts). Any `file_name` arriving over IPC goes through `sanitize_file_name`. Errors are `String` for plain messages, `Failure { kind, message }` when the UI must react by cause.

## GitNexus

This repo is indexed by GitNexus; MCP tools (`impact`, `context`, `query`, `detect_changes`) are available. Their usage rules live in the untracked local `AGENTS.md` / `.claude/`.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (1342 symbols, 3185 relationships, 108 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
