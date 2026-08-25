// The endpoints you keep (ADR-0021).
//
// This pane used to be a switch — one `base_url`, one key. It is not one any
// more: with `provider` an Action-level override, nothing here decides where a
// request goes. An Action does. So the pane is an **inventory**: the endpoints
// you keep, what each one costs you in credentials, and which Actions use it.
//
// Two consequences, both visible below:
//
// - No Enable button and no active row. One row is the *default* — what an
//   Action that names no provider gets — and that is a statement about
//   inheritance rather than a state, so it is a badge and a sentence in the
//   header rather than a verb on every row.
// - Every row carries its Action count. A global switch answered "where does my
//   text go" for free; this design has to earn that answer back, and this column
//   is where it does. A local row says so out loud.
//
// A row opens its own screen rather than a dialog (ADR-0012), and the screen has
// no Save button and must never have one (ADR-0003): every field commits to disk
// debounced, and the `config-changed` echo re-renders the list behind it. The one
// exception is the key, which goes to the OS credential store and cannot be read
// back — so that one field alone carries a commit button.
import * as React from "react";
import {
  ArrowLeftIcon,
  CheckIcon,
  PencilIcon,
  PlusIcon,
  SplitIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { CARD, CARD_HOVER, Field } from "@/components/Field";
import { Callout } from "@/components/Callout";
import { FieldGroup } from "@/components/FieldGroup";
import { ModelSelect } from "@/components/ModelSelect";
import { OnOffSwitch } from "@/components/OnOffSwitch";
import { PaneHeader } from "@/components/PaneHeader";
import { describeFailure } from "@/lib/failures";
import { useT, type Strings } from "@/lib/i18n";
import {
  deleteApiKey,
  describeError,
  openKeyPage,
  setApiKey,
  testConnection,
} from "@/lib/ipc";
import { canSuppressThinking, modelOptions, thinkingWarning, unknownModelHint } from "@/lib/models";
import {
  actionsByProvider,
  actionsUsing,
  blankProvider,
  chatUrl,
  host,
  isLocal,
  keyProblem,
  relaysThrough,
} from "@/lib/providers";
import { toasts } from "@/lib/toast";
import type { Provider } from "@/lib/types";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../actions";
import { settings } from "../store";

export function Connection() {
  const store = useStore(settings);
  const open = store.editingProvider;
  const row = open === null ? null : store.config?.api.providers.find((one) => one.id === open);
  // A row that vanished — an external edit to the file while its screen was
  // open — falls back to the inventory rather than to an empty screen.
  return row ? <EndpointScreen provider={row} /> : <Inventory />;
}

/** How many Actions a row carries, as its one line of prose. */
function usedBy(names: string[], t: Strings): string {
  if (names.length === 0) return t.settings.connection.usedByNone;
  if (names.length === 1) return t.settings.connection.usedByOne(names[0]);
  return t.settings.connection.usedByMany(names.length);
}

/* --------------------------------- the list --------------------------------- */

function Inventory() {
  const t = useT();
  const store = useStore(settings);
  const actions = useStore(actionStore).snapshot.actions;
  const config = store.config;
  if (!config) return null;

  const providers = config.api.providers;
  const fallback = store.defaultProvider;
  // Grouped once rather than filtered per row: the counts below are re-derived
  // on every notify, and a filter each is O(rows x actions) with a fresh array
  // per row.
  const usersByProvider = actionsByProvider(actions, config.defaults.provider);

  /** Rows the config does not already hold. Adding one is an ordinary config
   *  write — a preset is data, not a code path (ADR-0021). */
  const unused = store.presets.filter((one) => !providers.some((row) => row.id === one.id));

  function add(provider: Provider) {
    settings.editConfig((draft) => draft.api.providers.push(provider), true);
    // The row exists now; its credential status and its model list do not. One
    // row's status, not the whole map — and it is read rather than assumed
    // absent because a preset removed and re-added carries the same id, whose
    // key `save_config` deleted on the way out.
    void settings.refreshKey(provider.id);
    void settings.refreshModels(provider.id);
    settings.editProvider(provider.id);
  }

  return (
    <>
      <PaneHeader
        title={t.settings.connection.title}
        action={
          <div className="flex items-center gap-2">
            {unused.length > 0 ? (
              // Keyed on the count so the trigger returns to its placeholder
              // after a pick: this select chooses an *action*, not a value, so
              // it must not sit there displaying the last thing added.
              <Select
                key={unused.length}
                value=""
                onValueChange={(id) => {
                  const preset = unused.find((one) => one.id === id);
                  if (preset) add(structuredClone(preset));
                }}
              >
                <SelectTrigger className="w-fit" aria-label={t.settings.connection.addPreset}>
                  <SelectValue placeholder={t.settings.connection.addPreset} />
                </SelectTrigger>
                <SelectContent>
                  {unused.map((preset) => (
                    <SelectItem key={preset.id} value={preset.id}>
                      {preset.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : null}
            <Button variant="ghost" size="sm" onClick={() => add(blankProvider(providers))}>
              <PlusIcon className="size-3.5" /> {t.settings.connection.addBlank}
            </Button>
          </div>
        }
      >
        {t.settings.connection.lede(
          fallback?.label ?? config.defaults.provider,
          t.words.credentialStore,
        )}
      </PaneHeader>

      {store.firstRun && fallback ? (
        <Callout>
          <p>
            <strong>{t.settings.connection.welcomeLead}</strong>
            {t.settings.connection.welcomeBody}
          </p>
          <p>
            <Button variant="link" onClick={() => settings.editProvider(fallback.id)}>
              {t.settings.connection.setUp(fallback.label)}
            </Button>
          </p>
        </Callout>
      ) : null}

      {/* No group head: the header sentence above already says what the list
          is, and a group note without a title renders nothing. */}
      <FieldGroup>
        {providers.map((provider) => {
          const isDefault = provider.id === config.defaults.provider;
          const key = store.keyStatuses[provider.id];
          const users = usersByProvider.get(provider.id) ?? [];
          const local = isLocal(provider);
          const unkeyed = keyProblem(provider, key) !== null;
          const label = provider.label.trim() || t.settings.connection.unnamed;
          return (
            <div
              key={provider.id}
              className={[
                CARD,
                CARD_HOVER,
                "group flex items-center gap-3",
                // The strong edge, and nothing else: the badge beside the name
                // is what carries the claim, and a fill behind it would be a
                // second one on the same row. Not "the one in use" either —
                // several rows can be in use at once now.
                isDefault ? "border-foreground" : "",
              ].join(" ")}
            >
              {/* A monogram, not a logo: artwork per provider is a file per
                  provider to keep current, and a base_url you typed has none. */}
              <span
                aria-hidden
                className="bg-muted text-muted-foreground grid size-8 flex-none place-items-center rounded-md border font-medium"
              >
                {label.charAt(0).toUpperCase()}
              </span>

              <span className="flex min-w-0 flex-1 flex-col">
                <span className="flex items-center gap-2">
                  <span className="truncate font-medium">{label}</span>
                  {/* Inversion, which is this window's one accent — and the pane
                      spends it here, on the single row whose status differs.
                      That is also why the card behind it no longer fills: one
                      inverted thing per pane, or the accent stops meaning
                      anything.

                      Small and sentence-cased, not an uppercase eyebrow: an
                      eyebrow is a *label over a group*, and this is a word about
                      the row it sits on. Caps there also read as a state being
                      announced, where what this says is quieter — which row an
                      Action inherits. Cased in the catalog rather than by CSS,
                      because 默认 has no case to set and `uppercase` would be a
                      rule that does nothing in half the product. */}
                  {isDefault ? (
                    <Badge className="flex-none px-1.5 py-0 text-meta">
                      {t.settings.connection.defaultTag}
                    </Badge>
                  ) : null}
                </span>
                <span className="text-muted-foreground truncate font-mono text-meta">
                  {host(provider.base_url)}
                </span>
                <span className="flex items-center gap-2 text-meta">
                  <span className={users.length === 0 ? "text-muted-quiet" : "text-foreground"}>
                    {usedBy(
                      users.map((one) => one.name || one.file_name),
                      t,
                    )}
                  </span>
                  {/* `local` is the one word on this pane worth saying twice, so
                      it is here as well as on the endpoint's own screen. */}
                  {local ? (
                    <span className="text-muted-quiet">· {t.settings.connection.staysLocal}</span>
                  ) : null}
                  {unkeyed ? (
                    <span className="text-warning">· {t.settings.connection.missingKey}</span>
                  ) : null}
                </span>
              </span>

              <Button
                variant="ghost"
                size="icon-sm"
                className="text-muted-quiet flex-none"
                title={t.settings.connection.edit(label)}
                aria-label={t.settings.connection.edit(label)}
                onClick={() => settings.editProvider(provider.id)}
              >
                <PencilIcon className="size-3.5" />
              </Button>

              <RemoveButton provider={provider} users={users.length} icon />
            </div>
          );
        })}
      </FieldGroup>
    </>
  );
}

/**
 * The one control that refuses rather than cascading.
 *
 * A row an Action names cannot be removed out from under it: which endpoint
 * those Actions should use instead is not a decision this button gets to make,
 * and repointing them silently would move where somebody's text goes.
 */
function RemoveButton({
  provider,
  users,
  icon = false,
}: {
  provider: Provider;
  users: number;
  icon?: boolean;
}) {
  const t = useT();
  const store = useStore(settings);
  const last = (store.config?.api.providers.length ?? 0) <= 1;
  const label = provider.label.trim() || t.settings.connection.unnamed;

  function remove() {
    settings.editConfig((draft) => {
      draft.api.providers = draft.api.providers.filter((one) => one.id !== provider.id);
    }, true);
    settings.editProvider(null);
    // The default that just left is *not* repointed here: `save_config` folds
    // the table at the boundary, which is where "`defaults.provider` names a row
    // that exists" lives (ADR-0021), and `store.defaultProvider` covers the one
    // frame before the echo. Nor is the credential map re-read — the row is
    // gone, so its stale entry has nothing left to draw.
  }

  const blocked = last || users > 0;
  const why = last ? t.settings.connection.removeLast : t.settings.connection.removeLabel(label);

  if (icon) {
    return (
      <Button
        variant="ghost"
        size="icon-sm"
        className="text-muted-quiet hover:text-destructive flex-none"
        title={why}
        aria-label={t.settings.connection.removeLabel(label)}
        disabled={blocked}
        onClick={remove}
      >
        <Trash2Icon className="size-3.5" />
      </Button>
    );
  }
  return (
    <Button variant="destructive-outline" size="sm-note" disabled={blocked} onClick={remove}>
      {t.settings.connection.removeLabel(label)}
    </Button>
  );
}

/* ---------------------------- one endpoint's screen -------------------------- */

function EndpointScreen({ provider }: { provider: Provider }) {
  const t = useT();
  const store = useStore(settings);
  const actions = useStore(actionStore).snapshot.actions;
  const config = store.config;

  const key = store.keyStatuses[provider.id];
  const catalog = store.models[provider.id] ?? null;
  const test = store.testFor(provider.id);
  const loading = store.modelsLoading.has(provider.id);
  const label = provider.label.trim() || t.settings.connection.unnamed;
  const isDefault = provider.id === config?.defaults.provider;
  const users = config
    ? actionsUsing(provider.id, actions, config.defaults.provider)
    : [];
  // A row from a preset carries the right `reasoning` already, and putting that
  // field on screen there is an invitation to break it. A row you typed yourself
  // is the only one whose wire quirks nobody has filled in.
  const handMade = !store.presets.some((one) => one.id === provider.id);

  // The row on screen is the one whose list is about to be read, so its live
  // fetch is paid for here: a reveal only primes every row's offline answer
  // (`settings.refreshAll`).
  React.useEffect(() => {
    void settings.refreshModels(provider.id);
  }, [provider.id]);

  const options = React.useMemo(
    () => modelOptions(provider.model, catalog),
    [provider.model, catalog],
  );
  const modelHint = unknownModelHint(provider.model || null, catalog, t);
  // Nothing to pick, which is the initial state of every row a user adds: a row
  // ships no model and the endpoint has not answered yet. A sentence naming what
  // to do rather than an empty select — and a different one for a local row,
  // which wants no key at all (ADR-0021). Read off the memo above rather than
  // through `modelOption`, which would rebuild the same list to search it.
  const modelInfo =
    options.length === 0
      ? isLocal(provider)
        ? t.settings.connection.noModelsYetLocal
        : t.settings.connection.noModelsYet
      : (options.find((one) => one.id === provider.model)?.description ?? "");
  // Whose word the list is, said once: a cached list is this endpoint's own ids
  // but not its answer today, and `listNotice` may be sitting beneath it saying
  // why (ADR-0024). A lookup rather than a ternary chain, because `source` is
  // already the one field that decides.
  const listSource = catalog
    ? { live: t.settings.connection.live, cached: t.settings.connection.cached, none: null }[
        catalog.source
      ]
    : null;
  const thinkingHint = thinkingWarning(provider, provider.model, provider.thinking, catalog, t);
  const relay = relaysThrough(provider);
  const listNotice =
    !catalog || catalog.source === "live" || !catalog.fallback
      ? null
      : t.settings.connection.listNotice(
          describeFailure(catalog.fallback, t, t.settings.connection.listUnavailable),
        );

  /** Every edit to this row goes through here, so no field invents its own
   *  write path. `immediate` for the choices; the typed fields debounce. */
  const edit = (patch: Partial<Provider>, immediate = false) =>
    settings.editConfig((draft) => {
      const target = draft.api.providers.find((one) => one.id === provider.id);
      if (target) Object.assign(target, patch);
    }, immediate);

  async function saveKey() {
    const typed = store.keyDraft.trim();
    if (typed === "") return;
    try {
      settings.setKeyResult(
        provider.id,
        await setApiKey(provider.id, typed),
        t.settings.connection.saved,
      );
      settings.setKeyDraft("");
      // `live`, not `asked`: this list is a consequence of saving a key, not the
      // question that was asked. `asked` was for the no-key fallback, which
      // cannot be the outcome one line after `set_api_key` returned — and every
      // other way it can fail, `runTest` below reports verbatim as its own
      // sentence. Left as `asked` it answered a gesture that already had two
      // answers, and Save became three reports for one click.
      void settings.refreshModels(provider.id, "live");
      // A key that does not work is worth knowing now rather than at the first
      // hotkey press, and the test is also what learns the dialect — so the one
      // gesture the user already made settles both. The button stays, for a
      // retry after something else changed.
      void runTest();
    } catch (error) {
      settings.setKeyResult(provider.id, null, describeError(error).message);
    }
  }

  async function removeKey() {
    try {
      settings.setKeyResult(
        provider.id,
        await deleteApiKey(provider.id),
        t.settings.connection.removed,
      );
      void settings.refreshModels(provider.id);
    } catch (error) {
      settings.setKeyResult(provider.id, null, describeError(error).message);
    }
  }

  async function runTest() {
    settings.setTest(provider.id, "running");
    try {
      const report = await testConnection(provider.id);
      // The endpoint answered a question the pane used to ask. Written straight
      // to the row: a dialect the user was never able to look up is not worth a
      // confirmation step, and `null` — nothing learned, or a preset already
      // knew — leaves the row exactly as it was.
      if (report.reasoning && report.reasoning !== provider.reasoning) {
        edit({ reasoning: report.reasoning }, true);
      }
      settings.setTest(provider.id, "ok");
      toasts.show(
        "ok",
        report.reasoning
          ? t.settings.connection.testOkDetected(
              t.settings.connection.reasoningName[report.reasoning],
            )
          : t.settings.connection.testOk,
      );
    } catch (error) {
      settings.setTest(provider.id, "failed");
      // Verbatim, as everywhere: a rejected key, a missing credential and an
      // unreachable API stay three different sentences (ADR-0005).
      toasts.show("danger", describeFailure(describeError(error), t));
    }
    // A test that failed on the credential may have found it changed. This
    // row's status only: the other N were not touched.
    void settings.refreshKey(provider.id);
  }

  return (
    <>
      <div className="mb-4">
        <Button variant="ghost" size="sm" onClick={() => settings.editProvider(null)}>
          <ArrowLeftIcon className="size-3.5" /> {t.settings.connection.back}
        </Button>
      </div>

      <PaneHeader
        title={label}
        action={
          isDefault ? (
            <span className="text-muted-quiet text-meta">
              {t.settings.connection.defaultForNew}
            </span>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                settings.editConfig((draft) => (draft.defaults.provider = provider.id), true)
              }
            >
              {t.settings.connection.makeDefault}
            </Button>
          )
        }
      >
        {chatUrl(provider)} ·{" "}
        {usedBy(
          users.map((one) => one.name || one.file_name),
          t,
        )}
      </PaneHeader>

      {/* A relaying row says so before a key is stored, and says it on the
          identity line rather than in a banner: this is a property of the URL
          directly above it, and a Callout is the shape a reader learns to skip
          (ADR-0025). Derived from the host, so a hand-typed broker URL discloses
          as loudly as the preset does. */}
      {relay ? (
        <div className="mb-6.5 -mt-3 flex items-start gap-2 text-note">
          <SplitIcon aria-hidden className="text-warning mt-0.5 size-3.5 flex-none" />
          <p className="text-muted-foreground m-0 max-w-measure">
            <span className="text-foreground font-medium">{t.settings.connection.relaysLead}</span>{" "}
            {t.settings.connection.relaysBody(relay)}
          </p>
        </div>
      ) : null}

      {listNotice ? (
        <Callout tone="warn">
          <p>{listNotice}</p>
        </Callout>
      ) : null}

      <FieldGroup title={t.settings.connection.endpoint}>
        {/* Name and id share one card, because they are one fact about this row
            with two halves: the word you read and the word an Action writes.
            Two cards asked the reader to hold the distinction across a gap, and
            the id half is not editable, so its own card was a box around a
            sentence.

            The id is plain text rather than a disabled input: it is the
            credential account and what an Action's `provider` names, so editing
            it here would orphan a stored key and break every Action pointing at
            this row — and a greyed-out box says "not now" where nothing says
            "not here". Still selectable, which is what a hand-edited Action file
            needs from it. */}
        <Field label={t.settings.connection.name} stacked hint={t.settings.connection.nameHint}>
          {({ id, describedBy }) => (
            <div className="flex max-w-control-wide items-center gap-2.5">
              <Input
                id={id}
                aria-describedby={describedBy}
                className="min-w-0 flex-1"
                value={provider.label}
                onChange={(event) => edit({ label: event.currentTarget.value })}
              />
              <code className="text-muted-quiet flex-none font-mono text-meta">{provider.id}</code>
            </div>
          )}
        </Field>

        <Field
          label={t.settings.connection.baseUrl}
          measure="field"
          stacked
          hint={t.settings.connection.baseUrlHint}
        >
          {({ id, describedBy }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              spellCheck={false}
              value={provider.base_url}
              onChange={(event) => edit({ base_url: event.currentTarget.value })}
            />
          )}
        </Field>

        {/* Stacked: it is typed, and the two buttons share its line, so there is
            no width at which it could right-align against its own name. */}
        <Field label={t.settings.connection.apiKey} stacked hint={t.settings.connection.apiKeyHint}>
          {({ id, describedBy }) => (
            <div className="flex flex-col gap-1.25">
              {/* The line, not the field, takes the wide measure: the buttons
                  live on it, so holding the whole line to 420px is what keeps
                  this control the same width as the others. */}
              <div className="flex max-w-control-wide items-center gap-2">
                <Input
                  id={id}
                  aria-describedby={describedBy}
                  className="min-w-0 flex-1"
                  type="password"
                  value={store.keyDraft}
                  placeholder="sk-…"
                  autoComplete="off"
                  onChange={(event) => settings.setKeyDraft(event.currentTarget.value)}
                  onKeyDown={(event) => event.key === "Enter" && void saveKey()}
                />
                {/* The one green thing in the window. Everything else on the
                    pane is written to a TOML file as you type (ADR-0003), so
                    nothing else needs — or may have — a commit button. This one
                    does: the key goes to the OS credential store, it is cleared
                    from the field the moment it lands, and there is no way to
                    read it back to check. Outlined rather than filled, because
                    Remove sits on the same line and two solid buttons there
                    would each read as the thing to press. */}
                <Button
                  variant="success-outline"
                  disabled={store.keyDraft.trim() === ""}
                  onClick={() => void saveKey()}
                >
                  {t.settings.connection.save}
                </Button>
                {key?.kind === "present" ? (
                  <Button variant="destructive-outline" onClick={() => void removeKey()}>
                    {t.settings.connection.remove}
                  </Button>
                ) : null}
              </div>

              {/* The three key states stay three distinguishable outcomes all
                  the way to the UI (ADR-0005) — and since ADR-0021 there is a
                  fourth reading of the second: nothing stored for a *local*
                  endpoint is a working setup, not a fault, so it is not amber. */}
              {key?.kind === "present" ? (
                <p className="m-0 flex items-center gap-1 text-success text-note">
                  <CheckIcon className="size-3.5" /> {t.settings.connection.stored}{" "}
                  <code className="font-mono">{key.last4}</code>
                </p>
              ) : key?.kind === "read-error" ? (
                <p className="text-destructive m-0 flex items-start gap-1 text-note">
                  <TriangleAlertIcon className="size-3.5 flex-none" />
                  {t.settings.connection.readError(t.words.credentialStore, key.message)}
                </p>
              ) : (
                <p
                  className={`m-0 text-note ${
                    isLocal(provider) ? "text-muted-foreground" : "text-warning"
                  }`}
                >
                  {isLocal(provider)
                    ? t.settings.connection.unauthenticated
                    : t.settings.connection.noKeyYet}
                </p>
              )}

              {store.keyMessage ? (
                <p className="text-muted-foreground m-0 text-note">{store.keyMessage}</p>
              ) : null}

              {provider.key_page && key?.kind !== "present" ? (
                // Rust refuses anything but `https` and the OS may refuse the
                // URL, so this can fail — and it is drawn precisely where there
                // is no key yet, which is the worst place for a link that opens
                // nothing and explains nothing.
                <Button
                  variant="link"
                  className="self-start"
                  onClick={() =>
                    void openKeyPage(provider.id).catch((error: unknown) =>
                      toasts.show(
                        "danger",
                        describeFailure(
                          describeError(error),
                          t,
                          t.settings.connection.keyPageFailed,
                        ),
                      ),
                    )
                  }
                >
                  {t.settings.connection.getKeyFrom(host(provider.key_page))}
                </Button>
              ) : null}
            </div>
          )}
        </Field>

        <Field
          label={t.settings.connection.reachability}
          hint={t.settings.connection.reachabilityHint}
        >
          {() => (
            <div className="flex items-center gap-2">
              {/* One register down, like Refresh models below it: this is a
                  check, not a commit. `font-medium` over `outline`'s own 400 —
                  at 12px the light weight goes thinner than the label.
                  The outcome is a toast: a rejected key quotes the endpoint
                  verbatim, and that sentence beside the button turned this row
                  into three lines of red with the button wedged at its left. */}
              <Button
                variant="outline"
                size="sm"
                className="text-note font-medium"
                onClick={() => void runTest()}
                disabled={test === "running"}
              >
                {test === "running"
                  ? t.settings.connection.testing
                  : t.settings.connection.test}
              </Button>
            </div>
          )}
        </Field>
      </FieldGroup>

      {/* What this endpoint hands to any Action that names it and nothing else.
          Under a head of their own, because they are about requests rather than
          about reaching the host. */}
      <FieldGroup
        title={t.settings.connection.rowDefaults}
        note={t.settings.connection.rowDefaultsNote}
      >
        <Field
          label={t.controls.model.label}
          hint={modelHint ? undefined : modelInfo}
          error={modelHint}
        >
          {({ id, describedBy }) => (
            <div className="flex items-center gap-2">
              <ModelSelect
                id={id}
                describedBy={describedBy}
                value={provider.model}
                options={options}
                placeholder={t.controls.model.noneChosen}
                onChange={(model) => edit({ model }, true)}
              />
              <Button
                variant="ghost"
                size="sm-note"
                disabled={loading}
                onClick={() => void settings.refreshModels(provider.id, "asked")}
              >
                {loading ? t.settings.connection.loading : t.settings.connection.refresh}
              </Button>
              {/* Whether the offer is this endpoint's own word or a guess. It
                  belongs on this row rather than in a section of its own: the
                  list it describes is right here. */}
              {listSource ? (
                <span className="text-muted-foreground text-meta">{listSource}</span>
              ) : null}
            </div>
          )}
        </Field>

        <Field
          label={t.controls.model.thinking}
          warning={thinkingHint}
          hint={
            canSuppressThinking(provider)
              ? t.settings.connection.thinkingHint(label)
              : t.settings.connection.thinkingHintNone(label)
          }
        >
          {({ id, describedBy }) => (
            <OnOffSwitch
              id={id}
              describedBy={describedBy}
              label={t.controls.model.thinking}
              checked={provider.thinking}
              onChange={(thinking) => edit({ thinking }, true)}
            />
          )}
        </Field>

        {/* Read, not chosen. A wrong dialect is a 400 on every turn and it is
            the one field on this row nobody can look up — so Test connection
            asks the endpoint and writes the answer here, and what is left is a
            statement of what it found. A preset row never shows it: the vendor's
            own docs already answered, and detection is the weaker source. */}
        {handMade ? (
          <Field
            label={t.settings.connection.reasoning}
            hint={t.settings.connection.reasoningHint[provider.reasoning]}
          >
            {() => (
              <span className="text-muted-foreground text-note">
                {t.settings.connection.reasoningName[provider.reasoning]}
              </span>
            )}
          </Field>
        ) : null}
      </FieldGroup>

      <FieldGroup title={t.settings.connection.thisEndpoint}>
        <Field
          label={t.settings.connection.removeLabel(label)}
          hint={
            users.length > 0
              ? t.settings.connection.removeBlocked(
                  users.map((one) => one.name || one.file_name).join(", "),
                )
              : t.settings.connection.removeHint
          }
        >
          {() => <RemoveButton provider={provider} users={users.length} />}
        </Field>
      </FieldGroup>
    </>
  );
}
