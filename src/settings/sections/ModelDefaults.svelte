<script lang="ts">
  import Callout from "../../lib/ui/Callout.svelte";
  import Field from "../../lib/ui/Field.svelte";
  import ModelSelect from "../../lib/ui/ModelSelect.svelte";
  import Temperature from "../../lib/ui/Temperature.svelte";
  import Toggle from "../../lib/ui/Toggle.svelte";
  import { describeFailure } from "../../lib/failures";
  import { modelOption, modelOptions, thinkingWarning, unknownModelHint } from "../../lib/models";
  import { settings } from "../store.svelte";

  const config = $derived(settings.config);

  const modelHint = $derived(unknownModelHint(config?.defaults.model ?? null, settings.models));
  const modelInfo = $derived(
    config ? (modelOption(config.defaults.model, settings.models)?.description ?? "") : "",
  );
  const thinkingHint = $derived(
    config ? thinkingWarning(config.defaults.model, config.defaults.thinking, settings.models) : null,
  );

  const catalogNotice = $derived.by(() => {
    const catalog = settings.models;
    if (!catalog || catalog.live || !catalog.fallback) return null;
    return `${describeFailure(catalog.fallback, "The model list could not be fetched")} — showing the documented models.`;
  });
</script>

<h1>Model defaults</h1>

{#if catalogNotice}
  <Callout tone="warn"><p>{catalogNotice}</p></Callout>
{/if}

{#if config}
  <p class="lead">Any Action can override these in its own <code>[model]</code> table.</p>

  <Field label="Model" hint={modelHint ? undefined : modelInfo} error={modelHint}>
    {#snippet control({ id, describedBy })}
      <ModelSelect
        {id}
        {describedBy}
        value={config.defaults.model}
        options={modelOptions(config.defaults.model, settings.models)}
        onchange={(model) => settings.editConfig((draft) => (draft.defaults.model = model), true)}
      />
    {/snippet}
  </Field>

  <Field
    label="Think before answering"
    warning={thinkingHint}
    hint="DeepSeek thinks by default. Leaving it on adds seconds of latency to translation-shaped Actions, which is why this is off unless you ask for it."
  >
    {#snippet control({ id, describedBy })}
      <Toggle
        {id}
        {describedBy}
        label="Think before answering"
        showState
        checked={config.defaults.thinking}
        onchange={(on) => settings.editConfig((draft) => (draft.defaults.thinking = on), true)}
      />
    {/snippet}
  </Field>

  <Field
    label="Temperature"
    hint="How freely the model words its answer. Low is literal and repeatable — the right end for translation or reformatting; high is varied, and drifts. 0 to 2."
  >
    {#snippet control({ id, describedBy })}
      <Temperature
        {id}
        {describedBy}
        value={config.defaults.temperature}
        onchange={(value) => settings.editConfig((draft) => (draft.defaults.temperature = value))}
      />
    {/snippet}
  </Field>

  <div class="row">
    <button onclick={() => settings.refreshModels()} disabled={settings.modelsLoading}>
      {settings.modelsLoading ? "Loading models…" : "Refresh models"}
    </button>
    {#if settings.models?.live}
      <span class="hint">Listed by the endpoint at your base URL.</span>
    {/if}
  </div>
{/if}

<style>
  .lead {
    margin: 0 0 var(--space-4);
    font-family: var(--font-small);
    font-size: var(--text-sm);
    color: var(--text-dim);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
</style>
