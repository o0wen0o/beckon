// How an Input Source is drawn, in one place. The Launcher's list and the
// Actions list in Settings are meant to read as the same list — same icon, same
// label, same column — and two copies of this map is exactly how they stop.
import { SparklesIcon, TextCursorInputIcon, TextSelectIcon, type LucideIcon } from "lucide-react";
import type { InputSource } from "./types";

/** Every Input Source, in the order they are offered. The editor's segmented
 *  control reads this rather than restating the three values with the labels
 *  written out beside them — that copy is the one most likely to disagree,
 *  because it is the one place the value is chosen. */
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
