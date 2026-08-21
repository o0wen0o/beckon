// How an Input Source is drawn, in one place. The Launcher's list and the
// Actions list in Settings are meant to read as the same list — same icon, same
// label, same column — and two copies of this map is exactly how they stop.
import { SparklesIcon, TextCursorInputIcon, TextSelectIcon } from "lucide-react";
import type { InputSource } from "./types";

export const SOURCE_ICON = {
  selection: TextSelectIcon,
  prompt: TextCursorInputIcon,
  auto: SparklesIcon,
};

/** Title case for display; the value itself stays the CONTEXT.md term. */
export function sourceLabel(source: InputSource) {
  return source.charAt(0).toUpperCase() + source.slice(1);
}
