// The Launcher: the universal entry point to every Action, and only that.
// Picking is keyboard only and the window dies with its focus, which is what
// keeps authoring in Settings (ADR-0003) — a form inside a picker could not
// survive a click elsewhere.
//
// The window is the query, the ranked list and the keys. One row is
// `ActionRow`; the registry behind it is `actions.ts`.
import * as React from "react";
import { SearchIcon, SlidersHorizontalIcon } from "lucide-react";
import { BrandMark } from "@/components/BrandMark";
import { Kbd } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { rank } from "@/lib/fuzzy";
import {
  hideLauncher,
  onActionsChanged,
  onLauncherOpened,
  pickAction,
  showSettings,
  Subscriptions,
} from "@/lib/ipc";
import { COMMAND_MODIFIER, formatAccelerator, hasCommandModifier } from "@/lib/platform";
import { useStore } from "@/lib/useStore";
import { ActionRow, BrokenRow } from "./ActionRow";
import { actionStore } from "./actions";

export function Launcher() {
  const store = useStore(actionStore);
  const [query, setQuery] = React.useState("");
  // `null` until the user has done something. A summoned window draws no
  // cursor: the ink fill says "Enter runs this", and on a window nobody has
  // touched yet there is no this.
  const [wanted, setWanted] = React.useState<number | null>(null);
  const [selectionChars, setSelectionChars] = React.useState(0);

  const input = React.useRef<HTMLInputElement | null>(null);
  const active = React.useRef<HTMLLIElement | null>(null);

  const snapshot = store.snapshot;
  const matches = React.useMemo(
    () => rank(snapshot.actions, query, (action) => [action.name, action.description ?? ""]),
    [snapshot.actions, query],
  );

  // Clamped rather than reset: the list re-ranks on every keystroke, and a
  // selection that survives the narrowing is what makes typing-then-Enter work.
  // Where the cursor *would* be while it is still hidden is the top match, so
  // Enter has an answer either way.
  const selected = matches.length === 0 ? 0 : Math.min(wanted ?? 0, matches.length - 1);

  const focusQuery = React.useCallback(() => {
    input.current?.focus();
    input.current?.select();
  }, []);

  React.useEffect(() => {
    void actionStore.refresh();
    focusQuery();

    const subscriptions = new Subscriptions();
    subscriptions.add(onActionsChanged((next) => actionStore.adoptActions(next))).add(
      onLauncherOpened((chars) => {
        // A fresh summon starts from a clean slate: the window is reused
        // (ADR-0007), so the last visit's query is still sitting in it.
        setSelectionChars(chars);
        setQuery("");
        setWanted(null);
        focusQuery();
      }),
    );
    return () => void subscriptions.dispose();
  }, [focusQuery]);

  // Arrowing past the fold has to bring the row with it. Scrolling the *row*
  // rather than computing an offset keeps this correct whatever the row height
  // and whether or not the description line is there.
  React.useEffect(() => {
    active.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  // The list and the cursor, written during render for the window-level handler
  // to read. Depending on `matches` / `selected` instead would rebind the
  // handler on every keystroke, on this window's hot path.
  const latest = React.useRef({ matches, selected });
  latest.current = { matches, selected };

  /** Editing is elsewhere. `show_settings` hides this window itself — doing it
   *  from here first would let the foreground go back to the app underneath,
   *  and Settings would open behind it. */
  const openSettings = React.useCallback(() => void showSettings(), []);

  /** The one way to launch, so the mouse and the keyboard cannot diverge — and
   *  the keyboard is the primary path in a hotkey-summoned window. */
  const run = React.useCallback((index: number) => {
    const action = latest.current.matches[index];
    if (action) void pickAction(action.id);
  }, []);

  const move = React.useCallback((delta: number) => {
    const { matches, selected } = latest.current;
    if (matches.length === 0) return;
    // The first arrow reveals the cursor where it already was rather than moving
    // one past it. Only Down needs saying: a hidden cursor is clamped to 0, so
    // the wrap already lands the first Up on the last row.
    setWanted((at) =>
      at === null && delta > 0 ? 0 : (selected + delta + matches.length) % matches.length,
    );
  }, []);

  // On the window, not on the card: clicking a row leaves focus on the body, so
  // a handler bound to the tree stops answering Escape after the first click.
  React.useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      switch (event.key) {
        case "Escape":
          event.preventDefault();
          void hideLauncher();
          return;
        case "ArrowDown":
          event.preventDefault();
          move(1);
          return;
        case "ArrowUp":
          event.preventDefault();
          move(-1);
          return;
        case "Enter":
          event.preventDefault();
          run(latest.current.selected);
          return;
        case "Tab":
          // Focus never leaves the query box, so Tab is just another arrow.
          event.preventDefault();
          move(event.shiftKey ? -1 : 1);
          return;
        case ",":
          if (hasCommandModifier(event)) {
            event.preventDefault();
            openSettings();
          }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [move, run, openSettings]);

  const nothing = matches.length === 0 && snapshot.errors.length === 0;

  return (
    // The frameless card fills the window rect exactly, so the shadow under it
    // is the compositor's — DWM's or the WindowServer's. The radius matches the
    // ~8px Windows 11 rounds an undecorated window at, which is close enough to
    // macOS's own that one value serves both; a larger one is clipped and shows
    // as a nick in each corner.
    <div className="bg-background flex h-screen flex-col overflow-hidden rounded-lg border">
      <div className="flex h-14 flex-none items-center gap-3 border-b px-4">
        <SearchIcon className="text-muted-quiet size-4.5 flex-none" />
        <input
          ref={input}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            // Typing narrows the list, so the top match becomes an answer to
            // Enter — and an answer Enter will act on has to be visible.
            setWanted((at) => at ?? 0);
          }}
          placeholder="Search Actions…"
          spellCheck={false}
          autoComplete="off"
          aria-label="Search Actions"
          className="placeholder:text-muted-foreground min-w-0 flex-1 bg-transparent text-query outline-none"
        />
        <Kbd>Esc</Kbd>
      </div>

      {/* The rows are cards (ADR-0014), so the list is a gutter holding them
          rather than a run of full-bleed rows: `p-1.5` insets every frame from
          the window's own edge, and the gap is what keeps two adjacent frames
          from doubling into one 2px line.

          The gutter is also the window's one ground. A card on the same paper as
          everything around it is only a frame; on a `--muted` well it is a card,
          and the query bar above it stays paper because it is the subject rather
          than the body. The chrome keeps its hairlines and takes no tint of its
          own — a tint plus a border draws one boundary twice. */}
      <ul
        role="listbox"
        aria-label="Actions"
        className="bg-muted flex min-h-0 flex-1 list-none flex-col gap-1.25 overflow-y-auto p-1.5 scrollbar-gutter-stable"
      >
        {snapshot.errors.map((error) => (
          <BrokenRow
            key={error.file_name}
            file={error.file_name}
            message={error.message}
            onOpen={openSettings}
          />
        ))}

        {matches.map((action, index) => (
          <ActionRow
            key={action.id}
            ref={index === selected ? active : undefined}
            action={action}
            conflict={snapshot.hotkey_errors[action.id]}
            query={query}
            selected={wanted !== null && index === selected}
            onRun={() => run(index)}
          />
        ))}

        {/* Centred in what is left of the window rather than parked under the
            query box: the list is the window's whole body, so an empty one that
            hugs the top leaves a void that reads as content failing to load. */}
        {nothing ? (
          <li className="flex flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
            {snapshot.actions.length === 0 ? (
              <>
                <BrandMark className="text-brand size-6" />
                <p className="text-muted-foreground text-quiet">No Actions yet.</p>
                <Button size="sm" onClick={openSettings}>
                  <SlidersHorizontalIcon /> Add one in Settings
                </Button>
              </>
            ) : (
              <>
                <p className="text-muted-foreground text-quiet">
                  {/* Bare mono, the way every other `code` in the product is
                      set: a box around it is `Kbd`'s job, and a second
                      slightly-off version of it reads as a key to press. */}
                  Nothing matches <code className="text-foreground font-mono">{query}</code>.
                </p>
                <p className="text-muted-quiet text-note">Backspace to widen the search.</p>
              </>
            )}
          </li>
        ) : null}
      </ul>

      {/* The Selection count is the one thing this window knows that the list
          cannot show, and the keys are worth the space because the window is
          keyboard-first. The legend goes when there is nothing to move through. */}
      <div className="text-muted-quiet flex h-8 flex-none items-center gap-3 border-t pr-1 pl-4 text-meta">
        <span className="flex-1 truncate">
          {selectionChars > 0 ? `${selectionChars} characters selected` : "No selection"}
        </span>
        {matches.length > 0 ? (
          <span className="flex flex-none items-center gap-1" aria-hidden="true">
            <Kbd>↑</Kbd>
            <Kbd>↓</Kbd> move <Kbd>↵</Kbd> run
          </span>
        ) : null}
        <Button
          variant="ghost"
          size="icon-sm"
          title={`Settings (${formatAccelerator(`${COMMAND_MODIFIER}+,`)})`}
          aria-label="Settings"
          onClick={openSettings}
        >
          <SlidersHorizontalIcon className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}
