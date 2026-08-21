// The Popover window: a title bar, a scrolling conversation, and a composer when
// there is something to type into. The states live in `exchange.ts`; what is
// left here is the shell, the scroller the store cannot reach, and the keys that
// belong to the window rather than to a field.
import * as React from "react";
import { hidePopover, Subscriptions } from "@/lib/ipc";
import { useStore } from "@/lib/useStore";
import { Callout } from "@/components/Callout";
import { Kbd } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { Composer } from "./Composer";
import { PopoverHeader } from "./PopoverHeader";
import { TurnView } from "./Turn";
import { exchange } from "./exchange";

/** How far off the bottom counts as "the user has scrolled up to re-read". */
const STICK_WITHIN = 48;

export function Popover() {
  const store = useStore(exchange);
  const scroller = React.useRef<HTMLDivElement | null>(null);
  // Whether the stream should still be followed. Written on scroll rather than
  // measured after a delta lands: by then the distance to the bottom has grown
  // by whatever just arrived, and a large delta reads as "scrolled up".
  const stick = React.useRef(true);

  React.useEffect(() => {
    void exchange.load();
    const subscriptions = new Subscriptions();
    exchange.listen(subscriptions);
    const stopClock = exchange.startClock();
    return () => {
      stopClock();
      void subscriptions.dispose();
    };
  }, []);

  const answer = store.current?.answer;
  React.useEffect(() => {
    if (!stick.current || !scroller.current) return;
    scroller.current.scrollTop = scroller.current.scrollHeight;
  }, [answer]);

  // A new trigger is a new conversation in a window that was never destroyed
  // (ADR-0007), so it starts at the top and follows again.
  React.useEffect(() => {
    stick.current = true;
    if (scroller.current) scroller.current.scrollTop = 0;
  }, [store.epoch]);

  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        // Esc cancels a live request first, so partial text stays readable;
        // a second Esc closes the window (README: both behaviours).
        if (!exchange.cancel()) void hidePopover();
        return;
      }
      // Copy is the only export path, so it gets a shortcut that works while
      // the composer has focus.
      if (event.key.toLowerCase() === "c" && event.ctrlKey && event.shiftKey) {
        const current = exchange.current;
        if (!current?.answer) return;
        event.preventDefault();
        void exchange.copy(current.answer, exchange.turns.length - 1);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const view = store.view;
  const empty = store.turns.length === 0;

  return (
    // The frameless card fills the window rect exactly, so the drop shadow
    // under it is DWM's rather than one painted here.
    <div className="bg-background flex h-screen flex-col overflow-hidden rounded-lg border">
      <PopoverHeader
        actionName={view?.action_name ?? "Beckon"}
        model={view?.model ?? null}
        onClose={() => void hidePopover()}
      />

      <div
        ref={scroller}
        onScroll={(event) => {
          const box = event.currentTarget;
          stick.current = box.scrollHeight - box.scrollTop - box.clientHeight <= STICK_WITHIN;
        }}
        // The turns are sided rather than ruled, so the gap is what separates
        // them and the scroller carries the window's own padding.
        className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-3.5 py-3 scrollbar-gutter-stable"
      >
        {/* Which notice this is comes from the store; a notice has no side to
            sit on. Only one of them is an alarm, and it gets the marker the rest
            of the product uses for one — a rule, not a box; the other two are
            ordinary prose. */}
        {store.notice === "no-view" ? (
          <p className="text-muted-foreground">Nothing to show.</p>
        ) : store.notice === "empty-selection" ? (
          // `mb-0`: the margin baked into a callout is the ledger's rhythm
          // below one, and this scroller spaces its children with a gap.
          <Callout tone="warn" className="mb-0">
            <p>
              <strong>{view?.action_name}</strong> works on a Selection, and nothing was selected.
            </p>
            <p>Select some text and press the hotkey again. Elevated windows cannot be read at all.</p>
          </Callout>
        ) : store.notice === "awaiting-input" ? (
          // Body size and a bold name, matching `Callout`: all three notices
          // land in the same slot, so a step of difference between them says
          // nothing a reader can act on.
          <p className="text-muted-foreground">
            Type what you want to send to{" "}
            <strong className="text-foreground font-bold">{view?.action_name}</strong>.
          </p>
        ) : null}

        {store.turns.map((turn, index) => (
          <TurnView key={index} turn={turn} index={index} />
        ))}
      </div>

      {/* Esc cancels a live request, and nothing said so: the window offered
          only a close button, so the choice between "stop this" and "throw it
          away" was invisible unless you already knew the shortcut. */}
      {store.busy ? (
        <div className="flex h-11 flex-none items-center gap-2 border-t pr-2 pl-3.5">
          <span className="text-muted-quiet flex flex-1 items-center gap-2 text-meta">
            <span className="bg-foreground size-1.5 flex-none animate-pulse rounded-full motion-reduce:animate-none" />
            {store.runLabel}
          </span>
          <Button variant="outline" size="sm" onClick={() => exchange.cancel()}>
            Stop <Kbd className="bg-transparent">Esc</Kbd>
          </Button>
        </div>
      ) : null}

      {store.composing ? (
        <Composer
          // A fresh element per trigger: the draft and the grown height are the
          // browser's, and remounting is what clears both (ADR-0007).
          key={store.epoch}
          placeholder={empty ? "Your input…" : "Ask a follow-up…"}
          onSend={(text) => void exchange.send(text)}
        />
      ) : null}
    </div>
  );
}
