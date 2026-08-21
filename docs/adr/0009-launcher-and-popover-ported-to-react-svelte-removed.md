---
status: accepted
---

# The Launcher and the Popover are React too; Svelte and `app.css` are removed

[ADR-0008](0008-settings-is-react-with-shadcn-ui-and-tailwind.md) moved Settings to React on shadcn/ui and said the seam it opened was meant to close. It is closed. The Launcher and the Popover are React, styled from `src/globals.css` and Tailwind utilities, and the following are deleted:

- `src/app.css` — 446 lines: 65 tokens, the measured palette, the element base, the scrollbar rules and the two shared keyframes.
- Five `.svelte` components and their two rune stores, about 1 100 lines including roughly 700 of scoped component CSS.
- `src/lib/icons/` — 21 hand-rolled inline SVGs and their barrel.
- `svelte.config.js`, the `svelte` / `svelte-check` / `@sveltejs/vite-plugin-svelte` dependencies, and the Svelte half of `npm run check`, which is now just `tsc --noEmit`.
- `src/lib/boot.ts`. There is one `mountSurface` again, in `src/lib/boot.tsx`.
- The `data-theme` stamp in `src/lib/theme.ts`. One resolution, one stamp: `.dark`.

## The shape each window took

The question the prototype answered was not "which framework" — that was settled — but **what the ledger means for a window that is not a form**. Three variants of each surface were mocked at their real pixel sizes and judged side by side.

**The Launcher is the ledger row at picker density.** Full-bleed rows closed by a hairline, the same four columns as the Actions list in Settings — name over description, a fixed Input Source column, a fixed hotkey column — and the selected row ink-filled with paper text. The fixed columns are the point: with the hotkey chip optional, an ordinary flex row parks every Input Source at a different x and a list of eight Actions reads as ragged. The two lists are now literally the same row, which is why `SOURCE_ICON`, `sourceLabel` and the key chip moved to `src/lib/inputSource.ts` and `src/components/Kbd.tsx` rather than being written twice.

The rejected alternative worth recording is the third variant: a query-first window with no per-row metadata and no footer, on the argument that a picker summoned by a hotkey is looked at for half a second with the hands already typing, and therefore wants *less* structure than a pane read at rest. It was rejected because the footer is where the Selection count lives, and the Selection count is the one fact this window knows that its list cannot show — an Action that works on a Selection is about to run against nothing, and the window that says so is this one.

**The Popover is the ledger applied to turns.** A fixed label column holds `You`, the model's name, `Thinking` and — when it fails — `Failed`, against a content column, one hairline per row. A transcript with the question in a quoted block was the alternative, and the ledger won for the reason the ledger won in Settings: the label column is what stops "what I asked", "what it thought" and "what it said" being told apart by indentation and colour alone.

**Superseded in part by [ADR-0010](0010-popover-turns-are-sided-not-ruled.md):** the label column described in this section is gone, and the Popover is sided instead. Everything else here — the port itself, the Launcher, the pulse, the consequences below — still holds.

The label column was set in `Field`'s register — 13px, sentence case, normal weight — and not in the tracked uppercase micro it was first drawn in. There is one ledger across the three windows, so a row label is a row label; the micro register belongs to a group head, which is *quieter* than the rows under it, and uppercase fails outright on the one label here that is not a word, since a model id is lowercase and hyphenated and `DEEPSEEK-REASONER` reads as a shout. `ROW_LABEL` is exported so the composer's label cannot drift from the rows above it.

The header carries the Action, the model and the way out, and deliberately not the status. A running turn reports itself in its own row and again in the bar along the bottom where Stop is; a third report in the title bar is what made the old header a status display with a title in it.

## The travelling rail, and the one keyframe we did not add

ADR-0008 flagged this as the one animation with no Tailwind equivalent. It is resolved by not needing it: the waiting indicator is a full-width bar on `animate-pulse`, which Tailwind ships.

The old rail travelled, and travel had to be disabled under `prefers-reduced-motion` with a *static* substitute, because a frozen travelling bar reads as a stalled request. A pulse has no such failure mode — frozen, it is simply a bar — so the reduced-motion branch is `motion-reduce:animate-none` like everything else, and the seconds counter beside it is what still proves the wait is progressing. Same for the streaming caret, which was `breathe` and is now `animate-pulse` at the same reading.

So the two surfaces add no keyframes and no CSS, and one token: `--text-query`, the one step *above* body, with one consumer — the Launcher's query box, which is that window's subject sitting in a 56px bar where body size reads as timid. It is in the scale rather than borrowed from Tailwind's `text-base` so the scale still lists every size the product uses. `src/globals.css` is the whole design system for all three windows, and that is the only line either surface added to it.

## What did not change

- Every behaviour in the README and in ADRs 0002–0007. Esc still cancels a live request before it closes the window; partial text from an interrupted turn is still committed; an empty grab is still not an error; the Launcher still writes nothing and Settings is still the only place anything is authored (ADR-0003).
- The frameless windows still paint one card filling the window rect, at the ~8px radius Windows 11 rounds an undecorated window at, with no `box-shadow` of their own — the shadow is DWM's. That is now `rounded-lg border bg-background` on the root element and `bg-transparent` on the two `<body>`s, instead of a `.surface` class.
- The stores are still module-level singletons, because there is still exactly one of each window and it is never destroyed (ADR-0007). Runes became plain classes over `Notifier`, the same move Settings made.

## Consequences

- **The stores no longer touch the DOM.** The Svelte stores carried `onStream` / `onIdle` / `onReset` hooks the shell installed on mount, because a rune store scrolling a div is worse than a hook. In React those are effects keyed off what changed: following the stream watches the answer, and the composer clears by remounting on `epoch` — a counter the store bumps on every reveal — which resets the draft and the grown height in one move, since both belong to the element rather than to us.
- **Auto-growing the composer is the browser's job.** `field-sizing-content` with `min-h-9 max-h-30` replaces the resize handler and the mirrored `max-height` it needed.
- **Window keys are bound to `window`, not to the card.** The Svelte surfaces used `<svelte:window on:keydown>`; a React handler on the tree would stop answering Escape the moment the mouse was used, because clicking a row leaves focus on the body.
- **Bundle.** Launcher 7.8 kB and Popover 11.6 kB of their own, over a 237 kB shared React/Radix chunk the Settings window was already paying for and which all three now share. Resident cost is unchanged: the same three WebViews, two of them created hidden at startup.
- **Two library divergences the narrow windows exposed.** shadcn sizes `input` and `textarea` at `text-base md:text-sm` — its iOS zoom guard. Settings is 980px wide and above that breakpoint; the Launcher at 680 and the Popover at 620 are below it, so the identical field rendered 16px in two windows and 14px in the third. Both are now one size. And `Kbd` no longer names a size at all: `font-mono` is set at 0.92em in the base layer precisely so mono sits level with the sans beside it, and pinning the chip to `--text-meta` cancelled that and froze one absolute size onto an object that appears inside an 11.5px legend and inside a 14px row. An unregistered hotkey moved to mono with it — the same string was being set in two faces depending only on whether it happened to register.
- `src/lib/` is no longer "the framework-agnostic part". With one framework there is nothing to keep out of it, and `inputSource.ts` exports lucide components.
