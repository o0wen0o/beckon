// There is no Save button and there never will be (ADR-0003), so this line is
// where that promise is kept visible — and the one place a failed write is
// reported, rather than a banner competing with the form above it.
import { CheckIcon, LoaderCircleIcon, TriangleAlertIcon } from "lucide-react";

interface StatusBarProps {
  busy: boolean;
  error: string | null;
  /**
   * What this pane actually promises, when it is not the usual promise. The raw
   * file editor has a Save button of its own (a file that does not parse cannot
   * be written on every keystroke), and the standing line would sit directly
   * beneath it saying the opposite.
   */
  note?: string | null;
}

export function StatusBar({ busy, error, note = null }: StatusBarProps) {
  return (
    <div
      role="status"
      aria-live="polite"
      className={`bg-card font-small flex h-7 flex-none items-center gap-2 border-t px-4 text-xs ${
        error !== null ? "text-destructive" : "text-muted-foreground"
      }`}
    >
      {error ? (
        <>
          <TriangleAlertIcon className="size-3.5 flex-none" />
          <span>Not saved — {error}</span>
        </>
      ) : busy ? (
        <>
          {/* A frozen spinner reads as a stalled write, so the reduced-motion
              form is a static ring rather than a stopped one. */}
          <LoaderCircleIcon className="text-primary size-3.5 flex-none animate-spin motion-reduce:animate-none motion-reduce:opacity-50" />
          <span>Saving…</span>
        </>
      ) : (
        <>
          <CheckIcon className="size-3.5 flex-none" />
          <span>{note ?? "Changes are written to disk as you make them."}</span>
        </>
      )}
    </div>
  );
}
