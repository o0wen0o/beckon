// A file that fails to parse is never dropped (ADR-0003) — it is reported and
// stays editable as text, which is the only way back from a bad hand-edit.
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Callout } from "@/components/Callout";
import { useStore } from "@/lib/useStore";
import { actionStore } from "../../actions";

export function RawFileEditor() {
  const store = useStore(actionStore);
  const raw = store.raw;
  if (!raw) return null;

  const parseError = store.snapshot.errors.find((error) => error.file_name === raw.file)?.message;

  return (
    <>
      {parseError ? (
        <Callout tone="danger">
          <p>{parseError}</p>
        </Callout>
      ) : null}

      <Textarea
        className="font-mono min-h-55 text-quiet"
        spellCheck={false}
        value={raw.text}
        onChange={(event) => store.editRaw(event.currentTarget.value)}
      />

      {raw.error ? <p className="text-destructive mt-1 text-note">{raw.error}</p> : null}

      <div className="mt-3 flex items-center gap-2">
        <Button onClick={() => void store.saveRaw()}>Save file</Button>
        <span className="text-muted-foreground text-meta">
          It reloads the moment it parses.
        </span>
      </div>
    </>
  );
}
