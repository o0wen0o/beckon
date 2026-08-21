# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repository.

## What this is

Beckon is a tray-resident LLM shortcut for Windows and macOS, built on Tauri v2: a Rust backend plus three webview surfaces, all React on shadcn/ui and Tailwind. Press a global hotkey → the Selection is grabbed by simulating the platform's copy shortcut → a preset Action's prompt is sent to a DeepSeek/OpenAI-compatible API → the answer streams into a Popover next to the cursor.

Read before non-trivial changes:

- [README.md](README.md) — MVP scope, out-of-scope list, decided behavior, config/Action TOML schema.
- [CONTEXT.md](CONTEXT.md) — the ubiquitous language (Action, Input Source, Selection, Launcher, Direct Hotkey, Popover, Exchange) and the words to avoid. Naming in code and UI follows it.
- [docs/adr/](docs/adr/) — ADR-0001…0014. Modules cite them by number; a change that contradicts one needs a new ADR, not a quiet edit.

## Commands

```powershell
npm install                     # once
npm run tauri dev               # run the app (spawns vite on :1420, then cargo)
npm run tauri build             # MSI + NSIS on Windows, .app + .dmg on macOS
npm run dev                     # vite only, no Rust (webviews will fail on invoke)
npm run check                   # tsc --noEmit over the frontend
npx shadcn@latest add <name>    # pull a shadcn/ui component into src/components/ui
npm run icons                   # re-rasterize src-tauri/icons from assets/*.svg (pwsh scripts/gen-icons.ps1 on macOS)

cargo test                      # workspace tests (all in src-tauri)
cargo test action::tests::slugs_display_names        # one test
cargo test registry::                                # one module
cargo clippy --all-targets
cargo fmt                       # rustfmt.toml: max_width = 100
```

Tests are `#[cfg(test)]` modules beside the code. Platform- and network-touching code is deliberately untested; `platform::place_near_cursor` and `llm::sse` are pure functions precisely so they *can* be tested.

Half of `src-tauri/src/platform/` does not compile on the machine you are on, whichever machine that is. [.github/workflows/ci.yml](.github/workflows/ci.yml) runs `tsc`, `vite build`, `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` on **both** windows-latest and macos-latest — it is the compiler for the half you cannot build, and a green run on one platform is not evidence about the other (ADR-0013). The `bundle` job is `workflow_dispatch` only.

## Architecture

### Rust owns state; the windows are views

[src-tauri/src/state.rs](src-tauri/src/state.rs) holds every authoritative value (`Config`, `Registry`, `ExchangeManager`, `PopoverView`, hotkey bindings, pending Selection, previous foreground window or app). Frontend types in [src/lib/types.ts](src/lib/types.ts) mirror the serde shapes and are never a second source of truth. `PopoverPhase` — the resolution of `input_source` against the grab — is decided in Rust so the rule does not live in two places.

`AppState` is built in `load_state()` **before** `tauri::Builder`, because Tauri creates the configured windows during `build()` and a webview can invoke a command before `setup` runs.

### The three surfaces

`launcher.html`, `popover.html`, `settings.html` at the repo root are the Vite entry points ([vite.config.ts](vite.config.ts)) and the Tauri window URLs. Launcher and Popover are declared in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json), created hidden at startup, and only shown/hidden (ADR-0007) — WebView creation is too slow to pay per keypress. Settings is built on first use in `trigger::show_settings`. Window `close` is intercepted and turned into hide; the app quits only from the tray.

A new `window.*` API call from the frontend needs its permission added to [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json).

Every reveal is announced **before** the window is shown: `launcher:opened`, `popover:view`, `settings:opened`. The windows are reused and re-read their state asynchronously, so revealing first paints the previous trigger's contents for a few frames. `hide_popover` emits for the same reason.

Each surface is a shell plus its views, over a singleton store — one per window, safe because the window is created once (ADR-0007). Every store is a plain class over `Notifier` ([lib/store.ts](src/lib/store.ts)) that components subscribe to with `useStore`, and none of them touches the DOM.

