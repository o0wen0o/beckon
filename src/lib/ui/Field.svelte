<script lang="ts">
  // One layout for every labelled control: label (plus its explanation behind
  // an info icon), control, then whichever of warning / error applies.
  // Centralised so a field added later cannot invent its own spacing or forget
  // to wire `aria-describedby`.
  //
  // The hint lives in the icon's bubble, never inline: warnings and errors are
  // the only prose that earns a permanent line, because those are conditions
  // the user has to act on rather than background.
  import type { Snippet } from "svelte";
  import InfoHint from "./InfoHint.svelte";

  interface Props {
    label: string;
    /** Shown on hover/focus of the info icon; always in the a11y tree. */
    hint?: string;
    /** Red, and replaces the hint while it is present. */
    error?: string | null;
    /** Amber. Not a failure — something worth knowing. Coexists with the hint. */
    warning?: string | null;
    /**
     * Which way the hint bubble hangs. A bubble is absolutely positioned but
     * still laid out, so one that hangs off the right of a scrolling pane adds
     * to its `scrollWidth` and leaves a horizontal scrollbar behind while
     * showing nothing at all. A field in a right-hand column passes `"end"`.
     */
    hintAlign?: "start" | "end";
    control: Snippet<[{ id: string; describedBy: string | undefined }]>;
  }

  let {
    label,
    hint,
    error = null,
    warning = null,
    hintAlign = "start",
    control,
  }: Props = $props();

  const id = $props.id();
  const descriptionId = `${id}-description`;
  const hintId = `${id}-hint`;
  // The control is described by whatever is loudest, and by the hint whenever
  // there is one — the hint is invisible most of the time, so dropping it from
  // the description is the one place it would be lost outright.
  const described = $derived(
    [error || warning ? descriptionId : null, hint ? hintId : null].filter(Boolean).join(" ") ||
      undefined,
  );
</script>

<div class="field">
  <div class="label-row">
    <label class="label" for={id}>{label}</label>
    {#if hint}<InfoHint text={hint} id={hintId} align={hintAlign} />{/if}
  </div>
  {@render control({ id, describedBy: described })}

  {#if error}
    <p class="note error" id={descriptionId}>{error}</p>
  {:else if warning}
    <p class="note warn" id={descriptionId}>{warning}</p>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-4);
  }

  .label-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .label {
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    color: var(--text-dim);
  }

  .note {
    margin: 0;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .note.warn {
    color: var(--warn);
  }

  .note.error {
    color: var(--danger);
  }
</style>
