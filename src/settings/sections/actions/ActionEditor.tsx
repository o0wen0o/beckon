// The Action form. There is no Save button and there must never be one
// (ADR-0003): every field commits to disk, debounced, and the `actions-changed`
// echo re-renders the list behind it.
//
// The defaults an override inherits from and the model catalog come from the
// window's other store — this window already loads both for Model defaults, and
// a second copy could only drift from it.
import * as React from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { ModelSelect } from "@/components/ModelSelect";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { Segmented } from "@/components/Segmented";
import { Temperature } from "@/components/Temperature";
import { SOURCES as SOURCE_ORDER, sourceLabel } from "@/lib/inputSource";
import { modelOptions, thinkingWarning, unknownModelHint } from "@/lib/models";
import type { Action, InputSource } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { settings } from "../../store";

/** Derived rather than restated: this is the one place an Input Source is
 *  chosen, so a third hand-written copy of the three values and their labels is
 *  the copy most likely to disagree with the two lists that display them. */
const SOURCES = SOURCE_ORDER.map((value) => ({ value, label: sourceLabel(value) }));

const SOURCE_HINT: Record<InputSource, string> = {
  selection: "Uses the Selection only. An empty grab shows a hint and sends nothing.",
  prompt: "Uses typed input only. Any Selection is ignored.",
  auto: "Uses the Selection if there is one, otherwise asks for typed input.",
};

const TEMPERATURE_HINT =
  "How freely the model words its answer. Low is literal and repeatable — the right end for translation or reformatting; high is varied, and drifts. 0 to 2.";

const THINKING_HINT =
  "Adds seconds before the first word. Worth it where the Action needs the model to reason, not where it reformats.";

interface ActionEditorProps {
  /** The snapshot's copy — for identity and snapshot-derived errors only. Field
   *  values come from `actionStore.draft`, never from here. */
  action: Action;
}

