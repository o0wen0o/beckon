// Mirrors the serde shapes in src-tauri. Rust is authoritative for all of it
// (ADR-0003) — nothing here is a local source of truth.

export type InputSource = "selection" | "prompt" | "auto";

export interface ModelParams {
  model: string;
  thinking: boolean;
  temperature: number;
}

export interface ModelOverrides {
  model: string | null;
  thinking: boolean | null;
  temperature: number | null;
}

export interface PromptSpec {
  system: string;
  user: string | null;
}

/** The on-disk fields of an Action file. */
export interface ActionFile {
  name: string;
  description?: string | null;
  input_source: InputSource;
  hotkey?: string | null;
  prompt: PromptSpec;
  model: ModelOverrides;
}

/** A loaded Action: file contents plus the identity derived from the filename. */
export interface Action extends ActionFile {
  id: string;
  file_name: string;
}

export interface ActionError {
  file_name: string;
  message: string;
}

export interface RegistrySnapshot {
  actions: Action[];
  errors: ActionError[];
  /** Action id ⇒ why its Direct Hotkey is not active. */
  hotkey_errors: Record<string, string>;
}

/** `system` follows the OS appearance; `light` is what an absent setting is. */
export type Theme = "light" | "dark" | "system";

/**
 * Which language every surface is written in; `en` is what an absent setting
 * is. There is no `system` arm the way `Theme` has one — the OS locale is a
 * guess about a reader rather than a setting (ADR-0015).
 */
export type Language = "en" | "zh";

export interface Config {
  launcher_hotkey: string;
  autostart: boolean;
  theme: Theme;
  language: Language;
  api: { base_url: string };
  defaults: ModelParams;
  /** The size the Popover is summoned at, in logical pixels (ADR-0018). Rust
   *  clamps it, so this is always a size a window can actually be. */
  popover: { width: number; height: number };
}

export type KeyStatus =
  | { kind: "present"; last4: string }
  | { kind: "no-credential" }
  | { kind: "read-error"; message: string };

/**
 * Whether the OS will let Beckon synthesise the copy keystroke the Selection is
 * grabbed with (ADR-0002, ADR-0013).
 *
 * `not-required` is Windows, and is not the same statement as `granted`: there
 * is nothing to grant, so Settings says nothing rather than reporting a
 * permission the user has never heard of.
 */
export type InputPermission = "not-required" | "granted" | "denied";

/** What a model does with thinking mode; `null` when Beckon knows nothing. */
export type ThinkingSupport = "switchable" | "always-on" | "never";

/**
 * Where one dropdown option came from. `configured` is the user's own value
 * that neither the catalog nor the endpoint vouches for — it is offered so that
 * it is never silently rewritten.
 */
export type ModelOrigin = "documented" | "live" | "configured";

export interface ModelOption {
  id: string;
  label: string;
  description: string;
  thinking: ThinkingSupport | null;
  origin: ModelOrigin;
}

/** `get_models` never fails; `fallback` says why the list is not the live one. */
export interface ModelCatalog {
  options: ModelOption[];
  live: boolean;
  fallback: Failure | null;
}

export type PopoverPhase = "needs-input" | "empty-selection" | "running";

/**
 * A screenshot the user grabbed with the OS snip tool, normalised to PNG in Rust
 * (ADR-0016). `data_url` is both the `<img src>` and the value that goes on the
 * wire — one copy, so the thumbnail cannot differ from what was sent.
 */
export interface Capture {
  data_url: string;
  width: number;
  height: number;
  /** Encoded PNG length, before base64. */
  bytes: number;
}

export interface PopoverView {
  action_id: string;
  action_name: string;
  model: ModelParams;
  phase: PopoverPhase;
  input: string | null;
  exchange_id: string | null;
  /** Attached and not yet sent, oldest first (ADR-0017). */
  captures: Capture[];
  /** What the last snip had to say, if it had anything. */
  capture_notice: CaptureNotice | null;
}

/**
 * What one run of the snip tool left to say (ADR-0016).
 *
 * One value rather than a flag beside an error, because the two can never both
 * stand: a run either attached a Capture, produced nothing, or produced bytes
 * that cannot be sent. `cancelled` is not an error — nothing was captured, so
 * nothing is being dropped — while `failed` carries the same `Failure` a
 * refused command does, for the same `describeFailure` to read.
 */
export type CaptureNotice =
  | { kind: "cancelled" }
  | { kind: "failed"; failure: Failure };

/** `popover:capture`: the fields of the view one snip can change. */
export interface CapturePayload {
  captures: Capture[];
  notice: CaptureNotice | null;
}

/** A command failure the UI reacts to by kind, not just by printing. */
export interface Failure {
  kind: string;
  message: string;
}

export interface DeltaPayload {
  exchange_id: string;
  content: string;
  reasoning: string;
}

export interface ExchangeIdPayload {
  exchange_id: string;
}

export interface ErrorPayload extends ExchangeIdPayload {
  kind: string;
  message: string;
}

export interface InterruptedPayload extends ExchangeIdPayload {
  message: string;
}
