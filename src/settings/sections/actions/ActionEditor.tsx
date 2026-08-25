// The Action form. There is no Save button and there must never be one
// (ADR-0003): every field commits to disk, debounced, and the `actions-changed`
// echo re-renders the list behind it.
//
// The endpoint an override inherits from, and that endpoint's model catalog,
// come from the window's other store — the Connection pane already loads both,
// and a second copy could only drift from it.
//
// The `Overrides` group is where ADR-0021's design question landed. Three rows,
// each either inheriting or overriding, and changing the first changes what the
// other two inherit: `provider` resolves to a row, and that row is what `model`
// and `thinking` fall back to. Nothing new was built for it — `Field`'s own
// `override` prop already draws exactly this, and `provider` is a third row in a
// group made for it.
import * as React from "react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { HotkeyInput } from "@/components/HotkeyInput";
import { ModelSelect } from "@/components/ModelSelect";
import { NavCard } from "@/components/NavCard";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { Segmented } from "@/components/Segmented";
import { useT } from "@/lib/i18n";
import { SOURCES as SOURCE_ORDER, sourceLabel } from "@/lib/inputSource";
import {
  canWebSearch,
  modelOptions,
  thinkingWarning,
  unknownModelHint,
  webSearchState,
} from "@/lib/models";
import { chatUrl, isLocal, keyProblem, strandedModel } from "@/lib/providers";
import type { Action } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";
import { settings } from "../../store";

interface ActionEditorProps {
  /** The snapshot's copy — for identity and snapshot-derived errors only. Field
   *  values come from `actionStore.draft`, never from here. */
  action: Action;
}