- **Popover** — [Popover.tsx](src/popover/Popover.tsx) (shell, window keys, scroller) over [PopoverHeader](src/popover/PopoverHeader.tsx), [Turn](src/popover/Turn.tsx), [Composer](src/popover/Composer.tsx), backed by [popover/exchange.ts](src/popover/exchange.ts). Every state a turn can be in is decided in the store and rendered in `TurnView` — a state machine half-implemented in markup is what this split prevents.
- **Launcher** — [Launcher.tsx](src/launcher/Launcher.tsx) (shell, query, window keys, footer) over [ActionRow](src/launcher/ActionRow.tsx), backed by [launcher/actions.ts](src/launcher/actions.ts). A picker only: it reads the registry and writes nothing, so it can keep dying with its focus.
- **Settings** — [Settings.tsx](src/settings/Settings.tsx) (shell, subscriptions, delete dialog, pane element) over [SettingsNav](src/settings/SettingsNav.tsx) and [sections/](src/settings/sections/), backed by [settings/store.ts](src/settings/store.ts) (global config) and [settings/actions.ts](src/settings/actions.ts) (the Actions). The Actions section is one nav entry holding the whole list; the list and the editor are two views of one pane.

DOM work is an effect keyed off what changed in the store, never a hook the store calls: the Popover follows the stream by watching the current answer, and clears the composer by remounting it on `epoch`, a counter the store bumps on every reveal (ADR-0009). Window-level keys are bound to `window`, not to the card — clicking a row leaves focus on the body, and a handler on the tree would stop answering Escape from that moment on.

### One design system ([src/globals.css](src/globals.css))

All three surfaces are styled from one stylesheet (ADR-0008, ADR-0009, ADR-0010, ADR-0012 carry the measurements and the reasoning; the tokens carry their contrast ratios in comments beside them). The rules:

- **A component names no colour, size, or duration of its own.** A hardcoded `12px` or hex in a `className` is a bug.
- **globals.css is shadcn/ui's generated file** — `@import "tailwindcss"`, the `dark` variant, the `:root` / `.dark` token blocks, the `@theme inline` bridge, one `@layer base` reset. Nothing else belongs in it.
- **The accent is inversion, not a hue.** Base colour is `neutral` and the surface is monochrome; a selected nav item or segment is ink-filled with paper text, and that fill is the only fill on the pane. Brand colour lives in `--brand`, whose one consumer is `BrandMark`.
- **The type scale is ours, not Tailwind's.** `body` is 14px, with `--text-micro` 10.5 (tracked group head), `--text-meta` 11.5 (metadata), `--text-note` 12 (prose about state), `--text-quiet` 13 (pane lede, prose beside a control), `--text-title` 23, `--text-query` 16 (the Launcher's query box). Two faces only. Mono is 0.92em wherever it carries no size class, so a mono chip must **not** pin one.
- **One configuration, one card, and the pane draws no hairlines** (ADR-0012): the name and its explanation on the left, the control at the card's right edge. `Field` is the card, `FieldGroup` the tracked head plus its cards — and its optional `note`, the one statement about a whole group whose alternative is repeating itself on every card in it — `PaneHeader` the title/description/create action, `NavCard` a card that opens a screen instead of holding a control. The geometry (`rounded-lg border px-4.5 py-3.75`, 10px between cards) is written once, in `Field`. **No fill, no shadow, no bleed, one radius** — a tint would make a fill mean something other than "selected", and negative margins would buy title alignment by putting a card edge against the window frame. A control takes `--container-control` (340px) or `--container-control-wide` (420px) via `Field`'s `measure`, and the measure wraps the **control only** — the explanation runs to `--container-measure` (62ch). Air goes between `FieldGroup`s, not evenly between cards. **One weight of edge:** `--input` is `--border`, so a control's outline is the line of the card holding it, and the only state change either has under the pointer is `--border-strong` (`CARD_HOVER`, exported beside `CARD`) — the focus halo is 2px at `ring/25` and the focus *border* is what a keyboard user is answering. A card's name keeps shadcn's `font-medium`; the prose under it does not.
- **A card is right-aligned if its value is chosen and `stacked` if it is typed.** A text field cannot right-align against its own name: at the 780px minimum width a 420px control leaves 32px for the label. So the API key, the Base URL and all four of an Action's text fields pass `stacked` — name, control, explanation — and everything else parks its control at the card's right edge.
- **The Actions list keeps its hairline rows.** It is a list of records, not configurations, and it shares the Launcher's row *columns* — not its geometry, which is carded (ADR-0012, ADR-0014). `SettingsNav`'s divider and the `StatusBar`'s border are window chrome, not pane rules.
- **Motion is 150–200ms `ease-out`, everywhere.** `--default-transition-duration` and `--default-transition-timing-function` are set in `@theme` so that is the default, not a string each component repeats. `PaneEnter` animates opacity and a 4px *vertical* offset only (the pane is `overflow-y-auto`, so a horizontal transform flashes a scrollbar). Every animation carries `motion-reduce:`.
- Icons are `lucide-react`. `BrandMark` is the exception — identity, not a glyph.

