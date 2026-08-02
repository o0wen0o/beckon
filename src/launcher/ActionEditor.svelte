<script lang="ts">
  // The Action form, hosted by the Launcher. There is no Save button and there
  // must never be one (ADR-0003): every field commits to disk, debounced, and
  // the `actions-changed` echo re-renders the list behind it.
  import Callout from "../lib/ui/Callout.svelte";
  import Field from "../lib/ui/Field.svelte";
  import HotkeyInput from "../lib/ui/HotkeyInput.svelte";
  import ModelSelect from "../lib/ui/ModelSelect.svelte";
  import OverrideField from "../lib/ui/OverrideField.svelte";
  import Segmented from "../lib/ui/Segmented.svelte";
  import Temperature from "../lib/ui/Temperature.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import { modelOptions, unknownModelHint } from "../lib/models";
  import type { Action, InputSource } from "../lib/types";
  import { actions } from "./actions.svelte";

  interface Props {
    /** The snapshot's copy — for identity and snapshot-derived errors only.
     *  Field values come from `actions.draft`, never from here. */
    action: Action;
  }

  let { action }: Props = $props();

  const draft = $derived(actions.draft);
  const defaults = $derived(actions.config?.defaults);
  const hotkeyConflict = $derived(actions.snapshot.hotkey_errors[action.id]);

  const SOURCES: { value: InputSource; label: string }[] = [
    { value: "selection", label: "Selection" },
    { value: "prompt", label: "Prompt" },
    { value: "auto", label: "Auto" },
  ];

  const SOURCE_HINT: Record<InputSource, string> = {
    selection: "Uses the Selection only. An empty grab shows a hint and sends nothing.",
    prompt: "Uses typed input only. Any Selection is ignored.",
    auto: "Uses the Selection if there is one, otherwise asks for typed input.",
  };

  const TEMPERATURE_HINT =
    "How freely the model words its answer. Low is literal and repeatable — the right end for translation or reformatting; high is varied, and drifts. 0 to 2.";

  const modelHint = $derived(unknownModelHint(draft?.model.model ?? null, actions.models));
  const templateWarning = $derived(
    draft?.prompt.user && !draft.prompt.user.includes("{{input}}")
      ? "This template never includes the input."
      : null,
  );
  const nameWarning = $derived(
    draft && draft.name.trim() === ""
      ? "Without a name this Action shows as its file name in the Launcher."
      : null,
  );
</script>

