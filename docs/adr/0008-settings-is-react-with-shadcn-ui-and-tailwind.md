---
status: accepted
---

# Settings is built in React on shadcn/ui and Tailwind; the hand-rolled design system is retired there

Settings was a Svelte surface styled entirely from [src/app.css](../../src/app.css) — 65 tokens, a measured palette, and about 1700 lines of scoped component CSS across an eleven-component UI kit. It is now React, and every control in it comes from shadcn/ui with Tailwind utilities for layout. `src/app.css` still serves the Launcher and the Popover; nothing in Settings reads it.

## Why

The base colour is shadcn's `neutral`, not `slate`. Slate's greys carry chroma 0.007–0.046 at hue 250–265, which is the brand's own blue family, so the brand accents did not read as accents — they read as slightly bluer grey. At chroma 0 the greys stop competing and `--primary` is the only chromatic thing in the surface, which is what lets a two-pixel rail carry "this is the current section" on its own.

The kit was the whole cost of the surface. Eleven components existed only to give Settings a switch, a select, a slider, a modal, a tooltip and a segmented control, and each carried its own CSS, its own focus handling and its own accessibility wiring — `Segmented`'s roving tabindex, `ConfirmDialog`'s `showModal()` guard, `InfoHint`'s always-in-the-a11y-tree bubble. Those are solved problems with a maintained implementation behind them. shadcn/ui is that implementation, it is copy-in source rather than a dependency to track, and it is React-only.

Keeping the surface in Svelte and using shadcn-svelte was the alternative. It was rejected: the point of adopting shadcn is to stop maintaining a bespoke kit, and the React version is the one the upstream project develops against.

## What this costs, stated plainly

**Two frameworks in one repo.** Settings is React; the Launcher and the Popover are still Svelte. Each Vite plugin claims its own extensions and the surfaces share no components — only `src/lib/*.ts`, which is framework-agnostic — so this is a seam, not a conflict. It is meant to close: the other two surfaces follow.

**Bundle size.** The Settings chunk went from ~13 kB to ~405 kB (~125 kB gzipped), which is React plus Radix. It is build-time weight in a WebView that is already resident, not a second process, so [ADR-0001](0001-tauri-v2-on-windows-only.md)'s decision — Tauri over Electron, ~30 MB resident instead of 150–300 MB — is untouched. But ADR-0001's reasoning is what justified hand-rolling the icons rather than taking a package, and that argument no longer holds for this surface: Settings uses `lucide-react`, which is what shadcn/ui's own components import. The Svelte surfaces keep the hand-rolled set in [src/lib/icons/](../../src/lib/icons/) until they are ported.

**The measured palette, partly.** [src/app.css](../../src/app.css) documented a contrast ratio against every surface for every text and border colour, because brand cyan is 1.4:1 on white and needed darkening by hand. Settings does not carry that palette forward — it uses shadcn's generated token set, with the brand blue in `--primary` and `--ring`.

What *is* carried forward is the practice. The generated set was measured rather than assumed, which turned up two failures in it: `--muted-foreground` was 4.34:1 on `--muted`, a pairing live in the hotkey chip and in `Segmented`, and `--input` — the sole boundary of a text field, and so a UI component under WCAG 1.4.11 — was 1.26:1. Both are corrected in [src/globals.css](../../src/globals.css) with the ratio written beside the value, and `--input` is now split from the decorative `--border` exactly as app.css splits `--border` from `--border-strong`. So the surface is not individually verified end to end the way app.css was, but every value that deviates from the generated set is.

**One animation.** The travelling-rail keyframe has no Tailwind or `tw-animate-css` equivalent. Settings only needed the save spinner, which `animate-spin` covers; the Popover's waiting rail does not port and will need a shadcn `Progress` or `Skeleton` when that surface moves.

## What did not change

Every rule in [ADR-0003](0003-actions-as-toml-files-with-filesystem-as-source-of-truth.md) survives, because none of it lived in the components. There is still no Save button. The two stores still own every value and every write; components still receive values and callbacks. A snapshot is still refused while a text field in the pane has focus or a write is pending, still held rather than dropped, and focus is still read from the DOM rather than tracked in per-field flags.

Two mechanisms needed rebuilding to keep that true:

- **Radix portals its overlays to `document.body`**, which would put an open dropdown outside the pane — and the pane *is* the save protocol: `textFocusHeld` asks whether focus is inside it, and it flushes the debounced write on its own `focusout`. So the pane publishes itself through [src/lib/pane.tsx](../../src/lib/pane.tsx) and `select` and `popover` default their portal container to it. `alert-dialog` deliberately does not: the delete confirmation is hosted by the shell, outside the pane, exactly as the old native `<dialog>` was.
- **Svelte's runes were the stores' reactivity.** They are now plain classes over a `Notifier` that components subscribe to with `useSyncExternalStore` ([src/lib/store.ts](../../src/lib/store.ts)). A module-level singleton is still the honest shape, for the same reason as before: there is exactly one Settings window and it is never destroyed ([ADR-0007](0007-windows-are-created-hidden-at-startup-and-reused.md)), which is what lets `settings:opened` reset the last visit's leftovers from outside the React tree.

`ModelSelect` keeps its two rules — a controlled value with an `onChange`, never a two-way binding, and a refusal to write `""` where no inherit option exists. Radix rejects an item whose value is the empty string, so inherit is carried as a sentinel mapped at both edges of that one file.

## Consequences

- A new control in Settings comes from `npx shadcn@latest add`, not from `src/components/`. `src/components/ui/` is library source: editable, but every edit is a divergence to justify. Three so far: the two portal-container patches, and a `destructive-outline` button variant — red text and a red edge at rest with the fill on hover only, because a destructive button beside an ordinary one must not be the loudest thing on the pane, and hover is not a state a keyboard reaches. Solid `destructive` is left to the confirmation dialog, which the user has already chosen to open.
- Two tokens are added that shadcn does not ship, `--warning` and `--success`, plus a `--text-2xs` step below Tailwind's `text-xs`. The alternative was `text-amber-700 dark:text-amber-400` and `text-[11px]` written out at every use site, and a component naming its own colour or size is the one thing the token layer exists to prevent. `--destructive-foreground` went the other way and was deleted: nothing consumed it, and the generated value is 2.77:1 on `--destructive` in dark.
- Settings names no colour, size or duration of its own — but now because Tailwind owns them, not `app.css`. A hardcoded hex or a raw `12px` in a `.tsx` file is still a bug.
- Dark mode is stamped twice: [src/lib/theme.ts](../../src/lib/theme.ts) writes `data-theme` for the Svelte surfaces and toggles `.dark` for shadcn's variant. One resolution, two stamps, until the port finishes and the first can go.
- `src/app.css` and `src/lib/icons/` are on a countdown. When the Launcher and the Popover are ported, both are deleted and this seam closes.
