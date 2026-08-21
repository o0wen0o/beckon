// The language, the way `theme.ts` is the theme: Rust owns the setting
// (ADR-0003), this module only maps it onto what the window renders. Nothing is
// remembered locally, so the three surfaces cannot drift apart (ADR-0015).
//
// Unlike the theme, a language is not a class on the document — every string in
// the tree changes — so this is a `Notifier` like the other stores rather than a
// one-shot stamp: components read it through `useT()` and re-render when it
// moves.
//
// Two catalogs, typed against each other, and no key lookup by string: a
// `t("settings.nav.actions")` cannot be checked by the compiler, and a missing
// key would surface as an English fragment — or as the key itself — in front of
// a reader who cannot read either.
import * as React from "react";
import { getConfig, onConfigChanged } from "../ipc";
import { Notifier } from "../store";
import { useStore } from "../useStore";
import type { Language } from "../types";
import { EN, type Strings } from "./en";
import { ZH } from "./zh";

export type { Strings };

const CATALOGS: Record<Language, Strings> = { en: EN, zh: ZH };

class I18nStore extends Notifier {
  language: Language = "en";
  strings: Strings = EN;

  apply(language: Language) {
    // `config-changed` fires for every setting; re-rendering every window
    // because the temperature moved is exactly what this guard is for.
    if (language === this.language) return;
    this.language = language;
    this.strings = CATALOGS[language] ?? EN;
    this.notify();
  }
}

/** One per window, like every other store: the window is created once (ADR-0007). */
export const i18n = new I18nStore();

/** The catalog to render from. Subscribes the component to language changes. */
export function useT(): Strings {
  return useStore(i18n).strings;
}

/**
 * Read the stored language, apply it, and re-apply on every `config-changed`.
 *
 * Awaited before a surface mounts, for the reason the theme is: a window that
 * paints English and then swaps to Chinese has told the user their setting did
 * not take. The subscription is never disposed — the windows live as long as the
 * process does (ADR-0007).
 */
export async function startLanguage(): Promise<void> {
  let language: Language = "en";
  try {
    language = (await getConfig()).language;
  } catch (error) {
    // Losing the language must not stop a surface from mounting.
    console.warn("could not read the language; falling back to English", error);
  }
  i18n.apply(language);
  void onConfigChanged((config) => i18n.apply(config.language));
}

/**
 * A sentence with something rendered inside it — a bold Action name, a `code`
 * span holding a filename.
 *
 * The whole sentence stays one entry in the catalog, with `{name}`-shaped slots
 * in it, because word order is not shared between the two languages: split into
 * "before" and "after" fragments, a translator can only move the slot by making
 * one fragment a lie. Unfilled slots are left as they are rather than blanked,
 * so a typo shows as `{nmae}` instead of as a hole.
 */
export function fill(
  template: string,
  slots: Record<string, React.ReactNode>,
): React.ReactNode[] {
  return template.split(/(\{\w+\})/g).map((piece, at) => {
    const slot = /^\{(\w+)\}$/.exec(piece);
    const filled = slot ? slots[slot[1]] : undefined;
    return <React.Fragment key={at}>{filled ?? piece}</React.Fragment>;
  });
}
