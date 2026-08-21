// One entry per section, Actions included — a single "Actions" item, not one per
// file: the list belongs in the pane, where a row has room for its Input Source
// and its hotkey. A nav column of Actions would make "pick a section" and "pick
// an Action" the same gesture for two unlike things.
import { FolderIcon, KeyboardIcon, ListIcon, PaletteIcon, PlugIcon, SlidersIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { BrandMark } from "@/components/BrandMark";
import { revealConfigDir } from "@/lib/ipc";
import { useStore } from "@/lib/useStore";
import { actionStore } from "./actions";
import { settings, type SectionRoute } from "./store";

const SECTIONS: { id: SectionRoute; label: string; icon: typeof PlugIcon }[] = [
  { id: "connection", label: "Connection", icon: PlugIcon },
  { id: "actions", label: "Actions", icon: ListIcon },
  { id: "triggering", label: "Triggering", icon: KeyboardIcon },
  { id: "appearance", label: "Appearance", icon: PaletteIcon },
  { id: "defaults", label: "Model defaults", icon: SlidersIcon },
];

export function SettingsNav() {
  const store = useStore(settings);
  const actions = useStore(actionStore);

  /** A section is flagged when something inside it needs attention, so a
   *  problem in a pane you are not looking at is still discoverable. A degraded
   *  model list is the warning colour, not red: the dropdown still works. */
  const flags: Record<SectionRoute, "bad" | "warn" | null> = {
    connection:
      store.keyStatus?.kind === "read-error" || store.test.state === "failed"
        ? "bad"
        : store.keyStatus?.kind === "no-credential"
          ? "warn"
          : null,
    // A file that does not parse, or a Direct Hotkey that lost its conflict:
    // both are only repairable from inside this section.
    actions: actions.flagged ? "bad" : null,
    triggering: store.startupErrors.length > 0 ? "bad" : null,
    appearance: null,
    defaults: store.models !== null && !store.models.live ? "warn" : null,
  };

  return (
    <nav
      aria-label="Settings"
      className="bg-sidebar flex w-60 min-h-0 flex-none flex-col border-r px-2 py-3"
    >
      <div className="font-display flex items-center gap-2 px-2 pt-1 pb-5 text-sm font-semibold">
        <BrandMark className="text-primary size-5" />
        <span>Beckon</span>
      </div>

      <ul className="flex list-none flex-col gap-0.5 p-0">
        {SECTIONS.map((section) => {
          const Icon = section.icon;
          const active = store.route === section.id;
          return (
            <li key={section.id}>
              <button
                type="button"
                aria-current={active ? "page" : undefined}
                onClick={() => store.go(section.id)}
                className={[
                  "relative flex w-full items-center justify-start gap-2 rounded-md py-2 pr-2 pl-3 text-left",
                  "focus-visible:ring-ring/50 focus-visible:ring-[3px] focus-visible:outline-none",
                  // One signal for "current", one for "under the pointer". A
                  // row that is filled *and* railed says the same thing twice,
                  // and the fill then has nothing left to distinguish it from
                  // hover.
                  active
                    ? "text-foreground font-medium"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                ].join(" ")}
              >
                {/* The brand rail — now the primary marker for "current",
                    which is why it is the one place colour appears here.
                    `aria-current` above carries it for assistive tech. */}
                <span
                  aria-hidden
                  className={`bg-primary absolute inset-y-1.5 left-0 w-0.5 rounded-full transition-opacity ${
                    active ? "opacity-100" : "opacity-0"
                  }`}
                />
                <Icon className="size-3.75 flex-none" />
                <span className="min-w-0 flex-1 truncate">{section.label}</span>
                {flags[section.id] ? (
                  <span
                    title="Something in this section needs attention"
                    className={`size-1.5 flex-none rounded-full ${
                      flags[section.id] === "bad" ? "bg-destructive" : "bg-warning"
                    }`}
                  />
                ) : null}
              </button>
            </li>
          );
        })}
      </ul>

      <div className="mt-auto border-t pt-2">
        <Button
          variant="ghost"
          className="w-full justify-start"
          onClick={() => void revealConfigDir()}
        >
          <FolderIcon className="size-3.5" /> Open folder
        </Button>
      </div>
    </nav>
  );
}
