// An answer, rendered as Markdown.
//
// Only the answer: the question card is the user's own words and the reasoning
// is a model talking to itself, and neither is written to be formatted. An
// answer is — every model Beckon can reach emits Markdown unprompted, and as
// plain text a fenced block is three backticks and a table is a row of pipes.
//
// Three things stay true from when this was a `<p>` with preserved whitespace:
//
// - **Nothing can inject into the WebView.** There is no `rehype-raw` and no
//   `dangerouslySetInnerHTML`: HTML in an answer is drawn as the text it is.
// - **A single newline is still a line break** (`remark-breaks`). Half of what
//   comes back here is not prose — a translation keeps the lines of what it
//   translated — and CommonMark's "one newline is a space" would run an address
//   or a verse into one paragraph.
// - **Nothing is fetched.** An image is drawn as a link to itself: a remote
//   `<img>` is a request the user did not make, to a URL the model wrote, and
//   the model may have been steered by the Selection it was handed. The CSP
//   refuses it anyway (`img-src 'self' data:`); this is what is shown instead
//   of a broken tile.
//
// Every element is mapped to classes here because components carry no CSS of
// their own (`globals.css`), and the map is module-level so `React.memo` below
// has stable props: a settled turn is not re-parsed while a later one streams.
import * as React from "react";
import { CheckIcon, CopyIcon, TriangleAlertIcon } from "lucide-react";
import ReactMarkdown, { type Components, type ExtraProps } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import { useT } from "@/lib/i18n";
import { copyToClipboard, openLink } from "@/lib/ipc";
import { cn } from "@/lib/utils";
// How long a code block's Copy says what happened is the turn's own number:
// the two buttons are one behaviour at two sizes.
import { REPORT_FOR } from "./exchange";

/** The tree `react-markdown` hands a component beside its props. Read off the
 *  library's own type rather than imported from `hast`, which is a dependency
 *  of a dependency. */
type Element = NonNullable<ExtraProps["node"]>;
type Child = Element["children"][number];

/** The answer is the only prose in the product, so it gets its own leading.
 *  `max-w-full min-w-0` is for the blocks that scroll sideways: the column
 *  this sits in shrinks to fit, and without a ceiling a wide code block would
 *  widen the turn instead of scrolling inside it. */
const BLOCKS = "flex max-w-full min-w-0 flex-col gap-2 leading-relaxed wrap-break-word";

/**
 * The breathing bar that says "output is arriving", on the end of whatever
 * block is last. A pseudo-element rather than a node, because the last block is
 * the parser's — a paragraph now, a list item a moment later — and there is no
 * stable place in the tree to put a `<span>`.
 *
 * Not a blinking caret: a blink says "type here".
 */
const ARRIVING = cn(
  // Two places it can land, one variant: the last block, or — when that block
  // is a list — the list's last item, because a bar after the whole `<ul>` sits
  // on a line of its own under the bullets and reads as an empty item.
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:bg-foreground",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:ml-0.5",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:inline-block",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:h-4",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:w-1.5",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:animate-pulse",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:rounded-xs",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:align-text-bottom",
  "[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:content-['']",
  "motion-reduce:[&>:last-child:not(ul,ol),&>:is(ul,ol):last-child>li:last-child]:after:animate-none",
);

const PLUGINS = [remarkGfm, remarkBreaks];

/** Whether a link is one `open_link` will take. Rust decides (it is the side
 *  that did not render the model's text); this only stops the window drawing
 *  an affordance that would be refused. */
const isWebUrl = (url: unknown): url is string =>
  typeof url === "string" && /^https?:\/\//i.test(url);

function textOf(node: Element | Child): string {
  if (node.type === "text") return node.value;
  return "children" in node ? node.children.map(textOf).join("") : "";
}

/**
 * A link out. The webview never navigates — it is the app, and a page loaded
 * into it would inherit the IPC surface — so the click is handed to Rust, which
 * opens the system browser. `title` is the destination: the words are the
 * model's, and where they lead should be readable before the click.
 */
function Link({ href, children }: { href: unknown; children?: React.ReactNode }) {
  if (!isWebUrl(href)) return <>{children}</>;
  const open = (event: React.MouseEvent) => {
    event.preventDefault();
    // A refusal is not reported: the Popover hosts no Toaster, and the only
    // URLs that reach here already passed the same test Rust applies.
    void openLink(href).catch(() => undefined);
  };
  return (
    <a
      href={href}
      title={href}
      onClick={open}
      // A middle click is a navigation too, and not one `onClick` sees.
      onAuxClick={open}
      className="decoration-border-strong hover:decoration-foreground underline underline-offset-4"
    >
      {children}
    </a>
  );
}

/**
 * A fenced block: the code, what language it said it was, and a Copy of its
 * own — the one thing in an answer that is pasted somewhere else verbatim, and
 * selecting it by hand out of a scrolling box is the slowest gesture here.
 *
 * Built from the tree rather than from `children`, so the inline `code` below
 * never has to ask whether it is inside a `pre`.
 */
