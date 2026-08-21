import { CheckIcon, TriangleAlertIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
import { FieldGroup } from "@/components/FieldGroup";
import { PaneHeader } from "@/components/PaneHeader";
import { describeFailure } from "@/lib/failures";
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
  const store = useStore(settings);
  const config = store.config;

  async function saveKey() {
    const key = store.keyDraft.trim();
    if (key === "") return;
    try {
      const status = await setApiKey(key);
      store.setKeyDraft("");
      store.setKeyResult(status, "Saved.");
      void store.refreshModels();
    } catch (error) {
      store.setKeyResult(null, describeError(error).message);
    }
  }

  async function removeKey() {
    try {
      store.setKeyResult(await deleteApiKey(), "Removed.");
      void store.refreshModels();
    } catch (error) {
      store.setKeyResult(null, describeError(error).message);
    }
  }

  async function runTest() {
    store.setTest({ state: "running" });
    try {
      await testConnection();
      store.setTest({ state: "ok", message: "The key and base URL work." });
    } catch (error) {
      store.setTest({
        state: "failed",
        message: describeFailure(describeError(error)),
      });
    }
    store.setKeyResult(await getKeyStatus(), store.keyMessage);
  }

  return (
    <>
      <PaneHeader title="Connection">
        Where requests go, and the credential they go with. The key lives in the Windows Credential
        Manager, never in a file.
      </PaneHeader>

      {store.firstRun ? (
        <Callout>
          <p>
            <strong>Welcome.</strong> Beckon needs a DeepSeek API key before it can do anything.
          </p>
          <p>
            <Button
              variant="link"
              className="h-auto p-0 underline"
              onClick={() => void openApiKeyPage()}
            >
              Get a key from platform.deepseek.com
            </Button>
          </p>
        </Callout>
      ) : null}

      <FieldGroup title="Credential">
        {/* The state line lives inside the field rather than after it: it is
            what the field currently holds, so it reads on the field's own
            rhythm — above the explanation, below the control. */}
        <Field label="API key">
          {({ id, describedBy }) => (
            <div className="flex flex-col gap-1.25">
              {/* The row, not the field, is what takes the wide measure: the
                  buttons live on the same line, so holding the whole line to
                  420px is what keeps this row inside the value column instead
                  of running past every other control on the pane. */}
              <div className="flex max-w-105 items-center gap-2">
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
                    does: the key goes to the Windows Credential Manager, it is
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
                  Save
                </Button>
                {store.keyStatus?.kind === "present" ? (
                  // Outlined, not filled: it sits beside Save, and a solid red
                  // button reads as the thing to press.
                  <Button variant="destructive-outline" onClick={() => void removeKey()}>
                    Remove
                  </Button>
                ) : null}
              </div>

              {/* The three key states stay three distinguishable outcomes all
                  the way to the UI (ADR-0005): stored, never stored,
                  unreadable. */}
              {store.keyStatus?.kind === "present" ? (
                <p className="m-0 flex items-center gap-1 text-success text-note">
                  <CheckIcon className="size-3.5" /> Stored — ends in{" "}
                  <code className="font-mono">{store.keyStatus.last4}</code>
                </p>
              ) : store.keyStatus?.kind === "no-credential" ? (
                <p className="text-muted-foreground m-0 text-note">No key stored yet.</p>
              ) : store.keyStatus?.kind === "read-error" ? (
                <p className="text-destructive m-0 flex items-start gap-1 text-note">
                  <TriangleAlertIcon className="size-3.5 flex-none" />
                  The Credential Manager could not be read: {store.keyStatus.message}. Save the key
                  again to recreate the credential.
                </p>
              ) : null}

              {store.keyMessage ? (
                <p className="text-muted-foreground m-0 text-note">{store.keyMessage}</p>
              ) : null}
            </div>
          )}
        </Field>
      </FieldGroup>

      <FieldGroup title="Endpoint">
        {config ? (
          <Field
            label="Base URL"
            measure="field"
            hint="Any OpenAI-compatible endpoint. Requests go to /v1/chat/completions."
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

        <Field label="Reachability" hint="Sends one small request with the stored key.">
          {() => (
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                onClick={() => void runTest()}
                disabled={store.test.state === "running"}
              >
                {store.test.state === "running" ? "Testing…" : "Test connection"}
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
