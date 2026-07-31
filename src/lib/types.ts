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

/** `system` follows the Windows app theme; `light` is what an absent setting is. */
export type Theme = "light" | "dark" | "system";

export interface Config {
  launcher_hotkey: string;
  autostart: boolean;
  theme: Theme;
  api: { base_url: string };
  defaults: ModelParams;
}

export type KeyStatus =
  | { kind: "present"; last4: string }
  | { kind: "no-credential" }
  | { kind: "read-error"; message: string };

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

export interface PopoverView {
  action_id: string;
  action_name: string;
  model: ModelParams;
  phase: PopoverPhase;
  input: string | null;
  exchange_id: string | null;
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