export function ActionEditor({ action }: ActionEditorProps) {
  const t = useT();
  const store = useStore(actionStore);
  const config = useStore(settings).config;

  /** Derived rather than restated: this is the one place an Input Source is
   *  chosen, so a third hand-written copy of the two values and their labels
   *  is the copy most likely to disagree with the two lists that display them. */
  const sources = SOURCE_ORDER.map((value) => ({ value, label: sourceLabel(value, t) }));

  const draft = store.draft;
  const hotkeyConflict = store.snapshot.hotkey_errors[action.id];

  /** The row this Action's request would go to, and the row it would go to if
   *  the `provider` override were reverted. The second is what the revert
   *  control names, so both are needed even when they are the same. */
  const fallback = settings.defaultProvider;
  const provider = settings.provider(draft?.model.provider);

  // Above the guard, because hooks have to be, and memoized because every
  // keystroke in Name, Description or either prompt field re-renders this form:
  // without it the option list is rebuilt and re-scanned per character.
  //
  // Derived from the *effective* model rather than from the override: the row
  // shows what a request would carry, so while the key is absent the select
  // holds the inherited value — and a select whose value is missing from its own
  // options silently rewrites it.
  const effectiveModel = draft?.model.model ?? provider?.model ?? "";
  // This endpoint's catalog, not "the" catalog: there is one list per row
  // (ADR-0021). It is keyed on the resolved provider, so overriding the row
  // above re-reads the list here.
  const catalog = provider ? (settings.models[provider.id] ?? null) : null;
  const modelOverrideOptions = React.useMemo(
    () => modelOptions(effectiveModel, catalog),
    [effectiveModel, catalog],
  );
  const modelHint = React.useMemo(
    () => unknownModelHint(effectiveModel || null, catalog, t),
    [effectiveModel, catalog, t],
  );

  // The endpoint this Action would post to is the one whose list is about to be
  // read, so its live fetch is paid for here: a reveal only primes every row's
  // offline answer (`settings.refreshAll`). Keyed on the resolved id, so
  // overriding the row above fetches the new one's list.
  const resolvedProvider = provider?.id;
  React.useEffect(() => {
    if (resolvedProvider !== undefined) void settings.refreshModels(resolvedProvider);
  }, [resolvedProvider]);

  if (!draft || !config) return null;

  // The Definition screen's own warnings, carried onto the card that opens it:
  // one at a time, because that screen shows every one of them in full. A
  // field's problem must survive being one click away.
  const definitionWarning =
    draft.name.trim() === ""
      ? t.settings.actions.nameWarning
      : draft.prompt.user && !draft.prompt.user.includes("{{input}}")
        ? t.settings.actions.templateWarningShort
        : null;
  // The same two lines Model defaults draws under its own Model row: what the
  // model is, and a `thinking` setting it cannot honour. Both read the effective
  // values, since those are what a request would carry.
  const modelInfo =
    modelOverrideOptions.find((option) => option.id === effectiveModel)?.description ?? "";
  const effectiveThinking = draft.model.thinking ?? provider?.thinking ?? false;
  const thinkingHint = thinkingWarning(provider, effectiveModel, effectiveThinking, catalog, t);
  // The same shape one row down (ADR-0026), and since ADR-0027 it reads the
  // model too: an endpoint with no search field, or a model the vendor says
  // does not take that field, greys the switch instead of accepting a `true`
  // that reaches nothing. Both stay amber rather than errors — a search that
  // cannot be expressed costs the feature, not the turn.
  const effectiveWebSearch = draft.model.web_search ?? provider?.web_search ?? false;
  const { warning: webSearchHint, offOnly: webSearchOffOnly } = webSearchState(
    provider,
    effectiveModel,
    effectiveWebSearch,
    catalog,
    t,
  );
  /**
   * A model this Action pinned that its own endpoint does not serve — the case
   * that appears the moment you override the row above while a model is pinned.
   *
   * It is *kept*, never rewritten, so this says so out loud with the revert
   * control right beside it. Only claimed once the endpoint has answered:
   * `strandedModel` is `null` while the list is empty, because nothing is known
   * before it arrives.
   */
  const stranded =
    draft.model.model === null ? null : strandedModel(effectiveModel, catalog?.options);
  const keyMissing =
    provider !== undefined && keyProblem(provider, settings.keyStatuses[provider.id]) !== null;

  return (
    <>
      {hotkeyConflict ? (
        // `save_action` re-probes the Direct Hotkey and refuses the whole write
        // when it cannot be registered, so while this is true not even renaming
        // the Action can be saved. Say so, and offer the way out.
        <Callout tone="danger">
          <p>
            <strong>{t.settings.actions.hotkeyDeadLead}</strong> {hotkeyConflict}
          </p>
          <p>{t.settings.actions.hotkeyDeadBody}</p>
          <p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => store.editDraft((next) => (next.hotkey = null), true)}
            >
              {t.settings.actions.clearHotkey}
            </Button>
          </p>
        </Callout>
      ) : null}

      {/* What the Action is, behind one card that opens its own screen
          (ADR-0012): the four fields there are the only ones written rather
          than chosen, and stacking them ahead of the choices buried the
          choices. It sits above the first group head because it is not one of a
          set — it is the other half of this Action. */}
      <FieldGroup>
        <NavCard
          label={t.settings.actions.definition}
          hint={t.settings.actions.definitionHint}
          warning={definitionWarning}
          onClick={() => store.showScreen("definition")}
        />
      </FieldGroup>

      <FieldGroup title={t.settings.actions.trigger}>
        <Field
          label={t.settings.actions.inputSource}
          hint={t.settings.actions.sourceHint[draft.input_source]}
        >
          {({ id, describedBy }) => (
            <Segmented
              id={id}
              describedBy={describedBy}
              label={t.settings.actions.inputSource}
              value={draft.input_source}
              options={sources}
              onChange={(source) => store.editDraft((next) => (next.input_source = source), true)}
            />
          )}
        </Field>

        <Field
          label={t.settings.actions.directHotkey}
          hint={t.settings.actions.directHotkeyHint}
        >
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

      {/* Every `[model]` key is optional, and absent means "inherit Model
          defaults". The control is live either way and shows the effective
          value, so touching it *is* the override — one gesture for a select and
          a switch alike. What says which side of the default a row is on is
          `Field`'s `override`: a dot in the label's gutter and a revert control,
          on an overridden row only, with the head's note covering the rest. This
          was three bordered boxes indented into the value column, and the only
          thing on the pane that was not a row of its own (ADR-0011). */}
      {keyMissing && provider ? (
        // The per-Action version of "no credential". It belongs here rather than
        // only on the Connection pane: with an endpoint per Action, one Action
        // can be broken while every other one works (ADR-0021).
        <Callout tone="warn">
          <p>{t.settings.actions.needsKey(provider.label)}</p>
        </Callout>
      ) : null}

      <FieldGroup
        title={t.settings.actions.overrides}
        note={t.settings.actions.overridesNote(
          fallback?.label ?? config.defaults.provider,
        )}
      >
        <Field
          label={t.settings.actions.provider}
          hint={t.settings.actions.providerHint}
          override={{
            overridden: draft.model.provider !== null,
            defaultReading: fallback?.label ?? config.defaults.provider,
            onRevert: () => store.editDraft((next) => (next.model.provider = null), true),
          }}
        >
          {({ id, describedBy }) => (
            // The effective value, like every other row here: the control shows
            // what a request would carry, and touching it *is* the override.
            <Select
              value={provider?.id ?? ""}
              onValueChange={(next) =>
                store.editDraft((draft) => (draft.model.provider = next), true)
              }
            >
              <SelectTrigger id={id} aria-describedby={describedBy} className="w-fit min-w-40">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {config.api.providers.map((one) => (
                  <SelectItem key={one.id} value={one.id}>
                    {isLocal(one) ? t.settings.actions.providerLocal(one.label) : one.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </Field>

        <Field
          label={t.controls.model.label}
          hint={modelHint || stranded ? undefined : modelInfo}
          error={
            stranded
              ? t.settings.actions.strandedModel(
                  stranded,
                  provider?.label ?? "",
                  provider?.model ?? "",
                )
              : modelHint
          }
          override={{
            overridden: draft.model.model !== null,
            defaultReading: provider?.model ?? "",
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
          label={t.controls.model.thinking}
          warning={thinkingHint}
          hint={t.settings.actions.thinkingHint}
          override={{
            overridden: draft.model.thinking !== null,
            defaultReading: provider?.thinking ? t.controls.field.on : t.controls.field.off,
            onRevert: () => store.editDraft((next) => (next.model.thinking = null), true),
          }}
        >
          {({ id, describedBy }) => (
            <OnOffSwitch
              id={id}
              describedBy={describedBy}
              label={t.controls.model.thinking}
              checked={effectiveThinking}
              onChange={(value) => store.editDraft((next) => (next.model.thinking = value), true)}
            />
          )}
        </Field>

        <Field
          label={t.controls.model.webSearch}
          warning={webSearchHint}
          hint={t.settings.actions.webSearchHint}
          override={{
            overridden: draft.model.web_search !== null,
            defaultReading: provider?.web_search ? t.controls.field.on : t.controls.field.off,
            onRevert: () => store.editDraft((next) => (next.model.web_search = null), true),
          }}
        >
          {({ id, describedBy }) => (
            <OnOffSwitch
              id={id}
              describedBy={describedBy}
              label={t.controls.model.webSearch}
              disabled={webSearchOffOnly}
              checked={effectiveWebSearch}
              onChange={(value) => store.editDraft((next) => (next.model.web_search = value), true)}
            />
          )}
        </Field>
      </FieldGroup>

      {/* What a turn would actually carry, in one place. With an endpoint per
          Action, "where did this go" is no longer answerable from one global
          setting, so it is answered here (ADR-0021). */}
      {provider ? (
        <FieldGroup title={t.settings.actions.sends}>
          <div className="text-muted-foreground grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 font-mono text-meta">
            <span>POST</span>
            <span className="text-foreground [overflow-wrap:anywhere]">{chatUrl(provider)}</span>
            <span>model</span>
            <span className="text-foreground">{effectiveModel || "—"}</span>
            {/* Only when it is on, and only where it reaches a field: a row
                reading "web search off" on every Action is noise, and one
                reading "on" at an endpoint that has none would be a lie
                (ADR-0026). */}
            {effectiveWebSearch && canWebSearch(provider) ? (
              <>
                <span>search</span>
                <span className="text-foreground">{t.controls.field.on}</span>
              </>
            ) : null}
          </div>
        </FieldGroup>
      ) : null}

      {/* A card like every other, under a head of its own: with no hairlines
          left on the pane there is no divider to sit above, and a group head is
          what now says "this is not one of the settings". */}
      <FieldGroup title={t.settings.actions.thisFile}>
        <Field
          label={t.settings.actions.deleteLabel}
          hint={t.settings.actions.deleteHint(action.file_name)}
        >
          {/* Destructive up front, not only once the pointer is over it: hover is
              not a state a keyboard user passes through. The outline carries that
              at rest; solid red is the confirmation dialog's.

              Sized like the controls above it rather than as the pane's one
              filled button: the colour is what makes this row findable, so the
              extra box only made deleting look like the thing to do here. */}
          {() => (
            <Button
              variant="destructive-outline"
              size="sm-note"
              onClick={() => store.askDelete(action)}
            >
              {t.settings.actions.deleteButton}
            </Button>
          )}
        </Field>
      </FieldGroup>
    </>
  );
}
