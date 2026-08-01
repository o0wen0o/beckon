<script module lang="ts">
  export interface Option {
    value: string;
    label: string;
    /** Secondary line on the row. Absent means a one-line row. */
    description?: string;
  }
</script>

<script lang="ts">
  // The one dropdown in the app, wrapping Bits UI's headless Select.
  //
  // Two things a native `<select>` cannot do, and both of them matter here:
  // its popup is drawn by the OS, so it ignores the palette, the radius and the
  // font that every other control in the window shares; and an `<option>` is a
  // single string, so a model's one-line description has to be exiled to a
  // paragraph under the field instead of sitting on the row it describes.
  //
  // Deliberately **controlled**: `value` in, `onchange` out, never `bind:`.
  // Binding would let the list write back whatever it settled on before the
  // model catalog arrived, which is exactly how a configured model gets
  // silently replaced (see Settings.svelte).
  import { Select } from "bits-ui";
  import Check from "lucide-svelte/icons/check";
  import ChevronDown from "lucide-svelte/icons/chevron-down";

  interface Props {
    value: string;
    options: Option[];
    onchange: (value: string) => void;
    /** Needed only where no `<label>` wraps the field. */
    ariaLabel?: string;
    disabled?: boolean;
  }

  let { value, options, onchange, ariaLabel, disabled = false }: Props = $props();

  // A value the catalog does not carry still has to render as itself; falling
  // back to the placeholder would read as "nothing selected".
  const shown = $derived(options.find((option) => option.value === value)?.label ?? value);
</script>

<Select.Root
  type="single"
  {value}
  {disabled}
  items={options.map((option) => ({ value: option.value, label: option.label }))}
  onValueChange={onchange}
>
  <Select.Trigger class="bk-select-trigger" aria-label={ariaLabel}>
    <span class="bk-select-value">{shown}</span>
    <ChevronDown class="bk-select-chevron" size={15} aria-hidden="true" />
  </Select.Trigger>

  <Select.Portal>
    <Select.Content class="bk-select-content" sideOffset={6}>
      <Select.Viewport class="bk-select-viewport">
        {#each options as option (option.value)}
          <Select.Item class="bk-select-item" value={option.value} label={option.label}>
            {#snippet children({ selected })}
              <span class="bk-select-item-text">
                <span class="bk-select-item-label">{option.label}</span>
                {#if option.description}
                  <span class="bk-select-item-description">{option.description}</span>
                {/if}
              </span>
              {#if selected}
                <Check class="bk-select-check" size={15} aria-hidden="true" />
              {/if}
            {/snippet}
          </Select.Item>
        {/each}
      </Select.Viewport>
    </Select.Content>
  </Select.Portal>
</Select.Root>

<!-- The content is portalled to `document.body`, which scoped styles never
     reach; these have to be global, so every selector is prefixed. -->
<style>
  :global(.bk-select-trigger) {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: 34px;
    padding: 6px 10px;
    font: inherit;
    font-weight: 400;
    text-align: left;
    color: var(--text);
    background: var(--bg-input);
    border: 1px solid transparent;
    border-radius: var(--radius);
    cursor: pointer;
    transition:
      background var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }

  :global(.bk-select-trigger:hover:not(:disabled)) {
    background: var(--bg-input);
    border-color: var(--border-strong);
    color: var(--text);
  }

  :global(.bk-select-trigger[data-state="open"]) {
    border-color: var(--accent);
  }

  :global(.bk-select-value) {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.bk-select-chevron) {
    flex: none;
    color: var(--text-faint);
    transition: transform var(--dur) var(--ease);
  }

  :global(.bk-select-trigger[data-state="open"] .bk-select-chevron) {
    transform: rotate(180deg);
  }

  :global(.bk-select-content) {
    z-index: 50;
    /* Never narrower than the field it belongs to, never taller than the space
       the floating layer measured. */
    min-width: var(--bits-floating-anchor-width);
    max-height: min(320px, var(--bits-floating-available-height));
    overflow: hidden;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
  }

  :global(.bk-select-viewport) {
    max-height: inherit;
    overflow-y: auto;
    padding: var(--space-1);
  }

  :global(.bk-select-item) {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    outline: none;
  }

  /* Highlight follows the keyboard as well as the pointer — Bits UI sets
     `data-highlighted` for both, so there is one visual state to maintain. */
  :global(.bk-select-item[data-highlighted]) {
    background: var(--bg-hover);
  }

  :global(.bk-select-item-text) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  :global(.bk-select-item-label) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.bk-select-item-description) {
    font-size: 12px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.bk-select-check) {
    flex: none;
    color: var(--accent);
  }
</style>
