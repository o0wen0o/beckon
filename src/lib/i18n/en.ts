// The English catalog — and, because [`Strings`] is derived from it, the shape
// every other one has to match key for key. A key added here is a compile error
// in `zh.ts` until it is translated, which is the only mechanism keeping the two
// from drifting (`index.tsx` explains why the catalogs are typed rather than
// looked up by string).
//
// Nested by surface, so a sentence is found where it is rendered. Entries are
// plain strings; a sentence with a value in it is a function, and one with an
// element in it carries `{name}`-style slots for `fill` — Chinese does not put
// an interpolated name where English does, so the *whole* sentence has to be one
// translatable unit rather than two fragments glued around a component.
//
// House style, and the reason most sentences here are one clause: say what the
// reader can act on and stop. A wire field name, an HTTP header, a path inside
// the request — those are facts about Beckon's implementation, and a reader who
// needs them has the ADRs. What stays long is what a reader would be worse off
// not knowing: the privacy disclosure on a relaying row, and the two "this
// cannot be undone" sentences.
//
// The words that differ per platform live here too (ADR-0015). `platform.ts`
// keeps `IS_MAC` and the accelerator logic; what it no longer keeps is prose,
// because prose has two dimensions now — platform *and* language — and a
// constant can only hold one of them.
import { IS_MAC } from "../platform";
import { kb } from "./units";

