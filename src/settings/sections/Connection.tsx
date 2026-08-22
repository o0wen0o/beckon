import { CheckIcon, TriangleAlertIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { describeFailure } from "@/lib/failures";
import { useT } from "@/lib/i18n";
import {
  deleteApiKey,
  describeError,
  getKeyStatus,
  openApiKeyPage,
  setApiKey,
  testConnection,
} from "@/lib/ipc";
import { useStore } from "@/lib/useStore";
import { settings } from "../store";

export function Connection() {
  const t = useT();
  const store = useStore(settings);
  const config = store.config;
  const credentialStore = t.words.credentialStore;

  async function saveKey() {
    const key = store.keyDraft.trim();
    if (key === "") return;
    try {
      const status = await setApiKey(key);
      store.setKeyDraft("");
      store.setKeyResult(status, t.settings.connection.saved);
      void store.refreshModels();
    } catch (error) {
      store.setKeyResult(null, describeError(error).message);
    }
  }

  async function removeKey() {
    try {
      store.setKeyResult(await deleteApiKey(), t.settings.connection.removed);
      void store.refreshModels();
    } catch (error) {
      store.setKeyResult(null, describeError(error).message);
    }
  }

  async function runTest() {
    store.setTest({ state: "running" });
    try {
      await testConnection();
      store.setTest({ state: "ok", message: t.settings.connection.testOk });
    } catch (error) {
      store.setTest({
        state: "failed",
        message: describeFailure(describeError(error), t),
      });
    }
    store.setKeyResult(await getKeyStatus(), store.keyMessage);
  }

  return (
    <>
      <PaneHeader title={t.settings.connection.title}>
        {t.settings.connection.lede(credentialStore)}
      </PaneHeader>

      {store.firstRun ? (
        <Callout>
          <p>
            <strong>{t.settings.connection.welcomeLead}</strong>
            {t.settings.connection.welcomeBody}
          </p>
          <p>
            <Button
              variant="link"
              onClick={() => void openApiKeyPage()}
            >
              {t.settings.connection.getKey}
            </Button>
          </p>
        </Callout>
      ) : null}

      <FieldGroup title={t.settings.connection.credential}>
        {/* The state line lives inside the field rather than after it: it is
            what the field currently holds, so it reads on the field's own
            rhythm — above the explanation, below the control. */}
        {/* Stacked: it is typed, and the two buttons share its line, so there is
            no width at which it could right-align against its own name. */}
        <Field label={t.settings.connection.apiKey} stacked>
          {({ id, describedBy }) => (
            <div className="flex flex-col gap-1.25">
              {/* The line, not the field, is what takes the wide measure: the
                  buttons live on it, so holding the whole line to 420px is what
                  keeps this control the same width as the others rather than
                  running the length of the card. */}
              <div className="flex max-w-control-wide items-center gap-2">
                <Input
                  id={id}
                  aria-describedby={describedBy}
                  className="min-w-0 flex-1"
                  type="password"
                  value={store.keyDraft}
                  placeholder="sk-…"
                  autoComplete="off"
                  onChange={(event) => store.setKeyDraft(event.currentTarget.value)}
                  onKeyDown={(event) => event.key === "Enter" && void saveKey()}
                />
                {/* The one green thing in the window. Everything else on the
                    pane is written to a TOML file as you type (ADR-0003), so
                    nothing else needs — or may have — a commit button. This one
                    does: the key goes to the OS credential store, it is
                    cleared from the field the moment it lands, and there is no
                    way to read it back to check. So it carries a colour — and
                    it is outlined rather than filled, because Remove sits on
                    the same line and two solid buttons there would each read as
                    the thing to press. */}
                <Button
                  variant="success-outline"
                  disabled={store.keyDraft.trim() === ""}
                  onClick={() => void saveKey()}
                >
                  {t.settings.connection.save}
                </Button>
                {store.keyStatus?.kind === "present" ? (
                  // Outlined, not filled: it sits beside Save, and a solid red
                  // button reads as the thing to press.
                  <Button variant="destructive-outline" onClick={() => void removeKey()}>
                    {t.settings.connection.remove}
                  </Button>
                ) : null}
              </div>

              {/* The three key states stay three distinguishable outcomes all
                  the way to the UI (ADR-0005): stored, never stored,
                  unreadable. */}
              {store.keyStatus?.kind === "present" ? (
                <p className="m-0 flex items-center gap-1 text-success text-note">
                  <CheckIcon className="size-3.5" /> {t.settings.connection.stored}{" "}
                  <code className="font-mono">{store.keyStatus.last4}</code>
                </p>
              ) : store.keyStatus?.kind === "no-credential" ? (
                <p className="text-muted-foreground m-0 text-note">
                  {t.settings.connection.noKeyYet}
                </p>
              ) : store.keyStatus?.kind === "read-error" ? (
                <p className="text-destructive m-0 flex items-start gap-1 text-note">
                  <TriangleAlertIcon className="size-3.5 flex-none" />
                  {t.settings.connection.readError(credentialStore, store.keyStatus.message)}
                </p>
              ) : null}

              {store.keyMessage ? (
                <p className="text-muted-foreground m-0 text-note">{store.keyMessage}</p>
              ) : null}
            </div>
          )}
        </Field>
      </FieldGroup>

      <FieldGroup title={t.settings.connection.endpoint}>
        {config ? (
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
                value={config.api.base_url}
                spellCheck={false}
                onChange={(event) => {
                  const next = event.currentTarget.value;
                  store.editConfig((draft) => (draft.api.base_url = next));
                }}
              />
            )}
          </Field>
        ) : null}

        <Field
          label={t.settings.connection.reachability}
          hint={t.settings.connection.reachabilityHint}
        >
          {() => (
            <div className="flex items-center gap-2">
              {/* One register down, like Refresh models on the pane it shares a
                  purpose with: this is a check, not a commit, and the readout
                  beside it is already 12px. `font-medium` over `outline`'s own
                  400 — at 12px the light weight goes thinner than the label. */}
              <Button
                variant="outline"
                size="sm"
                className="text-note font-medium"
                onClick={() => void runTest()}
                disabled={store.test.state === "running"}
              >
                {store.test.state === "running"
                  ? t.settings.connection.testing
                  : t.settings.connection.test}
              </Button>
              {store.test.message ? (
                <span
                  className={`text-note ${
                    store.test.state === "failed"
                      ? "text-destructive"
                      : store.test.state === "ok"
                        ? "text-success"
                        : "text-muted-foreground"
                  }`}
                >
                  {store.test.message}
                </span>
              ) : null}
            </div>
          )}
        </Field>
      </FieldGroup>
    </>
  );
}
