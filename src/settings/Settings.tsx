// The shell. It owns the subscriptions and the focus signal, and nothing else:
// every piece of state and every write lives in a store — the global config in
// store.ts, the Actions in actions.ts (ADR-0003) — so a field component cannot
// acquire an opinion about the disk.
import * as React from "react";
import { onActionsChanged, onConfigChanged, onSettingsOpened, Subscriptions } from "@/lib/ipc";
import { PaneProvider } from "@/lib/pane";
import { useStore } from "@/lib/useStore";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { StatusBar } from "@/components/StatusBar";
import { SettingsNav } from "./SettingsNav";
import { actionStore } from "./actions";
import { Actions } from "./sections/actions/Actions";
import { Appearance } from "./sections/Appearance";
import { Connection } from "./sections/Connection";
import { ModelDefaults } from "./sections/ModelDefaults";
import { Triggering } from "./sections/Triggering";
import { settings } from "./store";

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

  // A new section starts at its own top. The pane is one scroll container for
  // every route, so without this, arriving at a short pane from a scrolled long
  // one lands you at whatever offset the previous section happened to be at.
  React.useEffect(() => {
    pane?.scrollTo({ top: 0 });
  }, [pane, store.route]);

  /** Both stores hold a write; whatever moved focus ends both of them. */
  const flush = React.useCallback(() => {
    settings.flush();
    actionStore.flush();
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
    return () => {
      window.removeEventListener("blur", flush);
      void subscriptions.dispose();
    };
  }, [flush]);

  const pendingDelete = actions.pendingDelete;

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
            {/* Keyed on the route so the wrapper remounts and the animation
                actually re-runs. The pane rises 4px as it fades: the nav item
                fills on the same 150–200ms curve, so the click reads as one
                movement from the column into the pane rather than as two
                unrelated repaints. Nothing here waits on it — the content is
                already laid out, only its opacity and offset animate. */}
            <div
              key={store.route}
              className="animate-in fade-in-0 slide-in-from-bottom-1 duration-200 ease-out motion-reduce:animate-none"
            >
              {store.route === "connection" ? (
                <Connection />
              ) : store.route === "actions" ? (
                <Actions />
              ) : store.route === "triggering" ? (
                <Triggering />
              ) : store.route === "appearance" ? (
                <Appearance />
              ) : (
                <ModelDefaults />
              )}
            </div>
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
