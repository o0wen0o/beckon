// The follow-up box. It owns its own text and nothing else — the height is
// `field-sizing-content` with a floor and a ceiling, which is the browser doing
// what a resize handler used to. It is mounted only when there is something to
// type into, so a fresh mount is also the reset: the window is reused
// (ADR-0007) and a draft must not survive into the next trigger.
//
// No label. The turns above it are sided rather than labelled, so a label
// column here would be the only one in the window.
import * as React from "react";
import { SendIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

interface ComposerProps {
  placeholder: string;
  onSend: (text: string) => void;
}

export function Composer({ placeholder, onSend }: ComposerProps) {
  const [draft, setDraft] = React.useState("");
  const box = React.useRef<HTMLTextAreaElement | null>(null);

  // The Popover is summoned by a hotkey; reaching for the mouse to click into
  // the one box on screen is the thing that would make it not worth summoning.
  React.useEffect(() => {
    box.current?.focus();
  }, []);

  const send = () => {
    const text = draft.trim();
    if (text === "") return;
    setDraft("");
    onSend(text);
  };

  return (
    <div className="flex flex-none items-end gap-2 border-t px-3.5 py-2.5">
      <Textarea
        ref={box}
        rows={1}
        value={draft}
        placeholder={placeholder}
        aria-label={placeholder}
        onChange={(event) => setDraft(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            send();
          }
        }}
        className="max-h-30 min-h-9 resize-none py-1.5"
      />
      <Button className="flex-none" disabled={draft.trim() === ""} onClick={send}>
        <SendIcon /> Send
      </Button>
    </div>
  );
}
