<script lang="ts">
  import Callout from "../../lib/ui/Callout.svelte";
  import Field from "../../lib/ui/Field.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import HotkeyInput from "../../lib/ui/HotkeyInput.svelte";
  import { settings } from "../store.svelte";

  const config = $derived(settings.config);

  function setLauncherHotkey(accelerator: string | null) {
    if (!accelerator) return;
    settings.editConfig((draft) => (draft.launcher_hotkey = accelerator), true);
  }
</script>

<h1>Triggering</h1>

{#if settings.startupErrors.length > 0}
  <Callout tone="danger">
    <p><strong>A hotkey is not active.</strong></p>
    <ul>
      {#each settings.startupErrors as error (error)}<li>{error}</li>{/each}
    </ul>
    <p>Record a different combination below; it is registered the moment you record it.</p>
  </Callout>
{/if}

{#if config}
  <Field
    label="Launcher hotkey"
    hint="Recorded hotkeys are registered immediately — if the combination is taken it goes red and is not saved."
  >
    {#snippet control()}
      <HotkeyInput value={config.launcher_hotkey} onchange={setLauncherHotkey} />
    {/snippet}
  </Field>

  <Field label="Autostart" hint="Beckon lives in the tray; starting with Windows is the point.">
    {#snippet control({ id, describedBy })}
      <Toggle
        {id}
        {describedBy}
        label="Start with Windows"
        checked={config.autostart}
        onchange={(on) => settings.editConfig((draft) => (draft.autostart = on), true)}
      />
    {/snippet}
  </Field>
{/if}