function CodeBlock({ node }: ExtraProps) {
  const t = useT();
  const [report, setReport] = React.useState<"none" | "copied" | "failed">("none");
  const clear = React.useRef<number | undefined>(undefined);
  React.useEffect(() => () => window.clearTimeout(clear.current), []);

  const code = node?.children.find(
    (child): child is Element => child.type === "element" && child.tagName === "code",
  );
  // The parser ends every block with a newline that is not part of the code.
  const text = (code ? textOf(code) : "").replace(/\n$/, "");
  const classes = code?.properties.className;
  const language = (Array.isArray(classes) ? classes : [])
    .map(String)
    .find((name) => name.startsWith("language-"))
    ?.slice("language-".length);

  const copy = async () => {
    let outcome: "copied" | "failed" = "copied";
    try {
      // A user-requested clipboard write: not restored.
      await copyToClipboard(text);
    } catch {
      outcome = "failed";
    }
    setReport(outcome);
    window.clearTimeout(clear.current);
    clear.current = window.setTimeout(() => setReport("none"), REPORT_FOR);
  };

  return (
    <div className="overflow-hidden rounded-md border">
      {/* On the paper, not on the block's own grey: `ghost` hovers to
          `--accent`, which is `--muted` to the pixel, and a Copy with no hover
          reads as a label. */}
      <div className="flex h-7 items-center justify-between gap-2 border-b pr-1 pl-3">
        <span className="text-muted-quiet truncate font-mono text-meta">{language}</span>
        <Button
          variant="ghost"
          size="xs"
          className={report === "failed" ? "text-destructive" : "text-muted-quiet"}
          onClick={() => void copy()}
        >
          {report === "failed" ? (
            <TriangleAlertIcon />
          ) : report === "copied" ? (
            <CheckIcon />
          ) : (
            <CopyIcon />
          )}
          {report === "failed"
            ? t.popover.copyFailed
            : report === "copied"
              ? t.popover.copied
              : t.popover.copy}
        </Button>
      </div>
      {/* Scrolls rather than wraps: wrapped code changes what a line is.
          `scheme-dark` is for the bar that scrolling draws: the theme is a
          class, not a `color-scheme`, so a native scrollbar stays the light
          one unless the box it belongs to says otherwise. */}
      <pre className="bg-muted overflow-x-auto px-3 py-2 font-mono leading-normal text-quiet dark:scheme-dark">
        <code>{text}</code>
      </pre>
    </div>
  );
}

/** Headings do not grow. The window is 620px and the scale has no step between
 *  body and the pane's display title; weight is what says "heading" here. */
const HEADING = "mt-1 font-semibold first:mt-0";
const SUBHEADING = "mt-1 font-medium first:mt-0";

const COMPONENTS: Components = {
  a: ({ href, children }) => <Link href={href}>{children}</Link>,
  img: ({ src, alt }) => <Link href={src}>{alt || (typeof src === "string" ? src : "")}</Link>,
  pre: CodeBlock,
  code: ({ children }) => (
    <code className="bg-muted rounded-sm px-1 py-0.5 font-mono">{children}</code>
  ),
  h1: ({ children }) => <h1 className={HEADING}>{children}</h1>,
  h2: ({ children }) => <h2 className={HEADING}>{children}</h2>,
  h3: ({ children }) => <h3 className={SUBHEADING}>{children}</h3>,
  h4: ({ children }) => <h4 className={SUBHEADING}>{children}</h4>,
  h5: ({ children }) => <h5 className={SUBHEADING}>{children}</h5>,
  h6: ({ children }) => <h6 className={SUBHEADING}>{children}</h6>,
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  // A task list is GFM's, and its items carry their own checkbox — a bullet
  // beside a box is two markers for one line.
  ul: ({ className, children }) => (
    <ul
      className={cn(
        "space-y-1",
        className?.includes("contains-task-list") ? "list-none" : "list-disc pl-5",
      )}
    >
      {children}
    </ul>
  ),
  ol: ({ start, children }) => (
    <ol start={start} className="list-decimal space-y-1 pl-5">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="[&>ol]:mt-1 [&>p+p]:mt-1 [&>ul]:mt-1">{children}</li>,
  input: ({ node: _node, ...props }) => <input {...props} className="mr-1.5 align-middle" />,
  blockquote: ({ children }) => (
    <blockquote className="border-border-strong text-muted-foreground flex flex-col gap-2 border-l-2 pl-3">
      {children}
    </blockquote>
  ),
  hr: () => <hr />,
  // Its own scroller, for the reason a code block has one: a table wider than
  // the window should move inside the turn, not widen it.
  table: ({ children }) => (
    <div className="max-w-full overflow-x-auto dark:scheme-dark">
      <table className="border-collapse text-quiet">{children}</table>
    </div>
  ),
  th: ({ style, children }) => (
    <th style={style} className="border-border-strong border-b px-2 py-1 text-left font-medium">
      {children}
    </th>
  ),
  td: ({ style, children }) => (
    <td style={style} className="border-b px-2 py-1 align-top">
      {children}
    </td>
  ),
};

interface MarkdownProps {
  text: string;
  /** Whether more is still arriving, which is what the bar on the end says. */
  streaming: boolean;
}

/**
 * Re-parsed whole on every delta, which is the simple thing — and deferred, so
 * that it stays affordable when the answer is not a screen or two. Rust sends
 * a delta per frame, and each one is a full parse of everything so far: on a
 * long, code-heavy answer that is quadratic work at sixty parses a second, and
 * what it starved was the window's own keys — Esc to stop, the composer.
 * `useDeferredValue` lets React skip the parses it cannot keep up with and
 * draw the latest text when it can; the last delta is never skipped, because a
 * deferred value always catches up once nothing newer is arriving.
 *
 * What `memo` buys is the turns *above* the live one — their text is the same
 * string, so a follow-up streaming under them costs them nothing.
 */
export const Markdown = React.memo(function Markdown({ text, streaming }: MarkdownProps) {
  const shown = React.useDeferredValue(text);
  return (
    <div className={cn(BLOCKS, streaming && ARRIVING)}>
      <ReactMarkdown remarkPlugins={PLUGINS} components={COMPONENTS}>
        {shown}
      </ReactMarkdown>
    </div>
  );
});
