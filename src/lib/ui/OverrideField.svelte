<script lang="ts">
  // An Action's `[model]` values are all optional: absent means "inherit the
  // global default". Expressing that as an "inherit" entry in a dropdown makes
  // it look like a value somebody picked, and hides what is being inherited.
  //
  // So the row *is* the control: it reads as a value with its provenance, and
  // opening it is what overrides — there is no separate button to press first.
  // It closes again when focus leaves, which keeps a list of these readable as
  // a summary of the Action rather than as a wall of open forms. Closing is
  // presentation only; the override itself is on disk the moment it is made.
  import { tick, type Snippet } from "svelte";
  import InfoHint from "./InfoHint.svelte";

  interface Props {
    label: string;
    /** How the inherited value reads, e.g. "deepseek-v4-flash". */
    inherited: string;
    /** How the overriding value reads. Ignored while inheriting. */
    current: string;
    overridden: boolean;
    /** true → seed from the inherited value; false → write null. */
    onoverride: (on: boolean) => void;
    control: Snippet;
    hint?: string;
    error?: string | null;
    warning?: string | null;
  }

  let {
    label,
    inherited,
    current,
    overridden,
    onoverride,
    control,
    hint,
    error = null,
    warning = null,
  }: Props = $props();

  let expanded = $state(false);
  let root = $state<HTMLDivElement | null>(null);

  async function open() {
    if (expanded) return;
    expanded = true;
    // The click itself is the override — nothing else to press.
    if (!overridden) onoverride(true);
    await tick();
    // Focus the control, which is also what arms the collapse: without focus
    // inside, `focusout` would never fire and the row would stay open.
    root?.querySelector<HTMLElement>("input, select, textarea, button")?.focus();
  }

  function onFocusOut(event: FocusEvent) {
    const next = event.relatedTarget;
    if (next instanceof Node && root?.contains(next)) return;
    expanded = false;
  }

  function useDefault() {
    onoverride(false);
    expanded = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="override"
  class:overridden
  class:expanded
  bind:this={root}
  onfocusout={onFocusOut}
  onkeydown={(event) => {
    // Esc closes the row rather than the window it sits in.
    if (event.key === "Escape" && expanded) {
      event.stopPropagation();
      expanded = false;
    }
  }}
  role="group"
  aria-label={label}
>
  {#if expanded}
    <div class="head">
      <span class="label">{label}</span>
      {#if hint}<InfoHint text={hint} />{/if}
      <button class="link" onclick={useDefault}>Use the default</button>
    </div>
    <div class="control">{@render control()}</div>
  {:else}
    <button class="summary" onclick={open}>
      <span class="label">{label}</span>
      <span class="value" class:inheriting={!overridden}>{overridden ? current : inherited}</span>
      <span class="tag">{overridden ? "overridden" : "from Model defaults"}</span>
    </button>
  {/if}

  {#if error}
    <p class="note error">{error}</p>
  {:else if warning}
    <p class="note warn">{warning}</p>
  {/if}
</div>

<style>
  .override {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: var(--space-1);
  }

  .override.overridden {
    border-color: var(--border-strong);
  }

  .override.expanded {
    background: var(--bg-raised);
    padding: var(--space-3);
  }

  .summary {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    width: 100%;
    border: none;
    background: none;
    text-align: left;
    padding: var(--space-2);
    border-radius: var(--radius-sm);
  }

  .summary:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: transparent;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .label {
    flex: none;
    font-size: var(--text-sm);
    font-weight: var(--weight-semibold);
    color: var(--text-dim);
  }

  .value {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text);
  }

  /* An inherited value is shown, but never as though it were this Action's. */
  .value.inheriting {
    color: var(--text-faint);
  }

  .tag {
    flex: none;
    font-family: var(--font-small);
    font-size: var(--text-xs);
    color: var(--text-faint);
  }

  .link {
    margin-left: auto;
    border: none;
    background: none;
    padding: 0;
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
    text-decoration: underline;
  }

  .link:hover:not(:disabled) {
    background: none;
    border-color: transparent;
    color: var(--accent);
  }

  .note {
    margin: 0 var(--space-2) var(--space-1);
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
