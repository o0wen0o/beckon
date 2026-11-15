// The Popover window: a title bar, a scrolling conversation, and a composer when
// there is something to type into. The states live in `exchange.ts`; what is
// left here is the shell, the scroller the store cannot reach, and the keys that
// belong to the window rather than to a field.
import * as React from "react";
import {
  hidePopover,
  onWindowResized,
  setPopoverSize,
  startCapture,
  Subscriptions,
} from "@/lib/ipc";
import { fill, useT } from "@/lib/i18n";
import { hasCommandModifier, isComposing } from "@/lib/platform";
import { useStore } from "@/lib/useStore";
import { Kbd } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { CapturePreview } from "./CapturePreview";
import { Composer } from "./Composer";
import { PopoverHeader } from "./PopoverHeader";
import { ResizeGrips } from "./ResizeGrips";
import { TurnView } from "./Turn";
import { exchange, isSettled } from "./exchange";

/** How far off the bottom counts as "the user has scrolled up to re-read". */
const STICK_WITHIN = 48;

/** How long a resize has to stop for before it counts as the size the user
 *  meant. A drag reports every pixel, and each report Rust keeps is a
 *  write to `config.toml`. */
const SIZE_SETTLE = 400;

export function Popover() {
  const t = useT();
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

  // The size the window is dragged to is remembered across summons.
  // Every resize reports itself, ours included; Rust drops the ones it caused,
  // so this reports all of them and decides nothing. The report is physical
  // pixels straight off the event — `setPopoverSize` converts, once per settled
  // drag rather than once per frame.
  React.useEffect(() => {
    let settle: number | undefined;
    const subscriptions = new Subscriptions().add(
      onWindowResized((width, height) => {
        window.clearTimeout(settle);
        settle = window.setTimeout(() => void setPopoverSize(width, height), SIZE_SETTLE);
      }),
    );
    return () => {
      window.clearTimeout(settle);
      void subscriptions.dispose();
    };
  }, []);

  const current = store.current;
  const answer = current?.answer;
  const reasoning = current?.reasoning;
  const status = current?.status;
  const turnCount = store.turns.length;

  // A turn that has just been asked for is followed, whatever was being
  // re-read: the user scrolled up, typed a follow-up and sent it, and the
  // question they just asked — and its answer — arrived below the fold of a
  // window that was no longer following. Retry is the same gesture.
  React.useEffect(() => {
    if (status === "waiting-first-token") stick.current = true;
  }, [status, turnCount]);

  // Everything that moves the bottom of the conversation, not the answer
  // alone. When a long answer settles, the Stop bar gives way to a taller
  // composer and the row of Copy, Insert and Regenerate is added under the
  // text — some forty pixels that used to land below the fold, so the buttons
  // an answer is used *with* were hidden at the end of every long one.
  React.useEffect(() => {
    if (!stick.current || !scroller.current) return;
    scroller.current.scrollTop = scroller.current.scrollHeight;
  }, [answer, reasoning, status, turnCount, store.composing, store.busy]);

  // A new trigger is a new conversation in a window that was never destroyed,
  // so it starts at the top and follows again.
  React.useEffect(() => {
    stick.current = true;
    if (scroller.current) scroller.current.scrollTop = 0;
  }, [store.epoch]);

  React.useEffect(() => {
    /** Whether the Insert chord went down in this window and has not come up. */
    let insertArmed = false;
    /** The last turn's answer, once it has stopped changing — or nothing. */
    const insertable = () => {
      const current = exchange.current;
      return current?.answer && isSettled(current.status) ? current.answer : null;
    };

    const onKeyDown = (event: KeyboardEvent) => {
      // A key the input method is still using is not the window's: the Escape
      // that abandons a half-typed composition must not also stop the request
      // or close the Popover over the draft.
      if (isComposing(event)) return;
      if (event.key === "Escape") {
        event.preventDefault();
        // Esc closes the preview first: it is the only layer over the
        // window, so it is the nearest thing to back out of — and closing it
        // must not also cancel the request underneath it.
        if (exchange.closePreview()) return;
        // Esc cancels a live request first, so partial text stays readable;
        // a second Esc closes the window (README: both behaviours).
        if (!exchange.cancel()) void hidePopover();
        return;
      }
      // The arrows walk the preview only while it is up. Unguarded they would
      // be the caret's keys in the composer, which is the field below it.
      if (exchange.preview && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
        event.preventDefault();
        exchange.stepPreview(event.key === "ArrowRight" ? 1 : -1);
        return;
      }
      // The conversation, from the keyboard. Focus lives in the composer —
      // the window is summoned by a hotkey and typed into — so the scroller
      // never has it, and a long answer could be read on only with the wheel.
      if ((event.key === "PageDown" || event.key === "PageUp") && !exchange.preview) {
        const box = scroller.current;
        if (!box) return;
        event.preventDefault();
        // Most of a page, so the last lines read stay on screen as the first.
        const page = Math.max(40, box.clientHeight - 48);
        box.scrollBy({ top: event.key === "PageDown" ? page : -page });
        return;
      }
      /** Every window shortcut is the same chord with one key changed. */
      const chord = (key: string) =>
        event.key.toLowerCase() === key && hasCommandModifier(event) && event.shiftKey;
      // The screenshot shortcut, so a Popover summoned by a hotkey can grab one
      // without the mouse. Window-scoped: it is not registered
      // globally and means nothing when the Popover is not up.
      if (chord("s")) {
        event.preventDefault();
        void startCapture();
        return;
      }
      // Insert: the answer goes back to the app Beckon was summoned
      // over and this window closes — so a Direct Hotkey, a read, and this chord
      // is a rewrite done without the mouse. Enter rather than V: this chord
      // with V is "paste as plain text" in the composer, and taking a text
      // field's own key away is how a shortcut gets resented. Only a settled
      // answer — half a stream pasted into a document is not what was asked for.
      //
      // Armed here and fired on the way up (`onKeyUp`). Insert hands the
      // foreground to another app while this key is still physically down, and
      // a key held past the repeat delay then repeats *into that app* — as a
      // bare Enter, because Rust has released the modifiers by then. In a chat
      // window a bare Enter is Send. Waiting for the release costs nothing a
      // person can feel and means the only keystroke the target ever sees is
      // the paste.
      if (chord("enter")) {
        if (!insertable()) return;
        event.preventDefault();
        insertArmed = true;
        return;
      }
      // Run the last turn again: Regenerate on a settled one, Retry on a failed
      // one — the same request either way. Swallowed even when there is nothing
      // to rerun, because unswallowed this chord is the webview's hard reload.
      if (chord("r")) {
        event.preventDefault();
        if (exchange.canRetry) void exchange.retry();
        return;
      }
      // Copy gets a shortcut that works while the composer has focus.
      if (chord("c")) {
        const current = exchange.current;
        if (!current?.answer) return;
        event.preventDefault();
        void exchange.copy(current.answer, exchange.turns.length - 1);
      }
    };

    // The release that fires an armed Insert. The modifiers are not asked
    // about: whichever key of the chord comes up first, the gesture was made
    // on the way down, and Rust releases what is still held.
    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key !== "Enter" || !insertArmed) return;
      insertArmed = false;
      // Asked again rather than remembered: a new trigger can have replaced
      // the conversation between the two halves of one key press.
      const answer = insertable();
      if (answer) void exchange.insert(answer, exchange.turns.length - 1);
    };
    // A release this window never sees — focus went elsewhere mid-press —
    // must not leave the next plain Enter armed.
    const disarm = () => {
      insertArmed = false;
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", disarm);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", disarm);
    };
  }, []);

  const view = store.view;
  const empty = store.turns.length === 0;

  return (
    // The frameless card fills the window rect exactly, so the drop shadow
    // under it is the compositor's rather than one painted here. `relative` is
    // for the preview layer: it covers this card, not the viewport, so the
    // rounded corners stay the window's own.
    <div className="bg-background relative flex h-screen flex-col overflow-hidden rounded-lg border">
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
            sit on. Neither is an alarm: an empty grab lands in the composer.
            They share one element rather than two matching ones, so "the same slot,
            with no step of difference between them" is structural instead of two
            class strings kept equal by hand. */}
        {store.notice === "none" ? null : (
          <p className="text-muted-foreground">
            {store.notice === "no-view"
              ? t.popover.nothingToShow
              : fill(t.popover.typeYourInput, {
                  name: <strong className="text-foreground font-bold">{view?.action_name}</strong>,
                })}
          </p>
        )}

        {store.turns.map((turn, index) => (
          <TurnView
            key={index}
            turn={turn}
            index={index}
            last={index === store.turns.length - 1}
          />
        ))}
      </div>

      {/* Esc cancels a live request, and nothing said so: the window offered
          only a close button, so the choice between "stop this" and "throw it
          away" was invisible unless you already knew the shortcut. */}
      {store.busy ? (
        <div className="flex h-11 flex-none items-center gap-2 border-t pr-2 pl-3.5">
          <span className="text-muted-quiet flex flex-1 items-center gap-2 text-meta">
            <span className="bg-foreground size-1.5 flex-none animate-pulse rounded-full motion-reduce:animate-none" />
            {store.streaming ? t.popover.runningStreaming : t.popover.runningWaiting}
          </span>
          <Button variant="outline" size="sm" onClick={() => exchange.cancel()}>
            {t.popover.stop} <Kbd className="bg-transparent">{t.launcher.escape}</Kbd>
          </Button>
        </div>
      ) : null}

      {store.composing ? (
        <Composer
          // A fresh element per trigger: the draft and the grown height are the
          // browser's, and remounting is what clears both.
          // A Capture arriving is *not* a new trigger — it lands on
          // `popover:capture` precisely so the draft it belongs to survives.
          key={store.epoch}
          placeholder={empty ? t.popover.firstInput : t.popover.followUp}
          captures={store.captures}
          captureNotice={store.captureNotice}
          captureRun={store.captureRun}
          onSend={(text) => void exchange.send(text)}
          onCapture={() => void startCapture()}
          onOpenCapture={(index) => exchange.openPreview("composer", index)}
          onDiscardCapture={(index) => exchange.discardCapture(index)}
        />
      ) : null}

      {/* Last child and `absolute inset-0`, so it covers the title bar as well:
          a preview under the drag region would leave the window's one grab
          handle sitting on top of the picture. */}
      {store.preview ? (
        <CapturePreview
          items={store.preview.items}
          index={store.preview.index}
          onStep={(by) => exchange.stepPreview(by)}
          onClose={() => exchange.closePreview()}
        />
      ) : null}

      {/* After the preview, and above it: the window stays resizable while a
          screenshot is being looked at, which is when its size matters most.
          */}
      <ResizeGrips />
    </div>
  );
}
