<script lang="ts" generics="T extends string">
  // Three-or-so mutually exclusive choices, shown rather than hidden behind a
  // dropdown. A real radiogroup with roving tabindex: one tab stop, arrows
  // move within it.
  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    value: T;
    options: Option[];
    label: string;
    id?: string;
    describedBy?: string;
    onchange: (value: T) => void;
  }

  let { value, options, label, id, describedBy, onchange }: Props = $props();

  function onKeydown(event: KeyboardEvent) {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 :
                 event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const at = options.findIndex((option) => option.value === value);
    const next = options[(at + step + options.length) % options.length];
    onchange(next.value);
  }
</script>

<!-- The group itself is deliberately not focusable: with roving tabindex the
     checked radio is the tab stop, which is what ARIA's radiogroup pattern
     asks for. A tabindex here would add a second, useless stop. -->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  class="segmented"
  role="radiogroup"
  aria-label={label}
  aria-describedby={describedBy}
  {id}
  onkeydown={onKeydown}
>
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={option.value === value ? 0 : -1}
      class:active={option.value === value}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--bg-sunken);
    align-self: flex-start;
  }

  button {
    border: none;
    background: none;
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-3);
    color: var(--text-dim);
    font-size: var(--text-sm);
  }

  button:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: transparent;
    color: var(--text);
  }

  button.active {
    background: var(--bg-raised);
    color: var(--text);
    font-weight: var(--weight-medium);
    box-shadow: var(--shadow-sm);
  }
</style>
