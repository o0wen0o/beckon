// Typed wrappers over invoke/listen. The only place window boundaries are
// crossed, so the surface stays reviewable.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import type {
  ActionFile,
  CapturePayload,
  Config,
  ConnectionReport,
  DeltaPayload,
  ErrorPayload,
  ExchangeIdPayload,
  Failure,
  InputPermission,
  InterruptedPayload,
  KeyStatus,
  ModelCatalog,
  PopoverView,
  Provider,
  RegistrySnapshot,
} from "./types";

// --- config ---------------------------------------------------------------

export const getConfig = () => invoke<Config>("get_config");
export const saveConfig = (config: Config) => invoke<void>("save_config", { config });
export const revealConfigDir = () => invoke<void>("reveal_config_dir");
/** The filled-in rows "Add from preset" offers (ADR-0021). */
export const getProviderPresets = () => invoke<Provider[]>("get_provider_presets");
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

/**
 * Every configured row's credential status, in one read — the Connection pane
 * draws the whole inventory at once (ADR-0021).
 */
export const getKeyStatuses = () => invoke<Record<string, KeyStatus>>("get_key_statuses");
/** One row's status, for the operations that change exactly one of them. */
export const getKeyStatus = (providerId: string) =>
  invoke<KeyStatus>("get_key_status", { providerId });
export const setApiKey = (providerId: string, key: string) =>
  invoke<KeyStatus>("set_api_key", { providerId, key });
export const deleteApiKey = (providerId: string) =>
  invoke<KeyStatus>("delete_api_key", { providerId });
export const testConnection = (providerId: string) =>
  invoke<ConnectionReport>("test_connection", { providerId });
/** Opens the row's own `key_page`; Rust refuses anything but `https`. */
export const openKeyPage = (providerId: string) =>
  invoke<void>("open_key_page", { providerId });

// --- models ---------------------------------------------------------------

/**
 * `live = false` answers without touching the network or the credential store —
 * from the cached list where there is one, and otherwise from whatever the
 * configuration names. `true` asks the endpoint, and falls back to the same.
 */
export const getModels = (providerId: string, live: boolean) =>
  invoke<ModelCatalog>("get_models", { providerId, live });

// --- hotkeys --------------------------------------------------------------

export const probeHotkey = (accelerator: string) => invoke<void>("probe_hotkey", { accelerator });

// --- platform -------------------------------------------------------------

/** Only macOS ever answers anything but `not-required`; see `InputPermission`. */
export const getInputPermission = () => invoke<InputPermission>("get_input_permission");
/** Opens a constant URL — the pane that grants it. */
export const openInputPermissionSettings = () =>
  invoke<void>("open_input_permission_settings");

// --- windows and exchanges ------------------------------------------------

export const getPopoverView = () => invoke<PopoverView | null>("get_popover_view");
export const pickAction = (actionId: string) => invoke<void>("pick_action", { actionId });
export const submitInput = (text: string) => invoke<string>("submit_input", { text });
export const followUp = (exchangeId: string, text: string) =>
  invoke<void>("follow_up", { exchangeId, text });
export const cancelExchange = (exchangeId: string) =>
  invoke<void>("cancel_exchange", { exchangeId });
export const retryExchange = (exchangeId: string) => invoke<void>("retry_exchange", { exchangeId });
/**
 * Runs the OS snip tool; the result arrives as `popover:capture` (ADR-0016).
 *
 * There is deliberately nothing worth awaiting and no "capturing" flag anywhere:
 * the window hides while the snip tool owns the screen, so it is not there to
 * show one.
 */
export const startCapture = () => invoke<void>("start_capture");
/** One tile's remove button, by position in the attached list (ADR-0017). */
export const discardCapture = (index: number) => invoke<void>("discard_capture", { index });
export const hidePopover = () => invoke<void>("hide_popover");
export const hideLauncher = () => invoke<void>("hide_launcher");
export const showSettings = () => invoke<void>("show_settings");
export const copyToClipboard = (text: string) => invoke<void>("copy_to_clipboard", { text });

// --- the Popover's own size (ADR-0018) ------------------------------------
/**
 * Which edge or corner of the window a grip drags (ADR-0018).
 *
 * Spelled out here rather than imported: `@tauri-apps/api/window` declares the
 * union but does not export it, and the member names are the plugin's wire
 * values.
 */
export type ResizeDirection =
  | "North"
  | "NorthEast"
  | "East"
  | "SouthEast"
  | "South"
  | "SouthWest"
  | "West"
  | "NorthWest";

/**
 * Hand a press on one of the Popover's grips to the window manager, which owns
 * the drag from there (ADR-0018). An undecorated window has no border for the OS
 * to hit-test, so this is the only way one is resized by pointer.
 */
export const startResizeDragging = (direction: ResizeDirection) =>
  getCurrentWindow().startResizeDragging(direction);

/**
 * Every resize of this window, in *physical* pixels — the user's drags and our
 * own `set_size` alike. Rust tells the two apart; see `remember_popover_size`.
 *
 * Physical because that is what the event carries and converting costs an IPC
 * round trip for the scale factor: this fires every frame of a drag and all but
 * the last one is thrown away, so `setPopoverSize` converts instead.
 */
export const onWindowResized = (fn: (width: number, height: number) => void) => {
  return getCurrentWindow().onResized(({ payload }) => fn(payload.width, payload.height));
};

/**
 * Remember the size the window was dragged to (ADR-0018), given the physical
 * size the resize event reported. Debounce it: a drag reports every pixel, and
 * each report Rust keeps is a write to `config.toml`.
 *
 * The scale factor is read here, once per settled drag rather than once per
 * frame, and read rather than cached: a window dragged onto a monitor with
 * different scaling has to be measured against the new factor.
 */
export const setPopoverSize = async (physicalWidth: number, physicalHeight: number) => {
  const window = getCurrentWindow();
  const logical = new PhysicalSize(physicalWidth, physicalHeight).toLogical(
    await window.scaleFactor(),
  );
  return invoke<void>("set_popover_size", { width: logical.width, height: logical.height });
};

// --- events ---------------------------------------------------------------

export const onActionsChanged = (fn: (snapshot: RegistrySnapshot) => void) =>
  listen<RegistrySnapshot>("actions-changed", (event) => fn(event.payload));
export const onConfigChanged = (fn: (config: Config) => void) =>
  listen<Config>("config-changed", (event) => fn(event.payload));
export const onPopoverView = (fn: () => void) => listen("popover:view", () => fn());
export const onPopoverCapture = (fn: (payload: CapturePayload) => void) =>
  listen<CapturePayload>("popover:capture", (event) => fn(event.payload));
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
