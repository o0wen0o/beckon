// How an Input Source is drawn, in one place: the Launcher's list and the
// Actions list in Settings must read as the same list.
import { SparklesIcon, TextCursorInputIcon, TextSelectIcon, type LucideIcon } from "lucide-react";
import type { InputSource } from "./types";

/** Every Input Source, in the order they are offered. The editor's segmented
 *  control reads this rather than restating the three values. */
export const SOURCES: InputSource[] = ["selection", "prompt", "auto"];

export const SOURCE_ICON: Record<InputSource, LucideIcon> = {
  selection: TextSelectIcon,
  prompt: TextCursorInputIcon,
  auto: SparklesIcon,
};

/** Title case for display; the value itself stays the CONTEXT.md term. */
export function sourceLabel(source: InputSource) {
  return source.charAt(0).toUpperCase() + source.slice(1);
}
