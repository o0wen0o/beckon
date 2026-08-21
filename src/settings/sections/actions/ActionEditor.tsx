// The Action form. There is no Save button and there must never be one
// (ADR-0003): every field commits to disk, debounced, and the `actions-changed`
// echo re-renders the list behind it.
//
// The defaults an override inherits from and the model catalog come from the
// window's other store — this window already loads both for Model defaults, and
// a second copy could only drift from it.
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { HotkeyInput } from "@/components/HotkeyInput";
import { ModelSelect } from "@/components/ModelSelect";
import { OverrideField } from "@/components/OverrideField";
import { Segmented } from "@/components/Segmented";
import { Temperature } from "@/components/Temperature";
import { modelOptions, unknownModelHint } from "@/lib/models";
import type { Action, InputSource } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { settings } from "../../store";

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

  if (!draft || !defaults) return null;

  const modelHint = unknownModelHint(draft.model.model ?? null, settings.models);
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

      <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-x-4">
        <Field label="Name" warning={nameWarning}>
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

        {/* Right-hand column: the hint bubble hangs leftwards, or its box
            overflows the pane and leaves a horizontal scrollbar behind. */}
        <Field
          label="Description"
          hint="Shown under the name in the Launcher, and searched."
          hintAlign="end"
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
      </div>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(220px,1fr))] gap-x-4">
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

        <Field
          label="Direct Hotkey"
          hint="Optional. Without one, the Action is Launcher-only."
          hintAlign="end"
        >
          {() => (
            <HotkeyInput
              value={draft.hotkey ?? null}
              clearable
              onChange={(accelerator) => store.editDraft((next) => (next.hotkey = accelerator), true)}
            />
          )}
        </Field>
      </div>

      <Field label="System prompt" hint="How the model should behave. Sent ahead of every input.">
        {({ id, describedBy }) => (
          <Textarea
            id={id}
            aria-describedby={describedBy}
            className="font-mono min-h-30 text-xs"
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
        warning={templateWarning}
        hint="{{input}} is replaced by the Selection or the typed input. Empty means just the input."
      >
        {({ id, describedBy }) => (
          <Input
            id={id}
            aria-describedby={describedBy}
            className="font-mono text-xs"
            value={draft.prompt.user ?? ""}
            placeholder="{{input}}"
            onChange={(event) => {
              const value = event.currentTarget.value || null;
              store.editDraft((next) => (next.prompt.user = value));
            }}
          />
        )}
      </Field>

      <h2 className="font-small text-muted-foreground mt-6 mb-3 text-2xs font-semibold tracking-widest uppercase">
        Model overrides
      </h2>

      <div className="mb-5 flex flex-col gap-2">
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
            options={modelOptions(draft.model.model ?? "", settings.models)}
            onChange={(model) => store.editDraft((next) => (next.model.model = model), true)}
          />
        </OverrideField>

        <OverrideField
          label="Thinking"
          inherited={defaults.thinking ? "on" : "off"}
          current={(draft.model.thinking ?? defaults.thinking) ? "on" : "off"}
          overridden={draft.model.thinking !== null}
          onOverride={(on) =>
            store.editDraft((next) => (next.model.thinking = on ? defaults.thinking : null), true)
          }
        >
          <div className="flex items-center gap-2">
            <Switch
              aria-label="Think before answering"
              checked={draft.model.thinking ?? defaults.thinking}
              onCheckedChange={(value) =>
                store.editDraft((next) => (next.model.thinking = value), true)
              }
            />
            <span aria-hidden className="text-muted-foreground font-small min-w-9 text-xs">
              {(draft.model.thinking ?? defaults.thinking) ? "On" : "Off"}
            </span>
          </div>
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

      {/* Above a divider rather than floating at the end of the form: it deletes
          a file, so it must not read as the last field's neighbour. */}
      <div className="flex items-center justify-between gap-3 border-t pt-4">
        <span className="text-muted-foreground font-small text-xs">
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
