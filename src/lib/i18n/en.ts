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
   *  miss the other. `fallback` is the unknown-kind case. */
  failure: {
    auth: "The API rejected this key",
    network: "Could not reach the API",
    http: "The API refused the request",
    "no-credential": "No API key stored",
    "read-error": `The ${IS_MAC ? "Keychain" : "Windows Credential Manager"} could not be read`,
    interrupted: "The answer stopped early",
    empty: "The endpoint listed no models",
    config: "Beckon is not configured for this",
    "capture-too-large": "The screenshot is too big to send",
    "capture-too-many": "There is no room for another screenshot",
    "capture-unreadable": "The screenshot could not be read",
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
    widen: "Backspace to widen the search.",
    selected: (characters: number) => `${characters} characters selected`,
    noSelection: "No selection",
    move: "move",
    run: "run",
    settingsTitle: (accelerator: string) => `Settings (${accelerator})`,
  },

  popover: {
    close: "Close",
    thinking: "thinking",
    nothingToShow: "Nothing to show.",
    /** `{name}` is the Action, set bold by the caller. */
    typeYourInput: "Type what you want to send to {name}.",
    waiting: "Waiting for the first token",
    runningWaiting: "Waiting",
    runningStreaming: "Streaming",
    stop: "Stop",
    firstInput: "Your input…",
    followUp: "Ask a follow-up…",
    send: "Send",
    showAll: "Show all",
    showLess: "Show less",
    showThinking: "Show what it thought",
    hideThinking: "Hide",
    failed: "Failed",
    interrupted: "Interrupted",
    interruptedEmpty: "Interrupted before any output",
    cancelled: "Cancelled.",
    retry: "Retry",
    openSettings: "Open Settings",
    copy: "Copy",
    copied: "Copied",

    /** The Capture (CONTEXT.md) is a screenshot in prose — nobody calls it a
     *  Capture out loud, and the term is for code and comments. */
    captureTooltip: (accelerator: string) => `Screenshot the screen (${accelerator})`,
    removeCapture: "Remove the screenshot",
    captureCancelled: "Nothing was captured.",
    captureRetry: (accelerator: string) => `Press ${accelerator} to try again.`,
    captureNote: "Add a note about the screenshots…",
    /** Beside the thumbnail: what was captured, and how big it is. Takes the
     *  raw byte count, so the two thumbnails cannot round it differently. */
    captureMeta: (width: number, height: number, bytes: number) =>
      `${width}×${height} · PNG ${kb(bytes)} KB`,
    /** Under the rail: what is attached, as a set. The individual sizes are one
     *  tap away in the preview, and four of them side by side read as noise. */
    captureSet: (count: number, bytes: number) =>
      `${count} screenshot${count === 1 ? "" : "s"} · ${kb(bytes)} KB total`,
    viewCapture: "View this screenshot",
    /** Said in words for a screen reader; the preview draws it as `2 / 3`. */
    capturePosition: (index: number, total: number) => `Screenshot ${index} of ${total}`,
    previousCapture: "Previous screenshot",
    nextCapture: "Next screenshot",
    closeCapture: "Close the screenshot",
    /** On the image itself, because zoom has no visible control of its own —
     *  the wheel and a click are the whole interface (ADR-0017). */
    zoomHint: "Click to zoom, scroll to zoom in and out",
    zoomOutHint: "Click to fit, drag to move",
  },

  settings: {
    nav: {
      label: "Settings",
      connection: "Connection",
      actions: "Actions",
      triggering: "Triggering",
      appearance: "Appearance",
      attention: "Something in this section needs attention",
      openFolder: "Open folder",
    },

    status: {
      notSaved: (message: string) => `Not saved — ${message}`,
      saving: "Saving…",
      standing: "Changes are written to disk as you make them.",
      rawFile: "This file does not parse, so it is written with the button above — not as you type.",
    },

    connection: {
      title: "Connection",
      /** `fallback` is the default row's label, `store` what the OS calls its
       *  credential store. Both are in the sentence because both are the answer
       *  to "where does my text go and what does it carry" (ADR-0021). */
      lede: (fallback: string, store: string) =>
        `The endpoints you keep. Each Action posts to one; an Action that does not say posts to ${fallback}. Keys live in the ${store}, one per endpoint, never in a file.`,
      welcomeLead: "Welcome.",
      welcomeBody:
        " DeepSeek is set up and waiting for a key. Any OpenAI-compatible endpoint works instead, at its own vendor's host — and a local one needs no key at all.",
      setUp: (label: string) => `Set up ${label}`,
      getKeyFrom: (host: string) => `Get a key from ${host}`,
      addPreset: "Add from preset…",
      addBlank: "Blank",
      /** The badge on the default row. Sentence case, not an uppercase eyebrow:
       *  it is a word about the row it sits on, and it says something quiet —
       *  which row an Action inherits, not a state the row is in. */
      defaultTag: "Default",
      defaultForNew: "default for Actions that do not say",
      makeDefault: "Make default",
      usedByNone: "no Actions",
      usedByOne: (name: string) => `1 Action — ${name}`,
      usedByMany: (count: number) => `${count} Actions`,
      staysLocal: "stays on this machine",
      missingKey: "no key",
      edit: (label: string) => `Edit ${label}`,
      removeLabel: (label: string) => `Remove ${label}`,
      removeHint: "Removes the row and its stored key.",
      /** Refused rather than cascaded: which endpoint those Actions should use
       *  instead is not a decision this button gets to make. */
      removeBlocked: (names: string) =>
        `${names} would be left without an endpoint. Point them elsewhere first.`,
      removeLast: "The last endpoint cannot be removed.",
      back: "Connection",
      unnamed: "(unnamed)",

      endpoint: "Endpoint",
      name: "Name",
      nameHint:
        "Display only. The id beside it is what an Action names and what the stored key is filed under — change that in config.toml, and move the key with it.",
      baseUrl: "Base URL",
      baseUrlHint:
        "Any OpenAI-compatible endpoint, at its own vendor's host. Requests go to /v1/chat/completions; a trailing /v1 is tolerated.",
      apiKey: "API key",
      apiKeyHint:
        "A stored key is sent as a Bearer header. Nothing stored means no header — which is what a local endpoint wants.",
      save: "Save",
      saved: "Saved.",
      remove: "Remove",
      removed: "Removed.",
      stored: "Stored — ends in",
      noKeyYet: "No key stored yet.",
      unauthenticated: "No key — requests go unauthenticated.",
      readError: (store: string, message: string) =>
        `The ${store} could not be read: ${message}. Save the key again to recreate the credential.`,
      reachability: "Reachability",
      reachabilityHint: "Sends one small request to this endpoint, with its own key.",
      test: "Test connection",
      testing: "Testing…",
      testOk: "Reached it — the endpoint answered.",

      rowDefaults: "Defaults for this endpoint",
      rowDefaultsNote: "an Action may override either",
      refresh: "Refresh models",
      loading: "Loading models…",
      live: "Listed by this endpoint.",
      catalogNotice: (cause: string) => `${cause} — showing what your configuration names.`,
      catalogFallback: "The model list could not be fetched",

      /** The wire field, on a row the user typed themselves. A preset carries
       *  the right value already, and showing it there invites breaking it. */
      reasoning: "Thinking control",
      reasoningName: {
        deepseek: "DeepSeek",
        qwen: "Qwen",
        none: "None",
      },
      reasoningHint: {
        deepseek:
          'Sends thinking:{type:"enabled"|"disabled"}. DeepSeek V4 reasons unless told not to.',
        qwen: "Sends chat_template_kwargs.enable_thinking. Qwen3 behind vLLM, SGLang or DashScope.",
        none: "Nothing sent either way — the endpoint's own default stands. Right for every endpoint with no such switch, which is most of them.",
      },
      thinkingHint: (label: string) =>
        `Off is sent to ${label} explicitly, because its models reason unless told not to.`,
      thinkingHintNone: (label: string) =>
        `${label} has no switch Beckon knows how to throw, so nothing is sent either way.`,
      thisEndpoint: "This endpoint",
    },

    triggering: {
      title: "Triggering",
      lede: "How Beckon is summoned. Every hotkey is registered the moment you record it.",
      hotkeyDeadLead: "A hotkey is not active.",
      hotkeyDeadBody: "Record a different combination below; it is registered the moment you record it.",
      permissionLead: "Beckon cannot read the Selection.",
      permissionBody:
        " Grabbing it means sending a Cmd+C to whatever is in front, and macOS allows that only for an app you have trusted under Privacy & Security → Accessibility.",
      permissionStillWorks:
        "Hotkeys still fire and Actions that ask you to type still work. Turn Beckon on in the list, then come back to this window.",
      openAccessibility: "Open Accessibility settings",
      summoning: "Summoning",
      launcherHotkey: "Launcher hotkey",
      launcherHotkeyHint: "If the combination is already taken it goes red and is not saved.",
      autostartHint: (tray: string) => `Beckon lives in the ${tray}; starting with the machine is the point.`,
    },

    appearance: {
      title: "Appearance",
      lede: "Applies to the Launcher, the Popover and this window at once.",
      theme: "Theme",
      light: "Light",
      dark: "Dark",
      system: "System",
      themeHint: (appearance: string) =>
        `Beckon starts light unless you say otherwise. “System” is the only setting that reads the ${appearance}, and it follows it live.`,
      language: "Language",
      /** Each language names itself, in itself: a reader who cannot read the
       *  current one still has to be able to find their own. */
      english: "English",
      chinese: "中文",
      languageHint:
        "Applies to every window at once, and to the tray. Beckon starts in English unless you say otherwise; your Actions are your own words and are never translated.",
    },


    actions: {
      title: "Actions",
      lede: "One Action is one prompt, stored as its own file. The filename is its identity; the name is only what you see.",
      create: "New Action",
      count: (count: number) => `${count} ${count === 1 ? "Action" : "Actions"}`,
      willNotParse: "Will not parse",
      empty: "No Actions yet. One Action is one prompt, stored as its own file.",
      gone: "That Action is gone.",
      rawUnder: "does not parse — edited as text",
      back: "Back to Actions",
      backTo: (name: string) => `Back to ${name}`,
      deleteTitle: (name: string) => `Delete “${name}”?`,
      /** `{file}` is the filename, set in mono by the caller. */
      deleteBody: "The file {file} is removed from disk. This cannot be undone.",
      deleteConfirm: "Delete file",

      hotkeyDeadLead: "This Action’s Direct Hotkey is not active.",
      hotkeyDeadBody: "No change to this Action can be saved until the hotkey is cleared or changed.",
      clearHotkey: "Clear the Direct Hotkey",

      definition: "Definition",
      definitionHint: "The name, what it is for, and the two prompts it sends.",
      trigger: "Trigger",
      inputSource: "Input Source",
      sourceHint: {
        prompt: "Uses typed input only. Any Selection is ignored.",
        auto: "Uses the Selection if there is one, otherwise asks for typed input.",
      },
      directHotkey: "Direct Hotkey",
      directHotkeyHint: "Optional. Without one, the Action is Launcher-only.",

      overrides: "Model overrides",
      overridesNote: (label: string) => `Unmarked rows follow ${label}`,
      /** The row ADR-0021 added, and the reason there is no global switch. */
      provider: "Endpoint",
      providerHint:
        "Where this Action posts. Leave it unmarked and it follows the default; set it and this Action stops following.",
      providerLocal: (label: string) => `${label} — local`,
      /** A model pinned before the endpoint changed. Kept, never rewritten. */
      strandedModel: (model: string, label: string, fallback: string) =>
        `${model} is not in ${label}’s list. It is kept, not rewritten — revert to use ${fallback}.`,
      needsKey: (label: string) =>
        `No key is stored for ${label}, so this Action would fail before its request leaves. Every other Action is unaffected.`,
      sends: "What this Action sends",
      thinkingHint:
        "Adds seconds before the first word. Worth it where the Action needs the model to reason, not where it reformats.",

      thisFile: "This file",
      deleteLabel: "Delete this Action",
      deleteHint: (file: string) => `Removes ${file} from disk. This cannot be undone.`,
      deleteButton: "Delete Action",

      name: "Name",
      nameWarning: "Without a name this Action shows as its file name in the Launcher.",
      description: "Description",
      descriptionHint: "Shown under the name in the Launcher, and searched.",
      systemPrompt: "System prompt",
      systemPromptHint: "How the model should behave. Sent ahead of every input.",
      userTemplate: "User template",
      userTemplateHint:
        "{{input}} is replaced by the Selection or the typed input. Empty means just the input.",
      templateWarning: "This template never includes the input.",
      templateWarningShort: "The user template never includes the input.",

      saveFile: "Save file",
      reloadsWhenItParses: "It reloads the moment it parses.",
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
      needsModifier: (advice: string) => `Add ${advice} — a bare key would fire everywhere.`,
    },
    model: {
      /** The two control names, here rather than under a pane: both the endpoint
       *  screen and an Action's overrides draw the same row (ADR-0021). */
      label: "Model",
      thinking: "Think before answering",
      configuredGroup: "Named by your configuration",
      unknownLive: "not in the endpoint's model list",
      unknownCatalog: "not one of the models Beckon knows",
      unknown: (model: string, why: string) => `${model} is ${why}. Kept because your configuration names it.`,
      alwaysThinks: (model: string) => `${model} always thinks; it cannot be turned off.`,
      neverThinks: (model: string) =>
        `${model} has no thinking mode; nothing is sent about it either way.`,
      /** Not a failure and not the model's fault: the *endpoint* has no field
       *  for it, so its own default stands (ADR-0021). */
      noThinkingSwitch: (label: string) =>
        `${label} has no thinking switch Beckon can throw; its own default stands.`,
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
