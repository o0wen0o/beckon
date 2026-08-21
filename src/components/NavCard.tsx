// A card that opens a screen instead of holding a control (ADR-0012). Same
// geometry as `Field`, so a navigating card and a configuring card sit in one
// column without a seam; the chevron is the only thing that says which is which.
//
// Hover strengthens the edge and the chevron — it does not fill. A fill means
// "selected" and nothing else on this surface.
import { ChevronRightIcon } from "lucide-react";
import { CARD, CARD_HOVER } from "@/components/Field";

interface NavCardProps {
  label: string;
  /** The permanent explanation, in the register a field's hint is set in. */
  hint?: string;
  /**
   * Something on the screen behind this card needs attention. Carried up here
   * because a warning nobody can see until they click is not a warning — the
   * same reason the navigation column flags a section.
   */
  warning?: string | null;
  onClick: () => void;
}

export function NavCard({ label, hint, warning = null, onClick }: NavCardProps) {
  const described = Boolean(hint || warning);

  return (
    <button
      type="button"
      onClick={onClick}
      className={`${CARD} ${CARD_HOVER} group focus-visible:ring-ring/25 grid grid-cols-[1fr_auto] items-center gap-x-6 text-left focus-visible:ring-[2px] focus-visible:outline-none`}
    >
      <span className="col-start-1 row-start-1 font-medium">{label}</span>

      {described ? (
        <span className="col-start-1 row-start-2 mt-0.75 flex max-w-measure flex-col gap-0.5">
          {warning ? <span className="text-warning text-note">{warning}</span> : null}
          {hint ? <span className="text-muted-foreground text-meta">{hint}</span> : null}
        </span>
      ) : null}

      <ChevronRightIcon
        className={`text-muted-quiet group-hover:text-foreground group-focus-visible:text-foreground col-start-2 row-start-1 size-4 flex-none transition-[transform,color] duration-150 ease-out group-hover:translate-x-0.5 group-focus-visible:translate-x-0.5 motion-reduce:transition-none ${
          described ? "row-span-2" : ""
        }`}
      />
    </button>
  );
}
