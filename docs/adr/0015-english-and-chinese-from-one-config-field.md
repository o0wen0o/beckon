---
status: accepted
---

# English and Chinese, chosen in Settings, from one config field

Beckon speaks two languages. `language` in `config.toml` is `"en"` (the default) or `"zh"`, it is
picked in Settings beside the theme, and it changes every surface at once — the Launcher, the
Popover, Settings, and the tray menu.

The words live in two catalogs, one per language: `src/lib/i18n/` for everything a webview renders,
`src-tauri/src/i18n.rs` for the handful of sentences Rust writes.

## Why a setting and not the OS locale

The theme has a `system` arm; the language does not, and the two are not the same question. An OS
appearance is a preference its owner set, about their screen. An OS locale is a guess about a
*reader* — a machine set to `zh-CN` because that is what shipped in the shop, a work laptop imaged
in English, a shared machine. Getting the theme wrong changes the palette until the user says
otherwise. Getting the language wrong replaces every word in the product, including the words that
would explain how to change it back.

So the default is English, the same way the theme defaults to light on a dark machine (README): a
setting is applied when it has been *chosen*.

## Why two catalogs and not one

The split is not English against Chinese — each catalog holds both — it is *who renders the
sentence*.

Almost everything is rendered by a webview, and it belongs in `src/lib/i18n/`. What cannot be:

- **The tray menu and its balloon.** Not a window. Nothing re-renders it, which is why
  `tray::retranslate` exists and `reload::reload_config` calls it.
- **Diagnostics that are derived state.** "This hotkey is already claimed by X" is decided in
  `hotkey::apply` and `Registry::load`, at the moment the conflict is detected, and travels to the
  UI as a string. Rust could hand the frontend a code plus arguments instead, and then the same
  sentence would exist as a Rust enum, a TypeScript union and a catalog entry. It is one sentence.
- **The model catalog's descriptions.** `models::CATALOG` is one row per model, read by both the
  request layer and the dropdown; a translation keyed off a model id in the frontend would put half
  of a row in each half of the app.

What is deliberately *not* translated on either side: `LlmError`'s `Display`, `toml`'s parse errors,
and the OS's own messages. Each is a cause quoted verbatim from something that does not speak
Chinese. The frontend names the *kind* in the reader's language — that is what `failure` in the
catalogs is for — and the quoted detail follows it as evidence.

Your Actions are not translated either, in either direction. They are your words, in the file you
own (ADR-0003), and Beckon has no business rewriting them. The one thing Beckon does write is the
`New Action` a fresh file starts as — English on purpose, because the file *name* is derived from
it and `slug("新建")` is `action`.

## Typed catalogs, not keys

`Strings` is `typeof EN`, and `ZH` is declared as a `Strings`. A key added to the English catalog is
a compile error in the Chinese one until it is translated; a key renamed in one is an error in the
other.

The alternative — `t("settings.nav.actions")` over a flat map, the shape every i18n library takes —
buys nothing here and costs the compiler. There is no runtime bundle loading, there are two
languages, and a missing key in that world surfaces as an English fragment or as the key itself, in
front of the one reader who cannot read either.

Sentences with something rendered inside them stay one entry, with `{name}` slots filled by `fill`.
Split into "before" and "after" fragments around a `<strong>`, a translator can only move the name
by making one fragment a lie — and Chinese does not put it where English does.

## What this changed

- **`platform.ts` no longer holds prose.** `CREDENTIAL_STORE`, `TRAY`, `AUTOSTART_LABEL`,
  `SYSTEM_APPEARANCE`, `MODIFIER_ADVICE` and `EMPTY_GRAB_CAUSE` are now `words` in each catalog,
  branching on the `IS_MAC` that module still exports. A constant can hold one dimension, and these
  have two: platform *and* language. What stays is the mechanism — `IS_MAC`, `COMMAND_MODIFIER` (a
  token that gets parsed, not a word), `hasCommandModifier`, `formatAccelerator`.
- **`sourceLabel` is a lookup, not a title-case.** "Selection" and "选中内容" are not the same string
  with a different first letter.
- **`describeFailure`, `unknownModelHint` and `thinkingWarning` take the catalog** rather than
  reaching for a module singleton. They stay pure functions, which is why they are outside
  components in the first place.
- **`--font-sans` and `--font-display` name the CJK faces.** Left to the engine's last resort, a
  Chinese run inside a Segoe UI line comes back a bitmap-era face a size heavier than the Latin
  around it. Latin still comes first in both stacks, so an English build renders exactly as before.
- **A language change re-derives the Action diagnostics.** They were phrased in the previous
  language and they are derived state, so `reload_config` re-runs `reload_actions` when the language
  moves — the same argument that has hotkeys re-registered rather than patched.

## What this costs

- **Every new string is two strings.** There is no fallback arm: an untranslated key does not
  compile. That is the point, and it is a tax on every UI change from here.
- **Two catalogs can agree in shape and disagree in meaning.** The compiler checks the keys, not the
  translation. `every_sentence_is_translated` in `i18n.rs` catches the Rust half of that — a copied
  English arm — and nothing catches the TypeScript half but reading it.
- **The language sits under Appearance**, which now covers rather more than a palette. A section of
  its own for one control is worse, and both settings are "how the product presents itself".
- **Chinese runs longer than English in some places and shorter in most.** The Launcher's four fixed
  columns (`ActionCells`) were measured against English; they hold, but they are one more thing a
  third language would have to be checked against.
