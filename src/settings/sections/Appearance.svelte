<script lang="ts">
  import Field from "../../lib/ui/Field.svelte";
  import Segmented from "../../lib/ui/Segmented.svelte";
  import type { Theme } from "../../lib/types";
  import { settings } from "../store.svelte";

  const config = $derived(settings.config);

  const THEMES: { value: Theme; label: string }[] = [
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
    { value: "system", label: "Follow Windows" },
  ];
</script>

<h1>Appearance</h1>

{#if config}
  <Field
    label="Theme"
    hint="Applies to the Launcher, the Popover and this window at once. Beckon starts light unless you say otherwise — “Follow Windows” is the only setting that reads the system preference."
  >
    {#snippet control({ id, describedBy })}
      <Segmented
        {id}
        {describedBy}
        label="Theme"
        value={config.theme}
        options={THEMES}
        onchange={(theme) => settings.editConfig((draft) => (draft.theme = theme), true)}
      />
    {/snippet}
  </Field>
{/if}
