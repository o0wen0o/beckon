import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { ModelSelect } from "@/components/ModelSelect";
import { PaneHeader } from "@/components/PaneHeader";
import { Temperature } from "@/components/Temperature";
import { describeFailure } from "@/lib/failures";
import { modelOption, modelOptions, thinkingWarning, unknownModelHint } from "@/lib/models";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

const THINKING_HINT =
  "DeepSeek thinks by default. Leaving it on adds seconds of latency to translation-shaped Actions, which is why this is off unless you ask for it.";

const TEMPERATURE_HINT =
  "How freely the model words its answer. Low is literal and repeatable — the right end for translation or reformatting; high is varied, and drifts. 0 to 2.";

export function ModelDefaults() {
  const store = useStore(settings);
  const config = store.config;

  const modelHint = unknownModelHint(config?.defaults.model ?? null, store.models);
  const modelInfo = config
    ? (modelOption(config.defaults.model, store.models)?.description ?? "")
    : "";
  const thinkingHint = config
    ? thinkingWarning(config.defaults.model, config.defaults.thinking, store.models)
    : null;

  const catalog = store.models;
  const catalogNotice =
    !catalog || catalog.live || !catalog.fallback
      ? null
      : `${describeFailure(catalog.fallback, "The model list could not be fetched")} — showing the documented models.`;

  return (
    <>
      <PaneHeader title="Model defaults">
        What every Action inherits unless its own <code className="font-mono">[model]</code> table
        says otherwise.
      </PaneHeader>

      {catalogNotice ? (
        <Callout tone="warn">
          <p>{catalogNotice}</p>
        </Callout>
      ) : null}

      {config ? (
        <>
          <FieldGroup title="Model">
            <Field label="Model" hint={modelHint ? undefined : modelInfo} error={modelHint}>
              {({ id, describedBy }) => (
                <ModelSelect
                  id={id}
                  describedBy={describedBy}
                  value={config.defaults.model}
                  options={modelOptions(config.defaults.model, store.models)}
                  onChange={(model) =>
                    store.editConfig((draft) => (draft.defaults.model = model), true)
                  }
                />
              )}
            </Field>

            <Field label="Think before answering" warning={thinkingHint} hint={THINKING_HINT}>
              {({ id, describedBy }) => (
                <div className="flex items-center gap-2 self-start">
                  <Switch
                    id={id}
                    aria-describedby={describedBy}
                    aria-label="Think before answering"
                    checked={config.defaults.thinking}
                    onCheckedChange={(on) =>
                      store.editConfig((draft) => (draft.defaults.thinking = on), true)
                    }
                  />
                  <span
                    aria-hidden
                    className="text-muted-foreground min-w-5.5 text-left text-meta"
                  >
                    {config.defaults.thinking ? "On" : "Off"}
                  </span>
                </div>
              )}
            </Field>

            <Field label="Temperature" hint={TEMPERATURE_HINT}>
              {({ id, describedBy }) => (
                <Temperature
                  id={id}
                  describedBy={describedBy}
                  value={config.defaults.temperature}
                  onChange={(value) =>
                    store.editConfig((draft) => (draft.defaults.temperature = value))
                  }
                />
              )}
            </Field>
          </FieldGroup>

          <FieldGroup title="Catalog">
            <Field
              label="Model list"
              hint="Refreshing asks the endpoint for its own list; the documented catalog is the fallback."
            >
              {() => (
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    onClick={() => void store.refreshModels()}
                    disabled={store.modelsLoading}
                  >
                    {store.modelsLoading ? "Loading models…" : "Refresh models"}
                  </Button>
                  {store.models?.live ? (
                    <span className="text-muted-foreground text-meta">
                      Listed by the endpoint at your base URL.
                    </span>
                  ) : null}
                </div>
              )}
            </Field>
          </FieldGroup>
        </>
      ) : null}
    </>
  );
}
