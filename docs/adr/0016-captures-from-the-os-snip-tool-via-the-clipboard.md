---
status: accepted
---

# A Capture is a screenshot from the OS snip tool, attached to a turn and sent as a data URL

The Popover has a screenshot button. Pressing it hides Beckon's windows, runs the *platform's own*
interactive snip tool — `ms-screenclip:` on Windows, `screencapture -i -c` on macOS — and comes back
with whatever landed on the clipboard attached to the composer as a **Capture**. Send then goes to
the model with the image and whatever was typed beside it, in one user message.

That is the whole feature, and every decision below is about one of two things: where the bytes
come from, and when they are sent. What happens when the model cannot read them is deliberately not
Beckon's question — see below.

## The snip tool is the OS's, not ours

Drawing our own selection overlay means a full-screen transparent window per monitor, DPI-correct
hit-testing, and a second thing to keep working across two platforms. The OS already ships one, the
user already knows its keystroke, and its output is already on the clipboard.

The consequence is that the two platforms answer differently, and the difference is not cosmetic:

- **macOS** runs `screencapture` as a child process, so the wait ends when the tool exits and a
  cancel is a non-zero exit *and* an unmoved `changeCount`. Both are checked, because only the
  second is guaranteed.
- **Windows** fires a shell verb. `explorer.exe ms-screenclip:` returns the moment the tool is
  launched and reports nothing afterwards — not success, not Esc. So the Windows half polls
  `GetClipboardSequenceNumber` exactly the way the Selection grab does, with a 45-second cap
  ([ADR-0002](./0002-selection-via-simulated-ctrl-c.md) uses 300ms). A cancelled snip is
  indistinguishable from a slow one until that cap runs out, and the Popover then says only that
  nothing was captured.

## The clipboard is *not* backed up here

ADR-0002 backs the clipboard up and restores it, because it synthesises a keystroke the user did not
press. Nothing is synthesised here: the user ran a screenshot tool, and a screenshot tool putting an
image on the clipboard is the behaviour they asked for. Restoring over it would be Beckon undoing
something the user did.

## Attach, then send — not capture-and-send

The button attaches; **Send** sends. It costs one extra keystroke against going straight to the
model, and it buys the two things a snip needs and a Selection does not:

- a mis-dragged rectangle is recoverable. There is no way to check a region before it is captured,
  and a spent request is not a free mistake;
- the note. "What is wrong here?" beside the image is most of why the image is being sent, and there
  is nowhere to type it if the request has already gone.

The button therefore lives in the composer rather than the title bar
([ADR-0014](./0014-launcher-rows-are-cards.md) is the same argument about registers): the title bar
is the drag region and holds no verbs, and a Capture is something you attach to what you are about
to send.

Consequences of "it is part of the turn":

- an attached Capture is consumed when the turn starts, so a follow-up does not silently resend it;
- it still travels in the Exchange's history like any other content, so the follow-up *can* refer to
  it (ADR-0004 resends the history untruncated — an image is ≤384 tokens, so this is cheap);
- a **retry** replays the exact message that failed, image included, rather than re-rendering the
  turn against whatever is attached now.

## Nothing is stored, so the wire format is a data URL

[ADR-0004](./0004-exchanges-are-never-persisted.md) says an Exchange is never persisted, and that
applies to its images: a Capture exists in `PopoverView` and in the Exchange's message list, both of
which die with the window. It therefore has no URL of its own, which leaves the `data:` URL form of
`image_url` as the only one available — the provider also accepts an external URL and a Files API
handle, and both would mean uploading the user's screen to something that keeps it.

One consequence is worth naming: the same base64 string is what the request sends *and* what the
Popover draws its thumbnail from. One copy, so the picture on screen cannot differ from the picture
that was sent.

Beckon caps an encoded Capture at 8 MiB. The provider allows 32 MiB per image inside a 48 MiB body,
so this is deliberately the *stricter* limit, and it exists so that a snip of a huge display is
refused as a sentence the reader can act on ("capture a smaller region") rather than as a 413.

## Every model is sent the image; the endpoint decides

Beckon does not check whether the chosen model reads images. A Capture is attached to the request
for any model, and whatever the endpoint answers — a description, or a refusal — is the answer the
user sees.

The alternative was a `vision` column in `llm/models.rs` beside `thinking`, refusing an image bound
for a model the table says takes none. It was tried and removed. `thinking` can carry a table
because it is a *wire-format* question: getting it wrong sends a field the provider silently
misreads, and the failure is invisible. Images are not that. Sending one to a model that cannot read
it produces an explicit, readable error from the endpoint itself, so there is nothing invisible to
protect the user from — and `base_url` is configurable, so the table would have to stay true for
every provider Beckon may be pointed at, which is not a promise a hand-kept list can keep. A table
that is wrong about a model refuses a request that would have worked; that is worse than passing the
question to the only party who knows the answer.

`deepseek-v4-flash-vision-exp` is still catalogued, because the dropdown should offer DeepSeek's
image-reading model by name. It is experimental and its `thinking` support is undocumented, so it is
catalogued as `Thinking::Never`: `thinking = true` for it is refused out loud instead of hopefully
sent.

## Considered Options

**A fourth `input_source`, `capture`,** so an Action could declare that it works on a screenshot the
way `selection` declares it works on the Selection. It is the right shape for "Explain this error
dialog" as a Direct Hotkey, and it is not this ADR: it would need the Launcher to say which Actions
take images, and a snip cannot run before the Popover exists without the window flashing. Deferred,
not rejected — `input_source` is where it would go.

**Capture-and-send from the title bar** was the shorter path and is rejected above.

**Our own selection overlay** was rejected above.

## Consequences

- `Message.content` is no longer a `String`. It is `Content::Text | Content::Parts`, serialised
  `untagged` so a text-only message still goes on the wire as a bare string — a `base_url` pointing
  at an endpoint that predates content parts keeps working.
- The snip needs the Popover *hidden*, and hiding it must not be `hide_popover`: that discards the
  Exchange (ADR-0004), and a conversation has to survive a screenshot taken in the middle of it. So
  `start_capture` hides the window only, and no one hands the foreground back — the snip tool takes
  the screen and we are coming straight back.
- The attached Capture arrives as its own event, `popover:capture`, not as `popover:view`.
  Re-reading the view is how a *new trigger* is handled: it resets the conversation and remounts the
  composer (ADR-0007), which would throw away the half-typed note the screenshot was taken for.
- A Capture is only reachable where the composer is — `needs-input`, or a settled turn. The
  `empty-selection` Popover is a two-line hint with no composer and stays that way; it is sized to
  never grow ([`POPOVER_HINT_H`](../../src-tauri/src/trigger/window.rs)).
- Two new dependencies, both narrow: `base64`, and `image` with `png`/`bmp`/`tiff` only — the
  clipboard hands over a BMP on Windows and a TIFF on macOS, and neither is a format the provider
  takes.
- Beckon knows nothing about which models read images, so `llm::models::CatalogEntry` has no
  `vision` column and `deepseek::build_body` has no image guard. A Capture sent to a text-only model
  surfaces the provider's own error through the existing failure path, in its words rather than
  ours.
- Nothing here can be checked by a compiler. The snip tools, the clipboard formats and the
  re-focusing are in [docs/macos-testing.md](../macos-testing.md) as manual steps, on both
  platforms.