Controls come from `npx shadcn@latest add` into [src/components/ui/](src/components/ui/) — library source, editable, but every edit is a divergence to justify. The current ones: `destructive-outline` and `success-outline` variants added; `outline`/`ghost` set to `font-normal` and `ghost` muted; `switch` rebuilt (off is paper with a bounded edge, since a fill means "on"; 13px knob in a 19px track, and it travels); `input`/`textarea` dropped shadcn's `text-base md:text-sm` iOS zoom guard, which rendered the same field a size larger in the two sub-`md` windows; `input`/`textarea`/`select` lightened the focus halo to `ring-[2px] ring-ring/25` once the rest border stopped being darker than the card; `xs` retuned for the Popover's quiet buttons; the button base given `duration-150 ease-out` and `active:scale-[0.98]`.

Beckon's own compositions live one level up in [src/components/](src/components/): `Field`, `FieldGroup`, `NavCard`, `PaneHeader`, `PaneEnter`, `Segmented`, `Callout`, `ConfirmDialog`, `ModelSelect`, `Temperature`, `OnOffSwitch`, `HotkeyInput`, `ActionCells`, `StatusBar`, `BrandMark`. Notable contracts:

- **A dropdown never covers the control it belongs to.** `select` defaults to `position="popper"` aligned to the trigger's left edge, not shadcn's `item-aligned`, which parks the open list over the trigger; the popper viewport drops shadcn's trigger-height pin, which would clip every list to one row.
- **Radix portals to `document.body`**, so `select` and `popover` are patched to default their container to the pane via [src/lib/pane.tsx](src/lib/pane.tsx) — the pane's `focusout` *is* the save protocol. `alert-dialog` is not patched: the delete confirmation is hosted by the shell, outside the pane. A new portalling component has to pick a side.
- `Segmented` is a track holding separate segments, not one welded row: the group is an edge and no fill (the selected segment carries the pane's one fill), and the choices inside it are rounded and set apart by a gap. A segment draws no edge of its own, so hover is a quiet `--muted` ground under a brightened label — a fill that is not the pane's inversion, kept a register below it and scoped to the inside of the group's edge — the same licence the Launcher's well takes (ADR-0014), and the two are the whole list. Stock gives hovered and selected the same `bg-accent`, which is what that cancels. An option may carry an icon, and the set is all-or-nothing: Theme has three, Input Source draws its glyph in the two lists instead.
- One destructive treatment, and it is the `destructive-outline` variant, not a class string per call site. Solid `destructive` belongs to the confirmation dialog alone — danger has to be legible at rest, since a keyboard never passes through hover.
- `Callout` is a rule and its text, never a card of its own and never a shadcn `Alert`. The tone lives in the rule alone; the prose stays muted and there is no icon, so nothing depends on colour alone.
- `HotkeyInput` is a chip beside a button, not one bordered control doing both.
- **One green button, and it is the API key's Save** (`success-outline`) — everything else on the pane writes as you type (ADR-0003), so it is the one control with a commit step. It carries colour, not a fill: Remove sits on the same line.
- The nav's attention dot is not drawn on the current row — on the inverted fill it is unreadable, and that row's problem is already on screen beside it.

**The Launcher's list is cards on a well, Settings' Actions list is the same row still ruled on paper, every other pane is cards and the Popover is neither** (ADR-0009, ADR-0010, ADR-0012, ADR-0014). Neither window adds a token, keyframe, or line of CSS of its own.

- The Launcher's row shares Settings' Actions row's four columns at the same fixed widths, so every Input Source parks at the same x. What it no longer shares is the geometry: the Launcher's row is a **card on a well** (ADR-0014) — `bg-background rounded-md border` inset in a `p-1.5 bg-muted` list with a `gap-1.25`, a `--border-strong` edge under the pointer, and the ink fill for the keyboard cursor, whose frame goes to `--primary` so a selected card is one block. **The tint is on the list, never on the chrome:** the header and footer keep their hairlines and stay paper, because a tint plus a border draws one boundary twice — the same argument `--sidebar` carries in Settings. Hover takes the edge rather than a ground because `--muted` is now what the card stands on — the same `--border-strong` Settings' `CARD_HOVER` takes. Hover does not select: the pointer never moves the cursor (that binding is what made a hover state impossible), and a click runs the row it landed on. **The cursor is hidden until the window is touched** — `wanted` is `null` on every summon, the first arrow reveals it where it already was (Down on the top match, Up on the last) and the first character typed reveals it too, since typing re-ranks the list and Enter must act on something visible. `SOURCES` / `SOURCE_ICON` / `sourceLabel` live in [lib/inputSource.ts](src/lib/inputSource.ts), the key chip in [components/Kbd.tsx](src/components/Kbd.tsx) — drawn through `formatAccelerator` from [lib/platform.ts](src/lib/platform.ts), which is where every word and shortcut that differs between the two platforms lives, for the same reason `inputSource.ts` exists — and the trailing Input Source / Direct Hotkey / conflict cells in [components/ActionCells.tsx](src/components/ActionCells.tsx) — shared precisely so the two lists cannot drift. The inversion classes on those cells are unconditional; only the Launcher marks a row `aria-selected`.
- The Popover has a fact the pane and picker do not: two speakers. So the side says who, and there is no label column and no hairline. Your input is a `--muted` card capped at 80% on the right, the answer runs left and bare capped at 11/12, and the gap between turns is the separator. **Both caps are proportions of the window** — they are two halves of one symmetry. A failure keeps a `Failed` marker over its sentence; a notice has no side, so the one notice that is an alarm (an Action needing a Selection, with none) is a `Callout` and the other two are ordinary prose at the same size. The three quiet buttons under a turn (reasoning disclosure, clamp toggle, Copy) sit at `--muted-quiet` as one shared constant. The header carries the Action, the model and the way out — and deliberately not the status, which the running turn already reports twice.
- The frameless windows are `rounded-lg border bg-background` on the root and `bg-transparent` on `<body>`. No `box-shadow`: the card fills the window rect, so the shadow you see is the compositor's.
- The waiting indicator and streaming caret are `animate-pulse` with `motion-reduce:animate-none`; the seconds counter beside them is what proves the wait is progressing.
- The composer grows with `field-sizing-content` between `min-h-9` and `max-h-30`.

The theme is stamped once by [lib/theme.ts](src/lib/theme.ts) as `.dark`, the class shadcn's own `dark` variant matches.

### Trigger flow ([src-tauri/src/trigger/](src-tauri/src/trigger/))

`mod.rs` is the flow; `window.rs` sizes, places and builds windows; `foreground.rs` remembers whose window was in front. `hotkey → grab → resolve input_source → show window`. Order is load-bearing:

- The grab happens **before any Beckon window is shown** (ADR-0006). Once the Launcher has focus, Beckon is the foreground window and a Ctrl+C would copy from the wrong process.
- The hotkey handler spawns a thread: the grab polls the clipboard for up to ~300 ms and must not block the event pump.
- `remember_foreground` skips Beckon's own HWNDs, and focus is handed back only once neither Launcher nor Popover is visible.
- The Launcher hotkey grabs eagerly into `pending_selection`; `pick_from_launcher` consumes it. Hiding the Launcher drops it.
- An empty grab is **not an error**: `selection` → `EmptySelection` hint and no request, `auto` → `NeedsInput`, `prompt` ignores the grab entirely.

### Exchange = one Popover's conversation ([src-tauri/src/exchange/](src-tauri/src/exchange/))

`mod.rs` is the bookkeeping (`ExchangeManager`, `TurnPlan`), `events.rs` the wire to the Popover, `turn.rs` the spawned task that runs one turn and drives that wire. In-memory only, never persisted (ADR-0004); `discard_all` on hide or on a replacing trigger. Follow-ups resend the full untruncated history. Each turn installs a fresh `CancellationToken` (a cancelled one stays cancelled). Partial text from an interrupted turn *is* committed to history, since it is what the user can see.

The Popover's state machine is driven by events, not return values: `exchange:first-token` (fires once; thinking text counts, because the UI must distinguish "waiting" from "streaming"), `exchange:delta` coalesced onto a 16 ms tick, then exactly one of `exchange:done` / `exchange:error` / `exchange:interrupted` — or silence on cancel, which the UI already knows about.

### LLM layer ([src-tauri/src/llm/](src-tauri/src/llm/))

`sse.rs` is a pure frame parser, `wire.rs` holds the response shapes plus the pure functions over them, `error.rs` is the one error type, `deepseek.rs` is the only place provider quirks live, `models.rs` is the model catalog, `client.rs` is only the requests — so everything but `client.rs` is testable without a network. **No HTTP timeout, on purpose** (README): a dead network must error immediately rather than spin, and a long thinking pause must not look like a hang. `thinking` is mapped explicitly and an unknown model is a hard error — omitting the field would silently leave DeepSeek thinking on. `LlmError::kind()` is the stable discriminant the frontend switches on.

`models::CATALOG` is read by **both** `deepseek::thinking_wire` and the Settings model dropdown (`get_models`), so the models offered and the models Beckon knows how to send cannot drift. Adding a model means adding a row there and nothing else. `get_models` prefers the endpoint's own `/v1/models` list and **never fails**: no credential, a rejected key, an offline machine or an empty list all fall back to the catalog and report the cause by kind. Whatever the config already names is always among the options — an unrecognised model is surfaced, never rewritten. Retired ids (`deepseek-chat`, `deepseek-reasoner`) stay in the catalog so an old config keeps working, but are not offered.

### Filesystem is the source of truth ([src-tauri/src/action/](src-tauri/src/action/), [reload.rs](src-tauri/src/reload.rs))

`%APPDATA%\Beckon` on Windows, `~/Library/Application Support/Beckon` on macOS — `config.toml` plus `actions/*.toml` (ADR-0003). Every mutation path — watcher event, Settings edit, startup — funnels through `reload::reload_config` / `reload_actions`, which re-read disk, re-derive hotkeys, and broadcast `config-changed` / `actions-changed`. Windows re-render from the snapshot; they never patch their own copy.

Rules that break subtly if ignored:

- An Action's **identity is its filename stem**; `name` is display only. Renaming `name` must not move the file.
- Mark `state.self_writes.mark(&path)` *before* any write, or the watcher echoes your own write back as an external change.
- Writes go through `atomic::write_atomic`; the watcher ignores dotfiles/temp files and reloads the whole directory rather than interpreting event kinds.
- A file that fails to parse is skipped and reported (`Registry::errors`), never fatal; the raw text stays editable via `read_action_raw` / `write_action_raw`.
- Hotkey registration is **derived state**: `hotkey::apply` unregisters everything and rebuilds from config + registry. Conflicts resolve by filename order, losers land in `hotkey_errors` and are flagged red. Failures are never silent — tray error icon + one-time balloon.
- The API key is only in the OS credential store — the Windows Credential Manager, the login Keychain on macOS (service `Beckon`, ADR-0005, ADR-0013). "No credential", "read error" and "key rejected" must stay three distinguishable outcomes all the way to the UI.
- A missing config file or missing field is a default, never an error; a *corrupt* config is reported, never overwritten.

### The editing surface is an editor, not an owner ([src/settings/](src/settings/))

There is no Save button and there must never be one (ADR-0003). **Settings is the only place anything is authored** — the global config (credential, Launcher hotkey, theme, model defaults) *and* the Actions; the Launcher writes nothing. Two stores, one per kind of file, both driving [src/lib/saveSlot.ts](src/lib/saveSlot.ts): [settings/store.ts](src/settings/store.ts) for `config.toml`, [settings/actions.ts](src/settings/actions.ts) for `actions\*.toml`. Components receive values and callbacks, never calling `saveConfig` / `saveAction` themselves. The Action store reads `defaults` and the model catalog off the config store rather than fetching its own.

- **Saving echoes back at the window that saved.** `save_config` → `reload_config` → `config-changed` (and `save_action` → `actions-changed`), broadcast to every window including the one that caused it. So the events being defended against are mostly our own writes arriving mid-keystroke, not the file watcher.
- A snapshot is refused while a text field in the pane has focus **or** a write is pending, then *held* and applied when both clear — dropping it would leave the form permanently stale after an external edit.
- Focus is read from the DOM when the event arrives, not tracked in per-field flags. The flags this replaced covered two of eleven inputs, so every field added later silently opted out.
- The navigation column sits **outside** the pane element, so changing section fires the pane's blur and flushes the slot. No route change can strand an unwritten edit.
- `ModelSelect` is controlled — `value=` + `onChange`, **never** a two-way binding — and refuses to write `""` where no inherit option exists. Both halves stop a configured model being silently rewritten before the catalog lands. Radix rejects an item valued `""`, so inherit rides a sentinel mapped at both edges of that one file.
- `save_action` re-probes the Direct Hotkey and refuses the *whole* write if it cannot be registered — so while an outside app holds an Action's hotkey, even renaming it fails. The editor says so and offers to clear the hotkey.
- The window is reused (ADR-0007): `settings:opened` clears the last visit's typed API key and test result and closes whatever Action it left open. The first open misses the event because the window is still being built, so `Settings.tsx` always loads itself too.
- An Action's `[model]` overrides are cards like any other (ADR-0011, ADR-0012), via `Field`'s `override` prop. The control is live whether the key is present or absent and shows the **effective** value, so touching it is what overrides — one gesture for a select, a switch and a slider alike. What the prop adds is the least that still tells the truth: a dot beside the name and a revert control naming the default, both on an overridden row only, plus `FieldGroup`'s `note` for the rest. The dot hangs out of the flow in the card's own padding, so it neither indents the group's names past every other name on the pane nor shifts the row that carries it; the revert slot is held open at the card's right edge whether it is filled or not, so the controls above it stay aligned.
- A field's explanation is a permanent line under the name, and there is no exception: the bubble (`InfoHint`) existed for the collapsed override rows alone and went with them.
- **The Action editor is two screens** (ADR-0012). The four typed fields — name, description, system prompt, user template — live behind one `NavCard` on the `Definition` screen, in **one** card via `Field`'s `bare`, each at the card's full width rather than a `measure` (they are one configuration, and it is the only card on the pane with no hover, since there is nothing else on that screen to move to); the main screen holds only choices. Which screen is open is `Editing.screen` in the store, `showScreen` flushes the save slot before it moves, the back control sits outside the form element, and `PaneEnter` is keyed on the screen as well as the route and the file. A warning belonging to the text screen is carried onto the card that opens it, since a warning nobody can see until they click is not a warning.

Keeping the forms out of the Launcher is what lets `WindowEvent::Focused(false)` hide it unconditionally: nothing unwritten inside it to lose, and no dropdown or dialog of its own to survive.

### Platform isolation ([src-tauri/src/platform/](src-tauri/src/platform/))

All Win32 lives under `platform/windows/`, all AppKit and CoreGraphics under `platform/macos/`, both re-exported through `platform/mod.rs` alongside `platform/fallback.rs` so the crate still compiles on a third platform (ADR-0001, ADR-0013). Those stubs are not a Linux promise — they are what a leaked `SendInput` or `NSPasteboard` breaks first. Do not scatter `#[cfg]` into business logic.

- Each `selection.rs` documents the grab's step order, and the two are the same order: release physically-held modifiers, back up the clipboard, poll the change counter (`GetClipboardSequenceNumber` / `NSPasteboard.changeCount`), restore, drop the backup. Every step fixes a specific failure, on both.
- `platform::cursor` is **not** per-platform and that is deliberate (ADR-0013): it asks Tauri, because tao already normalises macOS's bottom-left screen space into the top-left one `set_position` takes, and a second copy of that flip is a second place for it to be wrong.
- `focus::window_handle` returns an `HWND` on Windows and *our own pid* on macOS, because the unit of focus there is the application. `is_ours` is asking the same question either way.
- `permission::input_permission` is the one thing with no Windows counterpart: macOS refuses `CGEventPost` **silently** without Accessibility trust, so an empty grab is ambiguous and the state has to be read rather than inferred. Windows answers `NotRequired`, which the UI must treat as "say nothing", not as "granted".

### macOS-only wiring that is easy to break

`macOSPrivateApi` in [tauri.conf.json](src-tauri/tauri.conf.json) plus the `macos-private-api` cargo feature are what make the Launcher and Popover's transparency legal; removing either is a build error on Windows too, by design. `LSUIElement` in [src-tauri/Info.plist](src-tauri/Info.plist) and `ActivationPolicy::Accessory` in `setup` are both needed — the plist stops the Dock tile existing, the policy call is what a `cargo run` gets. Tauri's default macOS menu is left in place because it is the only reason Cmd+C/Cmd+V work in Settings' text fields, and it owns Cmd+Q, which is why `ExitRequested` is not refused on macOS (ADR-0013). The tray icon is explicitly **not** a template image: template mode renders from alpha alone and would erase the error state, which is colour.

### Icons are generated, not edited ([assets/](assets/))

Every PNG and the ICO in `src-tauri/icons` is output from `npm run icons` ([scripts/gen-icons.ps1](scripts/gen-icons.ps1)); editing a raster by hand is how the old 32px and 256px icons drifted apart. The script rasterizes with `tauri icon`, already a devDependency. The app icon needs two passes: only the default pass emits `icon.ico`, and only a `--png` pass can ask for 256.

There are three sources. `assets/logo.svg` is the app icon; its gradients, glow and drop shadow collapse into a grey smear at 32px, so the tray renders from `assets/tray-normal.svg` / `assets/tray-error.svg` — the same silhouette redrawn flat, strokes fattened, no shadow margin. The two tray sources must stay geometrically identical and differ only in accent colour.

### Adding an IPC command

`#[tauri::command]` in the matching file under [src-tauri/src/commands/](src-tauri/src/commands/) — `config`, `actions`, `secrets`, `models`, `platform`, `windows` (validate + delegate, keep it thin; `commands/mod.rs` re-exports them flat) → register in `generate_handler!` in [src-tauri/src/main.rs](src-tauri/src/main.rs) → typed wrapper in [src/lib/ipc.ts](src/lib/ipc.ts) → payload type in [src/lib/types.ts](src/lib/types.ts). Any `file_name` arriving over IPC goes through `sanitize_file_name`. Errors are `String` for plain messages, `Failure { kind, message }` when the UI must react by cause.

## GitNexus

This repo is indexed by GitNexus; the MCP tools (`impact`, `context`, `query`, `detect_changes`) and their usage rules live in the untracked local `AGENTS.md` / `.claude/`.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **beckon** (1510 symbols, 3671 relationships, 124 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
