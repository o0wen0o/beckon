import * as React from "react";
import { Button } from "@/components/ui/button";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { ModelSelect } from "@/components/ModelSelect";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { PaneHeader } from "@/components/PaneHeader";
import { describeFailure } from "@/lib/failures";
import { fill, useT } from "@/lib/i18n";
import { modelOptions, thinkingWarning, unknownModelHint } from "@/lib/models";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

export function ModelDefaults() {
  const t = useT();
  const store = useStore(settings);
  const config = store.config;

  const catalog = store.models;
  const model = config?.defaults.model ?? null;

  // Memoised for identity, not for the list-building: `modelOptions` allocates a
  // fresh array for a configured-but-unknown model, and `ModelSelect` memoises
  // its own filtering on `[options]`, so a new identity here re-filters there.
  // The two hint helpers below still rebuild the list internally — cheap enough
  // to leave alone, and passing it in would be a signature change in lib/models.
  const options = React.useMemo(() => modelOptions(model ?? "", catalog), [model, catalog]);
  const modelHint = unknownModelHint(model, catalog, t);
  const modelInfo = options.find((option) => option.id === model)?.description ?? "";
  const thinkingHint = config
    ? thinkingWarning(config.defaults.model, config.defaults.thinking, catalog, t)
    : null;

  const catalogNotice =
    !catalog || catalog.live || !catalog.fallback
      ? null
      : t.settings.defaults.catalogNotice(
          describeFailure(catalog.fallback, t, t.settings.defaults.catalogFallback),
        );

  return (
    <>
      <PaneHeader title={t.settings.defaults.title}>
        {fill(t.settings.defaults.lede, {
          table: <code className="font-mono">[model]</code>,
        })}
      </PaneHeader>

      {catalogNotice ? (
        <Callout tone="warn">
          <p>{catalogNotice}</p>
        </Callout>
      ) : null}

      {config ? (
        <>
          <FieldGroup title={t.settings.defaults.model}>
            <Field
              label={t.settings.defaults.model}
              hint={modelHint ? undefined : modelInfo}
              error={modelHint}
            >
              {({ id, describedBy }) => (
                <ModelSelect
                  id={id}
                  describedBy={describedBy}
                  value={config.defaults.model}
                  options={options}
                  onChange={(model) =>
                    store.editConfig((draft) => (draft.defaults.model = model), true)
                  }
                />
              )}
            </Field>

            <Field
              label={t.settings.defaults.thinking}
              warning={thinkingHint}
              hint={t.settings.defaults.thinkingHint}
            >
              {({ id, describedBy }) => (
                <OnOffSwitch
                  id={id}
                  describedBy={describedBy}
                  label={t.settings.defaults.thinking}
                  checked={config.defaults.thinking}
                  onChange={(on) =>
                    store.editConfig((draft) => (draft.defaults.thinking = on), true)
                  }
                />
              )}
            </Field>
          </FieldGroup>

          <FieldGroup title={t.settings.defaults.catalog}>
            <Field
              label={t.settings.defaults.modelList}
              hint={t.settings.defaults.modelListHint}
            >
              {() => (
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm-note"
                    onClick={() => void store.refreshModels()}
                    disabled={store.modelsLoading}
                  >
                    {store.modelsLoading
                      ? t.settings.defaults.loading
                      : t.settings.defaults.refresh}
                  </Button>
                  {store.models?.live ? (
                    <span className="text-muted-foreground text-meta">
                      {t.settings.defaults.live}
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
