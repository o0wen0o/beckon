---
status: accepted
---

# Style the surfaces from one token file; take behaviour, not styling, from a component library

[src/app.css](../../src/app.css) stays the only place a colour, radius, spacing step or duration is
named, and every surface styles itself out of those tokens. Third-party UI packages are allowed in
only where they supply **behaviour** — keyboard handling, focus management, floating placement — and
bring no palette of their own.

Three packages qualified and are now dependencies:

| Package | Version | What it supplies |
|---|---|---|
| `bits-ui` | 2.18 | Headless `Select`: roving focus, typeahead, floating placement, ARIA |
| `lucide-svelte` | 1.0 | The icon set. One family, one stroke weight, SVG |
| `@fontsource-variable/{inter,jetbrains-mono}` | 5.x | The two faces, bundled — no runtime network |

## The styled kits were the obvious answer, and they are the wrong one

The Svelte 5 field was surveyed at the versions current when this was written: `shadcn-svelte` 1.4,
`@skeletonlabs/skeleton` 5.0, `flowbite-svelte` 1.33, `carbon-components-svelte` 0.110, `svelte-ux`
1.0, plus the headless layers `bits-ui` 2.18 and `melt` 0.44. Any of them would render a nicer
button than a hand-written one.

The disqualifier is not bundle size, and it is not Tailwind: these are local WebViews and the CSS
tree-shakes. It is that **every styled kit owns the theme mechanism**. shadcn-svelte, Skeleton and
Flowbite all key dark mode off a `.dark` class on the root element and all ship their own token
vocabulary (`--background`, `--surface-*`, `--primary-*`) for components to consume.

Beckon keys dark mode off `data-theme`, written by [src/lib/theme.ts](../../src/lib/theme.ts) from
`Config::theme`, with **no** `prefers-color-scheme` fallback — a machine set to dark must still get
the light default until the user asks otherwise ([ADR-0003](./0003-actions-as-toml-files-with-filesystem-as-source-of-truth.md)
makes the config authoritative, and the theme is config-derived state like everything else).
Adopting a kit means either running two token vocabularies side by side — the exact thing app.css's
own header forbids — or rewriting the theme contract to suit a dependency. Neither is worth a
prettier button on three windows totalling under 2000 lines of markup.

The second reason is smaller but points the same way: the widget in each surface that actually
matters — the Launcher's listbox, the Popover's streaming transcript, the hotkey recorder — is
bespoke and keyboard-first. A kit's inventory is dialogs, sheets and dropdown menus. Beckon has one
dropdown.

## Why a headless Select is worth a dependency at all

A native `<select>` has two defects here that no amount of CSS fixes. Its popup is drawn by the
platform, so it ignores the palette, the radius and the font every other control in the window
shares. And an `<option>` is a single string, so a model's one-line description had to be exiled to
a paragraph under the field instead of sitting on the row it describes — which is why picking a
model used to mean reading two places at once.

Bits UI is behaviour only: no tokens, no theme mechanism, no Tailwind peer dependency. The wrapper
is [src/lib/ui/Select.svelte](../../src/lib/ui/Select.svelte) — one file, styled from the same
tokens as everything else.

## Consequences

- **`src/lib/ui/` is for wrappers, not a component library.** A file lands there when two surfaces
  need the same behaviour. Anything used once stays in the surface that uses it.
- **Bits UI portals its content to `document.body`**, which Svelte's scoped styles never reach, so
  the wrapper's rules are `:global()` and every selector is prefixed `bk-`. That prefix is the only
  reason the global block is safe; dropping it would leak styling into the whole app.
- **The Select is controlled** (`value` in, `onchange` out, never `bind:`). Binding would let the
  list write back whatever it settled on before the model catalog arrived, which is exactly how a
  configured model gets silently replaced.
- **Icons are `lucide-svelte` only.** Mixing a second set — or a text glyph standing in for an icon,
  which is what the Popover's close button used to be — breaks stroke weight and optical size
  against everything around it. Lucide draws at stroke-width 2 for 24px; app.css thins the whole set
  to 1.75 in one rule, because these windows use icons at 14–17px.
- **Fonts are bundled, never fetched.** A `@fontsource` package is a build-time dependency that
  emits woff2 into `dist/`; a Google Fonts `@import` would be a network request on a surface that
  must open in milliseconds and must work offline.
- The palette is checked against WCAG AA rather than assumed. The previous `--text-faint` failed at
  3.3:1 in light and 3.5:1 in dark against the surfaces it was used on — the 11–12px metadata labels
  were the least readable text in the app.
