// How an Input Source is drawn, in one place: the Launcher's list and the
// Actions list in Settings must read as the same list.
import { SparklesIcon, TextCursorInputIcon, type LucideIcon } from "lucide-react";
import type { Strings } from "./i18n";
import type { InputSource } from "./types";

/** Every Input Source, in the order they are offered. The editor's segmented
 *  control reads this rather than restating the values — two of them since
 *  ADR-0020 retired `selection`. */
export const SOURCES: InputSource[] = ["prompt", "auto"];

export const SOURCE_ICON: Record<InputSource, LucideIcon> = {
  prompt: TextCursorInputIcon,
  auto: SparklesIcon,
};

/** How the source reads to a person; the value itself stays the CONTEXT.md
 *  term. Looked up rather than title-cased: the two words are not the same word
 *  in every language (ADR-0015). */
export function sourceLabel(source: InputSource, t: Strings) {
  return t.inputSource[source];
}
