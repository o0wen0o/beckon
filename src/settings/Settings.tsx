// The shell: the subscriptions and the focus signal, and nothing else. Every
// piece of state and every write lives in a store — the global config in
// store.ts, the Actions in actions.ts (ADR-0003).
import * as React from "react";
import { onActionsChanged, onConfigChanged, onSettingsOpened, Subscriptions } from "@/lib/ipc";
import { PaneProvider } from "@/lib/pane";
import { useStore } from "@/lib/useStore";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { PaneEnter } from "@/components/PaneEnter";
import { StatusBar } from "@/components/StatusBar";
import { SettingsNav } from "./SettingsNav";
import { actionStore } from "./actions";
import { Actions } from "./sections/actions/Actions";
import { Appearance } from "./sections/Appearance";
import { Connection } from "./sections/Connection";
import { ModelDefaults } from "./sections/ModelDefaults";
import { Triggering } from "./sections/Triggering";
import { settings, type SectionRoute } from "./store";

/** One pane per route, as a lookup rather than a ternary chain: the mapped type
 *  is what makes a section added later a compile error here instead of silently
 *  rendering whatever the chain's last `else` happened to be. */
const PANES: Record<SectionRoute, React.ComponentType> = {
  connection: Connection,
  actions: Actions,
  triggering: Triggering,
  appearance: Appearance,
  defaults: ModelDefaults,
};

export function Settings() {
  const store = useStore(settings);
  const actions = useStore(actionStore);

  // The pane element, published two ways: the stores test focus against it, and
  // Radix portals its overlays into it (src/lib/pane.tsx). State rather than a
  // bare ref, so the portals re-render once it exists.
  const [pane, setPane] = React.useState<HTMLElement | null>(null);

  React.useEffect(() => {
    settings.pane = pane;
  }, [pane]);

  // A new section starts at its own top: the pane is one scroll container for
  // every route, so otherwise a short pane inherits the last one's offset.
  React.useEffect(() => {
    pane?.scrollTo({ top: 0 });
  }, [pane, store.route]);

  /** Both stores hold a write; whatever moved focus ends both of them. */
  const flush = React.useCallback(() => {
    settings.flush();
    actionStore.flush();
  }, []);

  const recheckPermission = React.useCallback(() => {
    void settings.refreshInputPermission();
  }, []);

  React.useEffect(() => {
    // The first open builds the window, so `settings:opened` fires before
    // anything is listening — this component always loads itself.
    void settings.refreshAll();
    void actionStore.refresh();

    const subscriptions = new Subscriptions();
    subscriptions
      .add(
        onConfigChanged((next) => {
          settings.adoptConfig(next);
          void settings.refreshStartupErrors();
        }),
      )
      .add(
        onActionsChanged((next) => {
          actionStore.adoptActions(next);
          // An Action's Direct Hotkey is what Triggering reports as a startup
          // error, so the two sections move together.
          void settings.refreshStartupErrors();
        }),
      )
      .add(
        onSettingsOpened(() => {
          settings.resetTransient();
          // The window is reused (ADR-0007), so a fresh open must not resume
          // whatever Action the last visit left open in the editor.
          actionStore.close();
          void settings.refreshAll();
          void actionStore.refresh();
        }),
      );

    // The window can be hidden mid-edit; a blur is the last chance to write.
    window.addEventListener("blur", flush);
    // Coming back is the signal that the Accessibility switch may have moved:
    // it is thrown in System Settings, and nothing tells us when (ADR-0013).
    window.addEventListener("focus", recheckPermission);
    return () => {
      window.removeEventListener("blur", flush);
      window.removeEventListener("focus", recheckPermission);
      void subscriptions.dispose();
    };
  }, [flush, recheckPermission]);

  const pendingDelete = actions.pendingDelete;
  const Pane = PANES[store.route];
  /** What is on screen, which is the route except inside Actions, where opening
   *  a file — and then one of that Action's screens (ADR-0012) — is a view
   *  change the route does not see. */
  const editing = actions.editing;
  const paneKey = [
    store.route,
    editing?.file ?? "",
    editing?.kind === "action" ? editing.screen : "",
  ].join(":");

  return (
    <PaneProvider value={pane}>
      <div className="flex h-screen flex-col">
        <div className="flex min-h-0 flex-1">
          <SettingsNav />

          {/* The navigation lives outside the pane on purpose: clicking a nav
              item moves focus out of it, which fires the blur that flushes the
              save slot. Changing section can never strand an unwritten edit. */}
          <main
            ref={setPane}
            onBlur={flush}
            className="min-w-0 flex-1 overflow-y-auto px-7.5 pt-6.5 pb-10"
          >
            {/* Keyed on the whole view, not just the route: the Actions section
                swaps between its list and its editor without the route moving,
                and the pane rising 4px as it fades is what makes that read as
                one movement from the nav column into the pane. Keying it here
                rather than inside the section is what keeps the entrance to one
                per change — as a wrapper in both places, arriving at Actions ran
                the two of them at once. */}
            <PaneEnter key={paneKey}>
              <Pane />
            </PaneEnter>
          </main>
        </div>

        <StatusBar
          busy={store.configSlot.busy || actions.slot.busy}
          error={store.saveError ?? actions.slot.error}
          note={
            actions.editing?.kind === "raw"
              ? "This file does not parse, so it is written with the button above — not as you type."
              : null
          }
        />
      </div>

      {/* Hosted here, not in the editor, and portalled to the body rather than
          into the pane: confirming deletes the Action, which unmounts the
          editor, and a dialog whose own container disappears goes with it. */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title={`Delete “${pendingDelete?.name || pendingDelete?.file_name}”?`}
        confirmLabel="Delete file"
        destructive
        onConfirm={() => pendingDelete && void actionStore.deleteAction(pendingDelete)}
        onCancel={() => actionStore.askDelete(null)}
      >
        <p>
          The file <code className="font-mono">{pendingDelete?.file_name}</code> is removed from
          disk. This cannot be undone.
        </p>
      </ConfirmDialog>
    </PaneProvider>
  );
}
