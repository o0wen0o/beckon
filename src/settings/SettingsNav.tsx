// One entry per section, and Actions is a single item rather than one per file:
// the list belongs in the pane, where a row has room for its Input Source and
// its hotkey, and "pick a section" must not become "pick an Action".
import {
  FolderIcon,
  KeyboardIcon,
  ListIcon,
  PaletteIcon,
  PlugIcon,
  SlidersIcon,
} from "lucide-react";
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
    // A denied Accessibility permission belongs here for the same reason a
    // hotkey that would not register does: the hotkey fires and nothing
    // happens, and this is the only pane that explains why (ADR-0013).
    triggering:
      store.startupErrors.length > 0 || store.inputPermission === "denied" ? "bad" : null,
    appearance: null,
    defaults: store.models !== null && !store.models.live ? "warn" : null,
  };

  return (
    <nav
      aria-label="Settings"
      className="bg-sidebar flex w-52 min-h-0 flex-none flex-col border-r px-2.5 py-3.5"
    >
      <div className="font-display flex items-center gap-2 px-2 pt-0.5 pb-4.5 text-sm font-semibold">
        {/* The one chromatic thing in the window, and the reason `--brand` is a
            token of its own: everything else here is ink on paper. */}
        <BrandMark className="text-brand size-4.25" />
        <span>Beckon</span>
      </div>

      <ul className="flex list-none flex-col gap-0.5 p-0">
        {SECTIONS.map((section) => {
          const Icon = section.icon;
          const active = store.route === section.id;
          // Actions is the one section with anything under it: a file open in
          // the editor, and a screen inside that (ADR-0012). Its nav row is the
          // way back out, so clicking the current row closes the editor instead
          // of doing nothing. `close` flushes, so a pending edit is written.
          const closesEditor = active && section.id === "actions" && actions.editing !== null;
          return (
            <li key={section.id}>
              <button
                type="button"
                aria-current={active ? "page" : undefined}
                onClick={() => (closesEditor ? actionStore.close() : store.go(section.id))}
                className={[
                  "flex w-full items-center justify-start gap-2.25 rounded-md px-2.25 py-1.5 text-left",
                  "focus-visible:ring-ring/50 focus-visible:ring-[3px] focus-visible:outline-none",
                  // The fill arrives rather than appearing, on the same 150ms
                  // curve the pane behind it swaps on.
                  "transition-colors duration-150 ease-out motion-reduce:transition-none",
                  // Inversion is the marker: the current row is the only filled
                  // thing in the window, so hover is free to mean "under the
                  // pointer" and nothing else.
                  active
                    ? "bg-primary text-primary-foreground font-medium"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                ].join(" ")}
              >
                {/* An inherited colour change only animates where the child
                    declares the transition too, or the glyph snaps. */}
                <Icon className="size-3.75 flex-none transition-colors duration-150 ease-out motion-reduce:transition-none" />
                <span className="min-w-0 flex-1 truncate">{section.label}</span>
                {/* Not drawn on the current row. A 6px dot on the inverted fill
                    is 2.7:1 at best and 1.3:1 for the warning tone — and it is
                    the one row whose problem is already on screen, in the pane
                    to the right. */}
                {flags[section.id] && !active ? (
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