export const EN = {
  /** Words the product needs in more than one surface. */
  words: {
    /** What the OS calls the place the API key is kept (ADR-0005). */
    credentialStore: IS_MAC ? "Keychain" : "Windows Credential Manager",
    /** Where Beckon sits when no window is open. */
    tray: IS_MAC ? "menu bar" : "tray",
    /** The autostart switch's name, which is the platform's own phrase for it. */
    autostart: IS_MAC ? "Start at login" : "Start with Windows",
    /** What `theme = "system"` follows. */
    systemAppearance: IS_MAC ? "macOS appearance" : "Windows preference",
    /** Which modifiers `hotkey::parse` accepts, in the platform's own names. */
    modifierAdvice: IS_MAC ? "Cmd, Control, Option or Shift" : "Ctrl, Alt or Shift",
    settings: "Settings",
    cancel: "Cancel",
    /** The one control a toast carries (src/components/Toaster.tsx). */
    dismiss: "Dismiss",
  },

  /** How an Input Source reads. The value itself stays the CONTEXT.md term. */
  inputSource: {
    prompt: "Prompt",
    auto: "Auto",
    /** The Launcher's and the Actions list's shared cell title. */
    cell: (label: string) => `Input Source: ${label}`,
    /** What a file that will not parse shows where its Input Source would be. */
    repair: "Repair",
  },

  /** One prefix per `Failure.kind`, so a new kind cannot reach one consumer and
   *  miss the other. `fallback` is the unknown-kind case. A prefix names the
   *  kind in four or five words; the cause quoted after it carries the detail. */
  failure: {
    auth: "The key was rejected",
    network: "Could not reach the API",
    http: "The API refused the request",
    "no-credential": "No API key saved",
    "read-error": `Could not read the ${IS_MAC ? "Keychain" : "Credential Manager"}`,
    interrupted: "The answer stopped early",
    empty: "No models found",
    config: "Not set up for this",
    "no-model": "No model chosen",
    "capture-too-large": "Screenshot too big to send",
    "capture-too-many": "Too many screenshots",
    "capture-unreadable": "Could not read the screenshot",
    fallback: "Failed",
  } as Record<string, string>,

  launcher: {
    searchPlaceholder: "Search Actions…",
    searchLabel: "Search Actions",
    listLabel: "Actions",
    escape: "Esc",
    noActions: "No Actions yet.",
    addInSettings: "Add one in Settings",
    /** `{query}` is the query, set in mono by the caller. */
    nothingMatches: "Nothing matches {query}.",
    widen: "Delete a letter to see more.",
    selected: (characters: number) => `${characters} characters selected`,
    noSelection: "No selection",
    move: "move",
    run: "run",
    settingsTitle: (accelerator: string) => `Settings (${accelerator})`,
  },

  popover: {
    close: "Close",
    thinking: "thinking",
    /** Beside it, and the same kind of thing: a capability this turn is using,
     *  and the one that is billed per request (ADR-0026). */
    webSearch: "web search",
    nothingToShow: "Nothing to show.",
    /** `{name}` is the Action, set bold by the caller. */
    typeYourInput: "Type what you want to send to {name}.",
    waiting: "Waiting for a reply",
    runningWaiting: "Waiting",
    runningStreaming: "Answering",
    stop: "Stop",
    firstInput: "Your input…",
    followUp: "Ask a follow-up…",
    send: "Send",
    showAll: "Show all",
    showLess: "Show less",
    showThinking: "Show thinking",
    hideThinking: "Hide",
    failed: "Failed",
    interrupted: "Interrupted",
    interruptedEmpty: "Stopped before any answer",
    cancelled: "Cancelled.",
    retry: "Retry",
    openSettings: "Open Settings",
    copy: "Copy",
    copied: "Copied",
    /** Copy is the only way a result leaves Beckon, so a clipboard write that
     *  failed cannot be reported by the checkmark simply not appearing. On the
     *  button rather than as a toast: the Popover hosts no Toaster, and the
     *  button is where the gesture was made. */
    copyFailed: "Could not copy",

    /** The Capture (CONTEXT.md) is a screenshot in prose — nobody calls it a
     *  Capture out loud, and the term is for code and comments. */
    captureTooltip: (accelerator: string) => `Take a screenshot (${accelerator})`,
    removeCapture: "Remove screenshot",
    captureCancelled: "Nothing was captured.",
    captureRetry: (accelerator: string) => `Press ${accelerator} to try again.`,
    captureNote: "Add a note…",
    /** Beside the thumbnail: what was captured, and how big it is. Takes the
     *  raw byte count, so the two thumbnails cannot round it differently. */
    captureMeta: (width: number, height: number, bytes: number) =>
      `${width}×${height} · ${kb(bytes)} KB`,
    /** Under the rail: what is attached, as a set. The individual sizes are one
     *  tap away in the preview, and four of them side by side read as noise. */
    captureSet: (count: number, bytes: number) =>
      `${count} screenshot${count === 1 ? "" : "s"} · ${kb(bytes)} KB total`,
    viewCapture: "View screenshot",
    /** Said in words for a screen reader; the preview draws it as `2 / 3`. */
    capturePosition: (index: number, total: number) => `Screenshot ${index} of ${total}`,
    previousCapture: "Previous screenshot",
    nextCapture: "Next screenshot",
    closeCapture: "Close screenshot",
    /** On the image itself, because zoom has no visible control of its own —
     *  the wheel and a click are the whole interface (ADR-0017). */
    zoomHint: "Click to zoom, scroll to resize",
    zoomOutHint: "Click to fit, drag to move",
  },

  settings: {
    nav: {
      label: "Settings",
      connection: "Connection",
      actions: "Actions",
      triggering: "Triggering",
      appearance: "Appearance",
      attention: "Something here needs attention",
      openFolder: "Open folder",
      /** A lead for `describeFailure`: `reveal_config_dir` can fail to create
       *  the directory or to hand it to the OS, and a button that opens nothing
       *  and says nothing reads as a dead button. */
      openFolderFailed: "Could not open the folder",
    },

    status: {
      notSaved: (message: string) => `Not saved — ${message}`,
      saving: "Saving…",
      standing: "Changes save automatically.",
      rawFile: "This file has an error. Use Save file above.",
    },

    connection: {
      title: "Connection",
      /** `fallback` is the default row's label, `store` what the OS calls its
       *  credential store. Both are in the sentence because both are the answer
       *  to "where does my text go and what does it carry" (ADR-0021). */
      lede: (fallback: string, store: string) =>
        `Where your text is sent. Actions use ${fallback} unless they say otherwise. Keys are kept in the ${store}, never in a file.`,
      welcomeLead: "Welcome.",
      welcomeBody:
        " DeepSeek is ready — add a key to start. Any OpenAI-compatible endpoint works instead, and a local one needs no key.",
      setUp: (label: string) => `Set up ${label}`,
      getKeyFrom: (host: string) => `Get a key from ${host}`,
      /** A lead for `describeFailure`. The row may carry no `key_page`, the URL
       *  may not be `https`, or the OS may refuse it — and this button is most
       *  often pressed on a first run, where a dead link is the worst moment for
       *  one. */
      keyPageFailed: "Could not open the key page",
      addPreset: "Add from preset…",
      addBlank: "Blank",
      /** The badge on the default row. Sentence case, not an uppercase eyebrow:
       *  it is a word about the row it sits on, and it says something quiet —
       *  which row an Action inherits, not a state the row is in. */
      defaultTag: "Default",
      defaultForNew: "used by Actions that do not pick one",
      makeDefault: "Make default",
      usedByNone: "no Actions",
      usedByOne: (name: string) => `1 Action — ${name}`,
      usedByMany: (count: number) => `${count} Actions`,
      staysLocal: "stays on this machine",
      missingKey: "no key",
      edit: (label: string) => `Edit ${label}`,
      removeLabel: (label: string) => `Remove ${label}`,
      removeHint: "Removes this endpoint and its saved key.",
      /** Refused rather than cascaded: which endpoint those Actions should use
       *  instead is not a decision this button gets to make. */
      removeBlocked: (names: string) => `${names} use this endpoint. Point them elsewhere first.`,
      removeLast: "The last endpoint cannot be removed.",
      back: "Connection",
      unnamed: "(unnamed)",

      endpoint: "Endpoint",
      name: "Name",
      nameHint: "Display only — safe to change any time.",
      baseUrlHint: "Any OpenAI-compatible endpoint. A trailing /v1 is fine.",
      baseUrl: "Base URL",
      apiKey: "API key",
      apiKeyHint: (store: string) => `Kept in the ${store}. Local endpoints need no key.`,
      save: "Save",
      saved: "Saved.",
      remove: "Remove",
      removed: "Removed.",
      stored: "Saved — ends in",
      noKeyYet: "No key saved yet.",
      unauthenticated: "No key — requests are sent without one.",
      readError: (store: string, message: string) =>
        `Could not read the ${store}: ${message}. Save the key again.`,
      reachability: "Connection test",
      reachabilityHint: "Sends one small test request.",
      test: "Test",
      testing: "Testing…",
      testOk: "Connected.",
      /** The test also settles the dialect on a row no preset filled in, so it
       *  reports what it found rather than making the user check. */
      testOkDetected: (dialect: string) => `Connected — thinking control set to ${dialect}.`,
      /** A relaying row, said on the identity line under the URL it is about
       *  (ADR-0025). Two sentences, and the length is earned: this is the one
       *  place a reader learns a second company sees their text. */
      relaysLead: "Relays.",
      relaysBody: (broker: string) =>
        `Your key stays with ${broker}, but your text goes on to whoever runs the model below. Two companies see it, not one.`,

      rowDefaults: "Defaults for this endpoint",
      rowDefaultsNote: "Actions can override these",
      refresh: "Refresh models",
      loading: "Loading models…",
      live: "From this endpoint.",
      /** A list from disk rather than from the endpoint just now (ADR-0024).
       *  Distinct from `live` because `listNotice` may be showing beside it. */
      cached: "From an earlier check.",
      /** Named for the list, not for a catalog: a provider row carries no
       *  catalog, so the only list is the endpoint's own — whether it answered
       *  just now or last time (CONTEXT.md, one name per thing). */
      listNotice: (cause: string) => `${cause} — showing the earlier list.`,
      /** The initial state of every row a user adds: a row ships no model, so
       *  until the endpoint answers there is nothing to pick. Two of them,
       *  because a local endpoint wants no key and telling it to store one names
       *  the single thing ADR-0021 says is not a fault. */
      noModelsYet: "Save a key, then press Refresh models.",
      noModelsYetLocal: "Start this endpoint, then press Refresh models.",
      listUnavailable: "Could not load the model list",
      /** The other half of `listUnavailable`. A refresh that succeeds and
       *  returns the same list it already had changes nothing on screen, so the
       *  gesture goes unanswered exactly as the failed one used to — the count
       *  is what makes an unchanged list still an answer. */
      listRefreshed: (count: number) => `Found ${count} model${count === 1 ? "" : "s"}.`,
      /** `#adoptOnlyModel` writing the one model on offer onto the row. Said out
       *  loud because nothing asked for it: Beckon changed config.toml, and a
       *  write with no gesture behind it is the one that has to announce
       *  itself. */
      modelAdopted: (model: string) => `Only ${model} was on offer, so it is now selected.`,

      /** The wire dialect, on a row the user typed themselves — a preset carries
       *  the right value already, and showing it there invites breaking it. Not
       *  a control any more: Test connection asks the endpoint and writes the
       *  answer, so this row states what was found. The hint names the vendor it
       *  is for and nothing about the field itself: the reader picks by whose
       *  endpoint they typed in, and the JSON is in `llm/request.rs`. */
      reasoning: "Thinking control",
      reasoningName: {
        deepseek: "DeepSeek",
        qwen: "Qwen",
        openai: "OpenAI",
        minimax: "MiniMax",
        openrouter: "OpenRouter",
        none: "None",
      },
      reasoningHint: {
        deepseek: "For DeepSeek. Its models think unless turned off.",
        qwen: "For Qwen3, on vLLM, SGLang or DashScope.",
        openai: "For OpenAI's own API, GPT-5.6 and newer.",
        minimax: "For MiniMax's own API.",
        openrouter: "For OpenRouter.",
        none: "This endpoint has no thinking switch, which is true of most.",
      },
      thinkingHint: (label: string) => `Slower, but better at hard questions. ${label} supports it.`,
      thinkingHintNone: (label: string) => `${label} has no thinking switch, so this does nothing.`,

      /** The web-search wire (ADR-0026). Unlike `reasoning`, this one is *asked*
       *  rather than stated: nothing detects it, because a probe would run a
       *  real search and be billed for it. A preset already carries the answer,
       *  so only a hand-made row is asked. */
      search: "Web search support",
      searchName: {
        xai: "xAI",
        dashscope: "DashScope",
        openrouter: "OpenRouter",
        none: "None",
      },
      searchHint: {
        xai: "For xAI. It searches unless turned off.",
        dashscope: "For Alibaba DashScope, on Qwen Plus and Flash — not Max.",
        openrouter: "For OpenRouter, which runs the search and charges per request.",
        none: "This endpoint cannot be asked to search, which is true of most.",
      },
      webSearchHint: (label: string) => `Slower, and ${label} charges extra per answer.`,
      webSearchHintNone: (label: string) => `${label} cannot search, so this does nothing.`,
      thisEndpoint: "This endpoint",
    },

    triggering: {
      title: "Triggering",
      lede: "How you call Beckon up.",
      hotkeyDeadLead: "A hotkey is not working.",
      hotkeyDeadBody: "Record a different combination below.",
      permissionLead: "Beckon cannot read your selected text.",
      permissionBody: " macOS allows this only for apps you trust under Privacy & Security → Accessibility.",
      permissionStillWorks:
        "Hotkeys and typed input still work. Turn Beckon on in the list, then come back here.",
      openAccessibility: "Open Accessibility settings",
      /** A lead for `describeFailure`: there is no such pane off macOS, and the
       *  OS can refuse the URL. */
      openAccessibilityFailed: "Could not open that settings pane",
      summoning: "Summoning",
      launcherHotkey: "Launcher hotkey",
      launcherHotkeyHint: "A combination already in use turns red.",
      autostartHint: (tray: string) => `Beckon sits in the ${tray}, ready when you need it.`,
      updates: "Updates",
      updateCheck: "Check for updates automatically",
      /** Says what the switch does *not* cover: the tray item stays there
       *  either way, and would otherwise read as the switch being ignored
       *  (ADR-0022). */
      updateCheckHint: (tray: string) =>
        `Checks quietly at startup. You can also check any time from the ${tray}.`,
    },

    appearance: {
      title: "Appearance",
      lede: "Applies to every Beckon window at once.",
      theme: "Theme",
      light: "Light",
      dark: "Dark",
      system: "System",
      themeHint: (appearance: string) => `“System” follows your ${appearance}, live.`,
      language: "Language",
      /** Each language names itself, in itself: a reader who cannot read the
       *  current one still has to be able to find their own. */
      english: "English",
      chinese: "中文",
      languageHint: "Applies to every window and the tray. Your own Actions are never translated.",
    },


    actions: {
      title: "Actions",
      lede: "One Action is one saved prompt. Its file name is its identity.",
      create: "New Action",
      count: (count: number) => `${count} ${count === 1 ? "Action" : "Actions"}`,
      willNotParse: "Has an error",
      empty: "No Actions yet. One Action is one saved prompt.",
      gone: "That Action is gone.",
      rawUnder: "has an error — edit as text",
      back: "Back to Actions",
      backTo: (name: string) => `Back to ${name}`,
      deleteTitle: (name: string) => `Delete “${name}”?`,
      /** `{file}` is the filename, set in mono by the caller. The one sentence
       *  that keeps its warning: a delete cannot be walked back. */
      deleteBody: "{file} will be deleted. This cannot be undone.",
      deleteConfirm: "Delete file",
      /** The four outcomes the status bar used to carry. It says "Not saved",
       *  which is the wrong sentence about a read, a create and a delete — and
       *  in the delete's case a standing red line about a write that never
       *  happened. Each names the file, because the editor closing is the only
       *  other sign and it does not say which one went. */
      deleted: (file: string) => `${file} was deleted.`,
      deleteFailed: "Could not delete the file",
      createFailed: "Could not create the Action",
      rawOpenFailed: "Could not read the file",
      /** The raw editor swapping back to the form is the answer already, but
       *  only to someone who knows why — said out loud, it is the repair
       *  landing rather than the pane changing under them. */
      rawSaved: (file: string) => `${file} is fixed — back to the editor.`,

      hotkeyDeadLead: "This Action’s hotkey is not working.",
      hotkeyDeadBody: "Clear or change it to save this Action.",
      clearHotkey: "Clear the Direct Hotkey",

      definition: "Definition",
      definitionHint: "Its name and the prompts it sends.",
      trigger: "Trigger",
      inputSource: "Input Source",
      sourceHint: {
        prompt: "Always asks you to type.",
        auto: "Uses your selected text, or asks you to type.",
      },
      directHotkey: "Direct Hotkey",
      directHotkeyHint: "Optional. Without one, run it from the Launcher.",

      overrides: "Model overrides",
      overridesNote: (label: string) => `Unmarked rows follow ${label}`,
      /** The row ADR-0021 added, and the reason there is no global switch. */
      provider: "Endpoint",
      providerHint: "Where this Action sends. Unmarked follows the default.",
      providerLocal: (label: string) => `${label} — local`,
      /** A model pinned before the endpoint changed. Kept, never rewritten. */
      strandedModel: (model: string, label: string, fallback: string) =>
        `${label} does not offer ${model}. It is kept as you set it — revert to use ${fallback}.`,
      needsKey: (label: string) =>
        `No key saved for ${label}, so this Action will fail. Other Actions still work.`,
      sends: "What this Action sends",
      thinkingHint: "Slower, but better at hard questions.",
      webSearchHint: "Reads the web before answering. Slower, and costs extra.",

      thisFile: "This file",
      deleteLabel: "Delete this Action",
      /** Keeps its warning, for the same reason `deleteBody` does. */
      deleteHint: (file: string) => `Deletes ${file}. This cannot be undone.`,
      deleteButton: "Delete Action",

      name: "Name",
      nameWarning: "Without a name, the Launcher shows the file name.",
      description: "Description",
      descriptionHint: "Shown in the Launcher, and searched.",
      systemPrompt: "System prompt",
      systemPromptHint: "How the model should behave.",
      userTemplate: "User template",
      userTemplateHint: "{{input}} becomes your text. Leave empty to send just your text.",
      templateWarning: "This template never uses {{input}}.",
      templateWarningShort: "The user template never uses {{input}}.",

      saveFile: "Save file",
      reloadsWhenItParses: "Saves once the file is valid.",
    },
  },

  controls: {
    hotkey: {
      record: "Record…",
      change: "Change…",
      recording: "Press keys…",
      clear: "Clear the Direct Hotkey",
      // The button's visible word, separate from the sentence above it: the
      // label a screen reader reads has to name *which* hotkey, and the same
      // sentence on a 12px button would make Clear the widest thing in the row.
      clearShort: "Clear",
      needsModifier: (advice: string) => `Add ${advice} — one key alone would fire everywhere.`,
    },
    model: {
      /** The two control names, here rather than under a pane: both the endpoint
       *  screen and an Action's overrides draw the same row (ADR-0021). */
      label: "Model",
      thinking: "Think before answering",
      configuredGroup: "Added by you",
      noneChosen: "No model chosen",
      /** One sentence, not a two-part composition: there used to be a second
       *  reading — "not one of the models Beckon knows" — for a list that came
       *  from a documented catalog. A row carries no catalog, so every list is
       *  the endpoint's own and there is one thing left to say. */
      unknown: (model: string) => `${model} is not in this endpoint's list, but your files name it.`,
      alwaysThinks: (model: string) => `${model} always thinks; it cannot be turned off.`,
      neverThinks: (model: string) => `${model} has no thinking mode.`,
      /** Not a failure and not the model's fault: the *endpoint* has no field
       *  for it, so its own default stands (ADR-0021). */
      noThinkingSwitch: (label: string) => `${label} has no thinking switch.`,
      /** The second switch a turn carries (ADR-0026). Named for what it does to
       *  the answer, not for the field it becomes on any one endpoint. */
      webSearch: "Search the web",
      /** The one thing that can go wrong with it, and it is amber: the Action
       *  still runs, the endpoint just never hears the question. */
      noSearchSwitch: (label: string) => `${label} cannot search, so this is ignored.`,
      /** The model's own answer rather than the endpoint's, and the one that
       *  greys the switch: the vendor says this model does not take the field
       *  (ADR-0027). Amber and not an error — an Action file that already says
       *  so still runs, without a search. */
      modelCannotSearch: (model: string) =>
        `${model} cannot search here. Pick another model, or leave this off.`,
    },
    field: {
      /** The revert control on an overridden row, which names what it reverts to. */
      useDefault: (reading: string) => `Use the default (${reading})`,
      on: "On",
      off: "Off",
    },
  },
};

/**
 * The shape of a catalog: derived from the English one, so English is where a
 * string is added and every other catalog is checked against it.
 */
export type Strings = typeof EN;