{#if draft && defaults}
  {#if hotkeyConflict}
    <!-- `save_action` re-probes the Direct Hotkey and refuses the whole write
         when it cannot be registered, so while this is true not even renaming
         the Action can be saved. Say so, and offer the way out. -->
    <Callout tone="danger">
      <p><strong>This Action's Direct Hotkey is not active.</strong> {hotkeyConflict}</p>
      <p>No change to this Action can be saved until the hotkey is cleared or changed.</p>
      <p>
        <button onclick={() => actions.editDraft((next) => (next.hotkey = null), true)}>
          Clear the Direct Hotkey
        </button>
      </p>
    </Callout>
  {/if}

  <div class="grid">
    <Field label="Name" warning={nameWarning}>
      {#snippet control({ id, describedBy })}
        <input
          {id}
          aria-describedby={describedBy}
          value={draft.name}
          oninput={(event) => {
            const value = event.currentTarget.value;
            actions.editDraft((next) => (next.name = value));
          }}
        />
      {/snippet}
    </Field>

    <Field label="Description" hint="Shown under the name in the Launcher, and searched.">
      {#snippet control({ id, describedBy })}
        <input
          {id}
          aria-describedby={describedBy}
          value={draft.description ?? ""}
          oninput={(event) => {
            const value = event.currentTarget.value || null;
            actions.editDraft((next) => (next.description = value));
          }}
        />
      {/snippet}
    </Field>
  </div>

  <div class="grid">
    <Field label="Input Source" hint={SOURCE_HINT[draft.input_source]}>
      {#snippet control({ id, describedBy })}
        <Segmented
          {id}
          {describedBy}
          label="Input Source"
          value={draft.input_source}
          options={SOURCES}
          onchange={(source) => actions.editDraft((next) => (next.input_source = source), true)}
        />
      {/snippet}
    </Field>

    <Field label="Direct Hotkey" hint="Optional. Without one, the Action is Launcher-only.">
      {#snippet control()}
        <HotkeyInput
          value={draft.hotkey ?? null}
          clearable
          onchange={(accelerator) => actions.editDraft((next) => (next.hotkey = accelerator), true)}
        />
      {/snippet}
    </Field>
  </div>

  <Field label="System prompt" hint="How the model should behave. Sent ahead of every input.">
    {#snippet control({ id, describedBy })}
      <textarea
        {id}
        aria-describedby={describedBy}
        class="system"
        value={draft.prompt.system}
        oninput={(event) => {
          const value = event.currentTarget.value;
          actions.editDraft((next) => (next.prompt.system = value));
        }}
      ></textarea>
    {/snippet}
  </Field>

  <Field
    label="User template"
    warning={templateWarning}
    hint="{'{{input}}'} is replaced by the Selection or the typed input. Empty means just the input."
  >
    {#snippet control({ id, describedBy })}
      <input
        {id}
        aria-describedby={describedBy}
        class="mono"
        value={draft.prompt.user ?? ""}
        placeholder={"{{input}}"}
        oninput={(event) => {
          const value = event.currentTarget.value || null;
          actions.editDraft((next) => (next.prompt.user = value));
        }}
      />
    {/snippet}
  </Field>

  <h2>Model overrides</h2>

  <div class="overrides">
    <OverrideField
      label="Model"
      inherited={defaults.model}
      current={draft.model.model ?? defaults.model}
      overridden={draft.model.model !== null}
      error={modelHint}
      onoverride={(on) =>
        actions.editDraft((next) => (next.model.model = on ? defaults.model : null), true)}
    >
      {#snippet control()}
        <!-- No inherit option: inherit is the row's job, so "" here could only
             be a render artefact, and ModelSelect refuses to write it. -->
        <ModelSelect
          value={draft.model.model ?? ""}
          options={modelOptions(draft.model.model ?? "", actions.models)}
          onchange={(model) => actions.editDraft((next) => (next.model.model = model), true)}
        />
      {/snippet}
    </OverrideField>

    <OverrideField
      label="Thinking"
      inherited={defaults.thinking ? "on" : "off"}
      current={(draft.model.thinking ?? defaults.thinking) ? "on" : "off"}
      overridden={draft.model.thinking !== null}
      onoverride={(on) =>
        actions.editDraft((next) => (next.model.thinking = on ? defaults.thinking : null), true)}
    >
      {#snippet control()}
        <Toggle
          label="Think before answering"
          checked={draft.model.thinking ?? defaults.thinking}
          onchange={(value) => actions.editDraft((next) => (next.model.thinking = value), true)}
        />
      {/snippet}
    </OverrideField>

    <OverrideField
      label="Temperature"
      hint={TEMPERATURE_HINT}
      inherited={String(defaults.temperature)}
      current={String(draft.model.temperature ?? defaults.temperature)}
      overridden={draft.model.temperature !== null}
      onoverride={(on) =>
        actions.editDraft(
          (next) => (next.model.temperature = on ? defaults.temperature : null),
          true,
        )}
    >
      {#snippet control()}
        <Temperature
          value={draft.model.temperature ?? defaults.temperature}
          onchange={(value) => actions.editDraft((next) => (next.model.temperature = value))}
        />
      {/snippet}
    </OverrideField>
  </div>

  <div class="footer-row">
    <button class="danger" onclick={() => (actions.pendingDelete = action)}>Delete Action</button>
  </div>
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 0 var(--space-4);
  }

  .system {
    min-height: 120px;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  h2 {
    margin: var(--space-2) 0 var(--space-3);
    font-family: var(--font-small);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .overrides {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }

  .footer-row {
    display: flex;
    justify-content: flex-end;
  }
</style>
