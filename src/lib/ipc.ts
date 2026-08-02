// Typed wrappers over invoke/listen. The only place window boundaries are
// crossed, so the surface stays reviewable.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ActionFile,
  Config,
  DeltaPayload,
  ErrorPayload,
  ExchangeIdPayload,
  Failure,
  InterruptedPayload,
  KeyStatus,
  ModelCatalog,
  PopoverView,
  RegistrySnapshot,
} from "./types";

// --- config ---------------------------------------------------------------

export const getConfig = () => invoke<Config>("get_config");
export const saveConfig = (config: Config) => invoke<void>("save_config", { config });
export const revealConfigDir = () => invoke<void>("reveal_config_dir");
export const getStartupErrors = () => invoke<string[]>("get_startup_errors");

// --- actions --------------------------------------------------------------

export const getActions = () => invoke<RegistrySnapshot>("get_actions");
export const saveAction = (fileName: string, action: ActionFile) =>
  invoke<void>("save_action", { fileName, action });
export const createAction = (name: string) => invoke<string>("create_action", { name });
export const deleteAction = (fileName: string) => invoke<void>("delete_action", { fileName });
export const readActionRaw = (fileName: string) => invoke<string>("read_action_raw", { fileName });
export const writeActionRaw = (fileName: string, text: string) =>
  invoke<void>("write_action_raw", { fileName, text });

// --- secrets --------------------------------------------------------------

export const getKeyStatus = () => invoke<KeyStatus>("get_key_status");
export const setApiKey = (key: string) => invoke<KeyStatus>("set_api_key", { key });
export const deleteApiKey = () => invoke<KeyStatus>("delete_api_key");
export const testConnection = () => invoke<void>("test_connection");

// --- models ---------------------------------------------------------------

/**
 * `live = false` is the documented catalog, answered without touching the
 * network; `true` asks the endpoint and falls back to the same catalog.
 */
export const getModels = (live: boolean) => invoke<ModelCatalog>("get_models", { live });

// --- hotkeys --------------------------------------------------------------

export const probeHotkey = (accelerator: string) => invoke<void>("probe_hotkey", { accelerator });

// --- windows and exchanges ------------------------------------------------

export const getPopoverView = () => invoke<PopoverView | null>("get_popover_view");
export const pickAction = (actionId: string) => invoke<void>("pick_action", { actionId });
export const submitInput = (text: string) => invoke<string>("submit_input", { text });
export const followUp = (exchangeId: string, text: string) =>
  invoke<void>("follow_up", { exchangeId, text });
export const cancelExchange = (exchangeId: string) =>
  invoke<void>("cancel_exchange", { exchangeId });
export const retryExchange = (exchangeId: string) => invoke<void>("retry_exchange", { exchangeId });
export const hidePopover = () => invoke<void>("hide_popover");
export const hideLauncher = () => invoke<void>("hide_launcher");
export const showSettings = () => invoke<void>("show_settings");
/** The Launcher is editing an Action: suspend its hide-on-blur until it is not. */
export const setLauncherModal = (active: boolean) =>
  invoke<void>("set_launcher_modal", { active });
export const copyToClipboard = (text: string) => invoke<void>("copy_to_clipboard", { text });

// --- events ---------------------------------------------------------------

export const onActionsChanged = (fn: (snapshot: RegistrySnapshot) => void) =>
  listen<RegistrySnapshot>("actions-changed", (event) => fn(event.payload));
export const onConfigChanged = (fn: (config: Config) => void) =>
  listen<Config>("config-changed", (event) => fn(event.payload));
export const onPopoverView = (fn: () => void) => listen("popover:view", () => fn());
export const onSettingsOpened = (fn: () => void) => listen("settings:opened", () => fn());
export const onLauncherOpened = (fn: (selectionChars: number) => void) =>
  listen<{ selection_chars: number }>("launcher:opened", (event) =>
    fn(event.payload.selection_chars),
  );

export const onFirstToken = (fn: (payload: ExchangeIdPayload) => void) =>
  listen<ExchangeIdPayload>("exchange:first-token", (event) => fn(event.payload));
export const onDelta = (fn: (payload: DeltaPayload) => void) =>
  listen<DeltaPayload>("exchange:delta", (event) => fn(event.payload));
export const onDone = (fn: (payload: ExchangeIdPayload) => void) =>
  listen<ExchangeIdPayload>("exchange:done", (event) => fn(event.payload));
export const onExchangeError = (fn: (payload: ErrorPayload) => void) =>
  listen<ErrorPayload>("exchange:error", (event) => fn(event.payload));
export const onInterrupted = (fn: (payload: InterruptedPayload) => void) =>
  listen<InterruptedPayload>("exchange:interrupted", (event) => fn(event.payload));

/** Collect unlisten handles so a component can drop them all on destroy. */
export class Subscriptions {
  private pending: Promise<UnlistenFn>[] = [];

  add(promise: Promise<UnlistenFn>) {
    this.pending.push(promise);
    return this;
  }

  async dispose() {
    const handles = await Promise.all(this.pending);
    this.pending = [];
    for (const unlisten of handles) unlisten();
  }
}

/** Command errors arrive as strings or as `{kind, message}`. */
export function describeError(error: unknown): Failure {
  if (typeof error === "string") return { kind: "error", message: error };
  if (error && typeof error === "object" && "message" in error) {
    const failure = error as Partial<Failure>;
    return { kind: failure.kind ?? "error", message: String(failure.message) };
  }
  return { kind: "error", message: String(error) };
}
