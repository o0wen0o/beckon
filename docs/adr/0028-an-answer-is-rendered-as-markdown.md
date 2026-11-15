---
status: accepted
---

# An answer is rendered as Markdown, and only the answer

The Popover drew an answer as one `<p>` with preserved whitespace. `Turn.tsx` said why in its own
header: "acceptable for the MVP, and it cannot inject anything into the WebView". The second half of
that sentence is a requirement. The first half stopped being true the day an Action asked for
anything but a translation.

Every model Beckon can reach writes Markdown unprompted. As plain text a fenced block is three
backticks above code that wraps mid-identifier, a table is rows of pipes, a list of steps is a column
of hyphens, and `**the one thing that matters**` arrives wearing its asterisks. The answer is the only
prose in the product and the reason the window exists, and it was the one surface drawn as source.

**So the answer is parsed and drawn as Markdown** — CommonMark plus GFM's tables, strikethrough and
task lists — by `src/popover/Markdown.tsx`.

## Only the answer

- **The question card stays plain.** It is the user's own words, or the Selection they grabbed, and
  neither was written to be formatted: a Selection out of a terminal is full of characters Markdown
  would take for syntax, and the card's job is to show what was sent.
- **The reasoning stays plain.** It is a model talking to itself in a clamped, scrolling aside; it is
  rarely structured, and it is not the thing being read.
- **Copy still copies the source.** `turn.answer` is the string the endpoint sent, and that is what
  goes to the clipboard — the place it is pasted next is as likely to be a Markdown editor or another
  prompt as a rich one, and source survives both. What the *eye* needed was the rendering; what the
  clipboard needs is the text.

## What stays true from the plain-text version

Three properties the `<p>` had for free, each now held on purpose:

- **Nothing can inject into the WebView.** No `rehype-raw`, no `dangerouslySetInnerHTML`. HTML in an
  answer is drawn as the text it is — `<script>` is eight characters on screen. A page that could run
  script here would inherit the IPC surface, so this is the one property that is not a matter of
  taste.
- **A single newline is still a line break.** CommonMark folds one into a space, which is right for
  prose wrapped at eighty columns and wrong here: half of what comes back is not prose. A translation
  keeps the lines of what it translated; an address, a verse and a list of names all arrive one per
  line. `remark-breaks` keeps them that way, so an answer with no Markdown in it looks exactly as it
  did.
- **Nothing is fetched.** An image is drawn as a link to itself. A remote `<img>` is a request the
  user did not make, to a URL the model wrote — and the model was handed a Selection that may have
  been written to steer it, which makes `![](https://…/?q=<what was selected>)` an exfiltration
  channel that fires on render. The CSP already refuses it (`img-src 'self' data:`); the component
  decides what is shown instead of a broken tile.

## Links leave the window, through Rust

A link in an answer is clickable, and the webview never navigates: it is the app, and a page loaded
into it would be talking to `invoke`. The click goes to `open_link`, which hands the URL to the
system browser.

That is the first command whose URL comes *from the webview* — `open_key_page` deliberately takes a
provider id and reads the URL from the row, because "open whatever is asked for" was more surface
than that gap was worth. Here there is no row to read it from: the URL is the model's text. So the
command is as narrow as a link can be, and narrow on the Rust side, which is the side that did not
render the untrusted text:

- `http` and `https` only. `file:`, `ms-settings:`, `javascript:` and every other handler the shell
  would launch are refused.
- No whitespace and no control characters, which is what an argument smuggled past a shell looks
  like and what a real link never contains.

The window applies the same test before drawing the affordance, so a link that would be refused is
plain text rather than a control that does nothing. The link's `title` is its destination: the words
are the model's, and where they lead should be readable before the click.

## A code block has a Copy of its own

A fenced block is the one part of an answer pasted somewhere else verbatim, and selecting it by hand
out of a box that scrolls sideways is the slowest gesture in the window. Each block carries the
language it claimed and a Copy that reports the same three outcomes, for the same 1600ms, as the
turn's own. The block scrolls rather than wraps, because wrapped code changes what a line is.

## Rejected

- **A per-Action `format = "plain"`.** The case for it is an Action whose output is legitimately full
  of Markdown syntax that must be shown raw. It would be a fourth thing an Action file may say, on the
  strength of a case nobody has reported; ADR-0019 and ADR-0020 spent two decisions narrowing that
  file. Copy already returns the source, which is what such an Action's output is for.
- **Syntax highlighting.** A grammar per language is the largest dependency this window would carry,
  for colour on a surface whose palette is deliberately monochrome (`globals.css`). The language
  label and a monospace box say "this is code"; which token is a keyword is the editor's job, one
  paste away.
- **Math.** KaTeX is a second renderer and a font set, for a notation most Actions never emit. An
  unrendered `$x^2$` is still legible.
- **Parsing only the settled answer, streaming as plain text.** It would reflow the whole turn at the
  moment the stream ends — the answer jumps just as the reader reaches the end of it. Parsing every
  delta is cheap at this length, and an unclosed fence mid-stream is drawn as the code block it is
  about to be.
- **Rendering HTML "safely" through a sanitiser.** A sanitiser is a list, and the property above is
  that there is no list to be wrong about.

## What this touches

ADR-0004 is untouched: nothing here is stored. ADR-0007 is why the copied/failed flag on a code
block is local state with a timer rather than something `exchange.load` clears — a block from the
last trigger is unmounted with its turn. ADR-0002's "a user-requested clipboard write is not
restored" covers the new Copy exactly as it covers the old one. And the README's "Citing the pages a
search read" stays under *Not doing*: a link the model writes into its answer is now clickable, but
the sources an endpoint streams *beside* the answer are still not read.
