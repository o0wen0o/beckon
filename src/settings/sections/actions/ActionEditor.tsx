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
import { OverrideField } from "@/components/OverrideField";
import { Segmented } from "@/components/Segmented";
import { Temperature } from "@/components/Temperature";
import { SOURCES as SOURCE_ORDER, sourceLabel } from "@/lib/inputSource";
import { modelOptions, unknownModelHint } from "@/lib/models";
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
  // without it the option list is rebuilt and re-scanned per character, while
  // the Model override row is still collapsed and showing none of it.
  const override = draft?.model.model ?? null;
  const catalog = settings.models;
  const modelOverrideOptions = React.useMemo(
    () => modelOptions(override ?? "", catalog),
    [override, catalog],
  );
  const modelHint = React.useMemo(() => unknownModelHint(override, catalog), [override, catalog]);

  if (!draft || !defaults) return null;

  const templateWarning =
    draft.prompt.user && !draft.prompt.user.includes("{{input}}")
      ? "This template never includes the input."
      : null;
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

      <FieldGroup title="Model overrides">
        {/* Indented past the label column and its gap — the ledger's own two
            tokens, added together by CSS rather than by hand, so the block
            follows `Field` if either ever moves. The rows then line up with the
            controls in the ledger above them rather than with their labels, and
            are held to the same measure as the widest control in it — otherwise the
            three of them are the only thing on the pane running past where every
            value stops. */}
        <div className="pt-3 pl-[calc(var(--spacing-ledger-label)+var(--spacing-ledger-gap))]">
          <div className="flex max-w-control-wide flex-col gap-2">
            <OverrideField
              label="Model"
              inherited={defaults.model}
              current={draft.model.model ?? defaults.model}
              overridden={draft.model.model !== null}
              error={modelHint}
              onOverride={(on) =>
                store.editDraft((next) => (next.model.model = on ? defaults.model : null), true)
              }
            >
              {/* No inherit option: inherit is the row's job, so "" here could only
              be a render artefact, and ModelSelect refuses to write it. */}
              <ModelSelect
                value={draft.model.model ?? ""}
                options={modelOverrideOptions}
                onChange={(model) => store.editDraft((next) => (next.model.model = model), true)}
              />
            </OverrideField>

            <OverrideField
              label="Thinking"
              inherited={defaults.thinking ? "on" : "off"}
              current={(draft.model.thinking ?? defaults.thinking) ? "on" : "off"}
              overridden={draft.model.thinking !== null}
              onOverride={(on) =>
                store.editDraft(
                  (next) => (next.model.thinking = on ? defaults.thinking : null),
                  true,
                )
              }
            >
              <OnOffSwitch
                label="Think before answering"
                checked={draft.model.thinking ?? defaults.thinking}
                onChange={(value) => store.editDraft((next) => (next.model.thinking = value), true)}
              />
            </OverrideField>

            <OverrideField
              label="Temperature"
              hint={TEMPERATURE_HINT}
              inherited={String(defaults.temperature)}
              current={String(draft.model.temperature ?? defaults.temperature)}
              overridden={draft.model.temperature !== null}
              onOverride={(on) =>
                store.editDraft(
                  (next) => (next.model.temperature = on ? defaults.temperature : null),
                  true,
                )
              }
            >
              <Temperature
                value={draft.model.temperature ?? defaults.temperature}
                onChange={(value) => store.editDraft((next) => (next.model.temperature = value))}
              />
            </OverrideField>
          </div>
        </div>
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