export function ActionEditor({ action }: ActionEditorProps) {
  const store = useStore(actionStore);
  const config = useStore(settings).config;

  const draft = store.draft;
  const defaults = config?.defaults;
  const hotkeyConflict = store.snapshot.hotkey_errors[action.id];

  // Above the guard, because hooks have to be, and memoized because every
  // keystroke in Name, Description or either prompt field re-renders this form:
  // without it the option list is rebuilt and re-scanned per character.
  //
  // Derived from the *effective* model rather than from the override: the row
  // shows what a request would carry, so while the key is absent the select
  // holds the inherited value — and a select whose value is missing from its own
  // options silently rewrites it.
  const effectiveModel = draft?.model.model ?? config?.defaults.model ?? "";
  const catalog = settings.models;
  const modelOverrideOptions = React.useMemo(
    () => modelOptions(effectiveModel, catalog),
    [effectiveModel, catalog],
  );
  const modelHint = React.useMemo(
    () => unknownModelHint(effectiveModel || null, catalog),
    [effectiveModel, catalog],
  );

  if (!draft || !defaults) return null;

  const templateWarning =
    draft.prompt.user && !draft.prompt.user.includes("{{input}}")
      ? "This template never includes the input."
      : null;
  // The same two lines Model defaults draws under its own Model row: what the
  // model is, and a `thinking` setting it cannot honour. Both read the effective
  // values, since those are what a request would carry.
  const modelInfo =
    modelOverrideOptions.find((option) => option.id === effectiveModel)?.description ?? "";
  const effectiveThinking = draft.model.thinking ?? defaults.thinking;
  const thinkingHint = thinkingWarning(effectiveModel, effectiveThinking, catalog);
  const nameWarning =
    draft.name.trim() === ""
      ? "Without a name this Action shows as its file name in the Launcher."
      : null;

  return (
    <>
      {hotkeyConflict ? (
        // `save_action` re-probes the Direct Hotkey and refuses the whole write
        // when it cannot be registered, so while this is true not even renaming
        // the Action can be saved. Say so, and offer the way out.
        <Callout tone="danger">
          <p>
            <strong>This Action&apos;s Direct Hotkey is not active.</strong> {hotkeyConflict}
          </p>
          <p>No change to this Action can be saved until the hotkey is cleared or changed.</p>
          <p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => store.editDraft((next) => (next.hotkey = null), true)}
            >
              Clear the Direct Hotkey
            </Button>
          </p>
        </Callout>
      ) : null}

      <FieldGroup title="Action">
        <Field label="Name" measure="field" warning={nameWarning}>
          {({ id, describedBy }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              value={draft.name}
              onChange={(event) => {
                const value = event.currentTarget.value;
                store.editDraft((next) => (next.name = value));
              }}
            />
          )}
        </Field>

        <Field
          label="Description"
          measure="field"
          hint="Shown under the name in the Launcher, and searched."
        >
          {({ id, describedBy }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              value={draft.description ?? ""}
              onChange={(event) => {
                const value = event.currentTarget.value || null;
                store.editDraft((next) => (next.description = value));
              }}
            />
          )}
        </Field>
        <Field label="Input Source" hint={SOURCE_HINT[draft.input_source]}>
          {({ id, describedBy }) => (
            <Segmented
              id={id}
              describedBy={describedBy}
              label="Input Source"
              value={draft.input_source}
              options={SOURCES}
              onChange={(source) => store.editDraft((next) => (next.input_source = source), true)}
            />
          )}
        </Field>

        <Field label="Direct Hotkey" hint="Optional. Without one, the Action is Launcher-only.">
          {() => (
            <HotkeyInput
              value={draft.hotkey ?? null}
              clearable
              onChange={(accelerator) =>
                store.editDraft((next) => (next.hotkey = accelerator), true)
              }
            />
          )}
        </Field>
      </FieldGroup>

      <FieldGroup title="Prompt">
        <Field
          label="System prompt"
          measure="wide"
          hint="How the model should behave. Sent ahead of every input."
        >
          {({ id, describedBy }) => (
            <Textarea
              id={id}
              aria-describedby={describedBy}
              className="font-mono min-h-30 text-quiet"
              value={draft.prompt.system}
              onChange={(event) => {
                const value = event.currentTarget.value;
                store.editDraft((next) => (next.prompt.system = value));
              }}
            />
          )}
        </Field>

        <Field
          label="User template"
          measure="wide"
          warning={templateWarning}
          hint="{{input}} is replaced by the Selection or the typed input. Empty means just the input."
        >
          {({ id, describedBy }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              className="font-mono text-quiet"
              value={draft.prompt.user ?? ""}
              placeholder="{{input}}"
              onChange={(event) => {
                const value = event.currentTarget.value || null;
                store.editDraft((next) => (next.prompt.user = value));
              }}
            />
          )}
        </Field>
      </FieldGroup>

      {/* Every `[model]` key is optional, and absent means "inherit Model
          defaults". The control is live either way and shows the effective
          value, so touching it *is* the override — one gesture for a select, a
          switch and a slider alike. What says which side of the default a row is
          on is `Field`'s `override`: a dot in the label's gutter and a revert
          control, on an overridden row only, with the head's note covering the
          rest. This was three bordered boxes indented into the value column, and
          the only thing on the pane that was not a ledger row (ADR-0011). */}
      <FieldGroup title="Model overrides" note="Unmarked rows follow Model defaults">
        <Field
          label="Model"
          hint={modelHint ? undefined : modelInfo}
          error={modelHint}
          override={{
            overridden: draft.model.model !== null,
            defaultReading: defaults.model,
            onRevert: () => store.editDraft((next) => (next.model.model = null), true),
          }}
        >
          {({ id, describedBy }) => (
            // No inherit option: an absent key is what inheriting means and the
            // revert control is the way back, so "" here could only be a render
            // artefact — which `ModelSelect` refuses to write.
            <ModelSelect
              id={id}
              describedBy={describedBy}
              value={effectiveModel}
              options={modelOverrideOptions}
              onChange={(model) => store.editDraft((next) => (next.model.model = model), true)}
            />
          )}
        </Field>

        <Field
          label="Think before answering"
          warning={thinkingHint}
          hint={THINKING_HINT}
          override={{
            overridden: draft.model.thinking !== null,
            defaultReading: defaults.thinking ? "on" : "off",
            onRevert: () => store.editDraft((next) => (next.model.thinking = null), true),
          }}
        >
          {({ id, describedBy }) => (
            <OnOffSwitch
              id={id}
              describedBy={describedBy}
              label="Think before answering"
              checked={effectiveThinking}
              onChange={(value) => store.editDraft((next) => (next.model.thinking = value), true)}
            />
          )}
        </Field>

        <Field
          label="Temperature"
          hint={TEMPERATURE_HINT}
          override={{
            overridden: draft.model.temperature !== null,
            defaultReading: String(defaults.temperature),
            onRevert: () => store.editDraft((next) => (next.model.temperature = null), true),
          }}
        >
          {({ id, describedBy }) => (
            <Temperature
              id={id}
              describedBy={describedBy}
              value={draft.model.temperature ?? defaults.temperature}
              onChange={(value) => store.editDraft((next) => (next.model.temperature = value))}
            />
          )}
        </Field>
      </FieldGroup>

      {/* Above a divider rather than floating at the end of the form: it deletes
          a file, so it must not read as the last field's neighbour. */}
      <div className="flex items-center justify-between gap-3 border-t pt-4">
        <span className="text-muted-foreground text-meta">
          Deleting removes <code className="font-mono">{action.file_name}</code> from disk.
        </span>
        {/* Destructive up front, not only once the pointer is over it: hover is
            not a state a keyboard user passes through. The outline carries that
            at rest; solid red is the confirmation dialog's. */}
        <Button
          variant="destructive-outline"
          className="flex-none"
          onClick={() => store.askDelete(action)}
        >
          Delete Action
        </Button>
      </div>
    </>
  );
}
