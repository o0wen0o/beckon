// One entry per section, and Actions is a single item rather than one per file:
// the list belongs in the pane, where a row has room for its Input Source and
// its hotkey, and "pick a section" must not become "pick an Action".
import {
  FolderIcon,
  KeyboardIcon,
  ListIcon,
  PaletteIcon,
  PlugIcon,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { BrandMark } from "@/components/BrandMark";
import { useT } from "@/lib/i18n";
import { revealConfigDir } from "@/lib/ipc";
import { keyProblem } from "@/lib/providers";
import { useStore } from "@/lib/useStore";
import { actionStore } from "./actions";
import { settings, type SectionRoute } from "./store";

/** The order, and the icon each one carries; the words are the catalog's. */
const SECTIONS: { id: SectionRoute; icon: typeof PlugIcon }[] = [
  { id: "connection", icon: PlugIcon },
  { id: "actions", icon: ListIcon },
  { id: "triggering", icon: KeyboardIcon },
  { id: "appearance", icon: PaletteIcon },
];

/**
 * Go to a section, at its **first layer**.
 *
 * Two sections have anything under them — Connection opens one endpoint's own
 * screen, Actions opens one file and then a screen inside that (ADR-0012) — and
 * a nav click resets both, whichever one it lands on. So the nav row is always
 * the way back out, and coming back to a section never resumes a screen the user
 * left minutes ago in another part of the window.
 *
 * All three calls flush, so nothing pending is stranded on the way.
 */
function enter(section: SectionRoute) {
  settings.go(section);
  settings.editProvider(null);
  actionStore.close();
}

export function SettingsNav() {
  const t = useT();
  const store = useStore(settings);
  const actions = useStore(actionStore);

  /** A section is flagged when something inside it needs attention, so a
   *  problem in a pane you are not looking at is still discoverable. A degraded
   *  model list is the warning colour, not red: the dropdown still works. */
  const tests = Object.values(store.test);
  const catalogs = Object.values(store.models);
  // Every row's credential verdict, through the one reader that knows a local
  // endpoint wants no key: any row can be the one an Action posts to (ADR-0021),
  // so any row's problem belongs on this dot.
  const problems = (store.config?.api.providers ?? []).map((provider) =>
    keyProblem(provider, store.keyStatuses[provider.id]),
  );
  const flags: Record<SectionRoute, "bad" | "warn" | null> = {
    // A degraded model list is the warning tone, not red: the dropdown still
    // works, which is why it shares this dot rather than its own section — Model
    // defaults was the pane that used to carry it.
    connection:
      problems.includes("unreadable") || tests.some((test) => test.state === "failed")
        ? "bad"
        : problems.includes("missing") ||
            // Degraded because a fetch *failed*, not because none was attempted:
            // a reveal primes every row's list offline and asks only the row
            // about to be read for its live one, so `!live` on its own is a dot
            // that is always lit — and a warning that is always lit is the line
            // nobody learns to read.
            catalogs.some((catalog) => !catalog.live && catalog.fallback !== null)
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
  };

  return (
    <nav
      aria-label={t.settings.nav.label}
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
          return (
            <li key={section.id}>
              <button
                type="button"
                aria-current={active ? "page" : undefined}
                onClick={() => enter(section.id)}
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
                <span className="min-w-0 flex-1 truncate">{t.settings.nav[section.id]}</span>
                {/* Not drawn on the current row. A 6px dot on the inverted fill
                    is 2.7:1 at best and 1.3:1 for the warning tone — and it is
                    the one row whose problem is already on screen, in the pane
                    to the right. */}
                {flags[section.id] && !active ? (
                  <span
                    title={t.settings.nav.attention}
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
          <FolderIcon className="size-3.5" /> {t.settings.nav.openFolder}
        </Button>
      </div>
    </nav>
  );
}
