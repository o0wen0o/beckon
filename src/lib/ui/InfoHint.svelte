<script lang="ts">
  // A field's explanation, moved off the form and behind an icon: a settings
  // pane where every control carries two lines of prose reads as documentation
  // rather than as a form, and the density hides the controls themselves.
  //
  // The text stays in the accessibility tree at all times — it is the element
  // `aria-describedby` points at — so hiding it is a visual decision only. That
  // is also why this is a `<span tabindex=0>` and not a `<button>`: there is
  // nothing to activate, and a button would promise an action.
  import { Info } from "../icons";

  interface Props {
    text: string;
    /** The id the described control points at. */
    id?: string;
    /** Which side of the icon the bubble hangs on. */
    align?: "start" | "end";
  }

  let { text, id, align = "start" }: Props = $props();

  // Hover is the affordance; the click is for everyone hover does not serve —
  // a keyboard reaches it as a button, a touch or a trackpad tap pins it open.
  let pinned = $state(false);
</script>

<span class="info" class:pinned data-align={align}>
  <button
    class="dot"
    aria-label="More information"
    aria-expanded={pinned}
    onclick={() => (pinned = !pinned)}
    onblur={() => (pinned = false)}
  >
    <Info size={13} />
  </button>
  <!-- Not `role="tooltip"`: the description is consumed through
       aria-describedby on the control, and a second announcement path would
       read the same sentence twice. -->
  <span class="bubble" {id}>{text}</span>
</span>

<style>
  .info {
    position: relative;
    display: inline-flex;
    vertical-align: middle;
  }

  .dot {
    display: flex;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-faint);
    border-radius: var(--radius-pill);
    cursor: help;
    transition: color var(--dur-fast) var(--ease-out);
  }

  .dot:hover:not(:disabled) {
    background: none;
    border-color: transparent;
  }

  .dot:active:not(:disabled) {
    transform: none;
  }

  .info:hover .dot,
  .dot:focus-visible {
    color: var(--accent);
  }

  .bubble {
    position: absolute;
    z-index: var(--z-overlay);
    left: 0;
    top: calc(100% + var(--space-1));
    width: max-content;
    max-width: 280px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bg-raised);
    box-shadow: var(--shadow-md);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
    line-height: 1.45;
    color: var(--text-dim);
    /* Hidden the way a tooltip is: still rendered, still described, not
       clickable — `display: none` would take it out of the a11y tree too. */
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transform: translateY(-2px);
    transition:
      opacity var(--dur-fast) var(--ease-out),
      transform var(--dur-fast) var(--ease-out),
      visibility var(--dur-fast);
  }

  /* Anchored to the right edge instead, for an icon near the window's. */
  .info[data-align="end"] .bubble {
    left: auto;
    right: 0;
  }

  .info:hover .bubble,
  .info.pinned .bubble,
  .dot:focus-visible + .bubble {
    opacity: 1;
    visibility: visible;
    transform: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .bubble {
      transition: none;
      transform: none;
    }
  }
</style>
