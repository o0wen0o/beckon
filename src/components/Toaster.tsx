// Where `lib/toast.ts` lands: bottom-right, stacked oldest-first, above the
// status bar and clear of it.
//
// Hosted by the shell rather than portalled, and deliberately *outside* the pane
// (src/lib/pane.tsx): a toast holds no focus and takes none, so the save
// protocol never sees it — and being outside the pane is what keeps it still
// while the pane scrolls under it.
import { CheckIcon, TriangleAlertIcon, XIcon, type LucideIcon } from "lucide-react";
import { useT } from "@/lib/i18n";
import { toasts, type ToastTone } from "@/lib/toast";
import { useStore } from "@/lib/useStore";
import { cn } from "@/lib/utils";

/** Everything one tone decides, in one row each: the rule carries the tone
 *  exactly as `Callout`'s does, so the two read as the same vocabulary seen
 *  twice, and prose stays muted in both. One table rather than four lookups so
 *  a fourth tone cannot be half-added — and so the icons two tones share are
 *  visibly a choice rather than the fallback arm of a ternary. */
const TONE: Record<
  ToastTone,
  { rule: string; icon: string; Icon: LucideIcon; role: "status" | "alert" }
> = {
  ok: { rule: "border-l-success", icon: "text-success", Icon: CheckIcon, role: "status" },
  warn: {
    rule: "border-l-warning",
    icon: "text-warning",
    Icon: TriangleAlertIcon,
    role: "status",
  },
  danger: {
    rule: "border-l-destructive",
    icon: "text-destructive",
    Icon: TriangleAlertIcon,
    role: "alert",
  },
};

export function Toaster() {
  const t = useT();
  const store = useStore(toasts);

  if (store.items.length === 0) return null;

  return (
    // `pointer-events-none` on the stack and back on for each toast: the column
    // is as wide as its widest sentence and as tall as all of them, and an
    // invisible strip over the pane's bottom-right corner would swallow clicks
    // meant for whatever is under it.
    //
    // No live region on the column itself: each toast carries its own `status`
    // or `alert`, and a live region wrapping live regions announces twice.
    <div className="pointer-events-none fixed right-5 bottom-11 z-50 flex max-w-measure flex-col gap-2">
      {store.items.map((toast) => {
        const { rule, icon, Icon, role } = TONE[toast.tone];
        return (
          <div
            key={toast.id}
            role={role}
            className={cn(
              "bg-background text-muted-foreground pointer-events-auto flex items-start gap-2",
              "rounded-md border border-l-2 py-2 pr-2 pl-3 text-note shadow-md",
              "animate-in fade-in-0 slide-in-from-bottom-2 duration-200 ease-out motion-reduce:animate-none",
              rule,
            )}
          >
            <Icon aria-hidden className={cn("mt-0.5 size-3.5 flex-none", icon)} />
            <span className="min-w-0">{toast.message}</span>
            {/* A dismiss rather than a click-anywhere target: the message can be
                a quoted cause worth selecting, and selecting text must not be
                the gesture that deletes it. */}
            <button
              type="button"
              aria-label={t.words.dismiss}
              onClick={() => toasts.dismiss(toast.id)}
              className="text-muted-quiet hover:text-foreground -my-0.5 flex-none rounded p-0.5"
            >
              <XIcon className="size-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
