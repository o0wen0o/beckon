<script lang="ts">
  // A native select, deliberately. A hand-rolled listbox could show the
  // per-model description inline, but this is the one surface whose entire
  // hazard is focus and re-render, and a native control gets keyboard, IME and
  // edge-of-window popup placement for free.
  import type { ModelOption } from "../types";

  interface Props {
    /** "" means inherit, and is only legitimate when `inheritLabel` is given. */
    value: string;
    options: ModelOption[];
    inheritLabel?: string;
    id?: string;
    describedBy?: string;
    onchange: (id: string) => void;
  }

  let { value, options, inheritLabel, id, describedBy, onchange }: Props = $props();

  const known = $derived(options.filter((option) => option.origin !== "configured"));
  const configured = $derived(options.filter((option) => option.origin === "configured"));

  /**
   * `value=` + `onchange`, never `bind:`. Binding would write back whatever the
   * select settled on before the catalog arrived, which is exactly how a
   * configured model gets silently replaced.
   *
   * The guard is the other half of that rule: without an inherit option there
   * is no legitimate "", so an empty value can only be a render artefact from a
   * select momentarily holding a value not in its own list — and writing it
   * would blank the configured model.
   */
  function choose(next: string) {
    if (next === "" && inheritLabel === undefined) return;
    onchange(next);
  }
</script>

<select {id} {value} aria-describedby={describedBy} onchange={(e) => choose(e.currentTarget.value)}>
  {#if inheritLabel !== undefined}
    <option value="">{inheritLabel}</option>
  {/if}
  {#each known as option (option.id)}
    <option value={option.id}>{option.label}</option>
  {/each}
  {#if configured.length > 0}
    <!-- Quarantined rather than mixed in: nothing vouches for these but the
         configuration file that names them. -->
    <optgroup label="Named by your configuration">
      {#each configured as option (option.id)}
        <option value={option.id}>{option.label}</option>
      {/each}
    </optgroup>
  {/if}
</select>
