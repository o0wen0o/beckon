// Hand-rolled Lucide-geometry icons as inline SVG components. Inline because
// the CSP allows no remote asset and `img-src` would still cost a request;
// hand-rolled because an icon package is a runtime dependency for a dozen
// glyphs in a tray utility (ADR-0001's footprint argument).
//
// Every icon shares one contract: a 24×24 viewBox, `currentColor` strokes at
// 1.75, a `size` prop defaulting to 16, and `aria-hidden` — an icon never
// carries the accessible name, the control around it does.
export { default as ArrowLeft } from "./ArrowLeft.svelte";
export { default as Auto } from "./Auto.svelte";
export { default as BrandMark } from "./BrandMark.svelte";
export { default as Check } from "./Check.svelte";
export { default as ChevronRight } from "./ChevronRight.svelte";
export { default as Close } from "./Close.svelte";
export { default as Copy } from "./Copy.svelte";
export { default as Folder } from "./Folder.svelte";
export { default as Info } from "./Info.svelte";
export { default as Keyboard } from "./Keyboard.svelte";
export { default as ListIcon } from "./ListIcon.svelte";
export { default as Palette } from "./Palette.svelte";
export { default as Pencil } from "./Pencil.svelte";
export { default as Plug } from "./Plug.svelte";
export { default as Plus } from "./Plus.svelte";
export { default as Prompt } from "./Prompt.svelte";
export { default as Retry } from "./Retry.svelte";
export { default as Search } from "./Search.svelte";
export { default as Send } from "./Send.svelte";
export { default as Sliders } from "./Sliders.svelte";
export { default as TextSelect } from "./TextSelect.svelte";
export { default as Warning } from "./Warning.svelte";
