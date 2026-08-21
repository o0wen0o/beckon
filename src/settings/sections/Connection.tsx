import { CheckIcon, TriangleAlertIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Callout } from "@/components/Callout";
import { Field } from "@/components/Field";
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
      store.setTest({ state: "failed", message: describeFailure(describeError(error)) });
    }
    store.setKeyResult(await getKeyStatus(), store.keyMessage);
  }

  return (
    <>
      <h1 className="font-display mb-6 text-xl font-semibold">Connection</h1>

      {store.firstRun ? (
        <Callout>
          <p>
            <strong>Welcome.</strong> Beckon needs a DeepSeek API key before it can do anything.
          </p>
          <p>The key goes into the Windows Credential Manager, never into a file.</p>
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

      {/* The state line lives inside the field rather than after it: it is what
          the field currently holds, so it reads on the field's own rhythm. */}
      <Field label="API key">
        {({ id, describedBy }) => (
          <div className="flex max-w-155 flex-col gap-1">
            <div className="flex items-center gap-2">
              {/* The same measure as Base URL below. The row is wider than the
                  field because it also holds the buttons; letting the field
                  take that width would leave the two text boxes on the page
                  ending at different x. */}
              <Input
                id={id}
                aria-describedby={describedBy}
                className="max-w-105"
                type="password"
                value={store.keyDraft}
                placeholder="sk-…"
                autoComplete="off"
                onChange={(event) => store.setKeyDraft(event.currentTarget.value)}
                onKeyDown={(event) => event.key === "Enter" && void saveKey()}
              />
              <Button disabled={store.keyDraft.trim() === ""} onClick={() => void saveKey()}>
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

            {/* The three key states stay three distinguishable outcomes all the
                way to the UI (ADR-0005): stored, never stored, unreadable. */}
            {store.keyStatus?.kind === "present" ? (
              <p className="font-small m-0 flex items-center gap-1 text-success text-xs">
                <CheckIcon className="size-3.5" /> Stored — ends in{" "}
                <code className="font-mono">{store.keyStatus.last4}</code>
              </p>
            ) : store.keyStatus?.kind === "no-credential" ? (
              <p className="text-muted-foreground font-small m-0 text-xs">No key stored yet.</p>
            ) : store.keyStatus?.kind === "read-error" ? (
              <p className="text-destructive font-small m-0 flex items-start gap-1 text-xs">
                <TriangleAlertIcon className="size-3.5 flex-none" />
                The Credential Manager could not be read: {store.keyStatus.message}. Save the key
                again to recreate the credential.
              </p>
            ) : null}

            {store.keyMessage ? (
              <p className="text-muted-foreground font-small m-0 text-xs">{store.keyMessage}</p>
            ) : null}
          </div>
        )}
      </Field>

      {config ? (
        <Field
          label="Base URL"
          hint="Any OpenAI-compatible endpoint. Requests go to /v1/chat/completions."
        >
          {({ id, describedBy }) => (
            <Input
              id={id}
              aria-describedby={describedBy}
              className="max-w-105"
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

      <div className="mt-2 flex items-center gap-2">
        <Button
          variant="outline"
          onClick={() => void runTest()}
          disabled={store.test.state === "running"}
        >
          {store.test.state === "running" ? "Testing…" : "Test connection"}
        </Button>
        {store.test.message ? (
          <span
            className={`font-small text-xs ${
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
    </>
  );
}
