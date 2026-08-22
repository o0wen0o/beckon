---
status: accepted
---

# Grab the Selection eagerly at hotkey press, before showing any window

Both trigger paths grab the Selection as the *first* thing they do, before the Launcher or the
Popover is shown. The Launcher path stashes the result and hands it to whichever Action the user
then picks.

The reason is that [ADR-0002](./0002-selection-via-simulated-ctrl-c.md) grabs text by sending
Ctrl+C to the **foreground window**. The moment one of our windows is shown it takes focus — the
Popover is specified to, and a search box the user types into obviously must — so from that instant
the foreground window is Beckon. A Ctrl+C sent then copies from our own empty window, or from
nothing at all.

## Considered Options

**Show the Launcher first, then grab after the pick** is the arrangement that would let us skip the
grab entirely for `prompt`-only Actions. It requires restoring focus to the original window, waiting
for that to take effect, and only then sending Ctrl+C — reintroducing exactly the focus race this
ordering exists to avoid, and one that fails differently per application. Rejected.

**Grab only for the Direct Hotkey path, and treat the Launcher as prompt-only** was rejected because
it silently breaks the most useful combination there is: select text, summon the Launcher, pick
"Translate".

## Consequences

- The clipboard round-trips even when the user goes on to pick an Action with
  `input_source = "prompt"`, which ignores the Selection. Restoration makes this invisible, but it is
  a real cost: roughly one clipboard write plus a sequence-number poll of up to 300ms per trigger.
- The grab runs on a worker thread, never on the thread that pumps events: it polls for up to 300ms
  and would otherwise stall the UI it is about to show.
- The cached Selection is deliberately short-lived. It is dropped when the Launcher hides, when a
  pick consumes it, and it is never written anywhere — the same "no extra retained copy" rule
  ADR-0002 sets for the clipboard backup.
- `input_source` resolution has to happen *after* the pick, because until then we do not know
  whether the grab is even wanted. That ordering is also the reason `prompt` survived the trim in
  [ADR-0020](0020-the-input-source-loses-its-selection-only-arm.md): a Selection is always in hand by
  the time an Action is known, so ignoring it has to be something an Action can declare. The
  resolution therefore lives in Rust next to the trigger flow, not in the window that renders the
  result.
