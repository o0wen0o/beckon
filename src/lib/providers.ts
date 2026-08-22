// Reading a provider row: the small derivations three surfaces need, in one
// place (ADR-0021).
//
// Every function here **mirrors a rule in Rust**, and each one names its
// counterpart. That duplication is deliberate and bounded: Rust is authoritative
// (ADR-0003) — it decides what goes on the wire and whether a turn runs — while
// these decide what a pane *says*, which is a question no round trip should have
// to answer per row while a list is being drawn. Nothing here changes a value;
// they only read one.
//
// Adding a rule to this file means adding it in `config.rs` too, and a rule that
// cannot be stated twice without drifting belongs only in Rust, reaching the
// window as a field.
import type { KeyStatus, Provider } from "./types";

/** The host, for a row that has no room for the whole URL. */
export function host(url: string): string {
  return url.trim().replace(/^https?:\/\//, "").replace(/\/+$/, "");
}

/**
 * Where a turn actually posts, as `client::api_url` computes it: a `base_url`
 * that already carries the version segment does not get a second one.
 *
 * Rendered rather than assembled at each call site, because
 * `api.openai.com/v1/v1/chat/completions` is exactly what a pane that forgets
 * the rule shows — and it did, until a prototype drew it.
 */
export function chatUrl(provider: Provider): string {
  const base = host(provider.base_url);
  return base.endsWith("/v1") ? `${base}/chat/completions` : `${base}/v1/chat/completions`;
}

/**
 * Whether a missing key here is a local setup rather than a mistake. Mirrors
 * `Provider::is_local`, including the one range whose prefix is not a literal.
 *
 * Loopback and the three private ranges only: a host we cannot place is treated
 * as remote, because sending nothing to something that wanted a key fails as a
 * 401 the user then has to decode.
 */
export function isLocal(provider: Provider): boolean {
  const where = host(provider.base_url).toLowerCase();
  if (/^(localhost|127\.|0\.0\.0\.0|\[::1\]|192\.168\.|10\.)/.test(where)) return true;
  const octet = /^172\.(\d{1,3})\./.exec(where);
  return octet !== null && Number(octet[1]) >= 16 && Number(octet[1]) <= 31;
}

/**
 * The Actions that resolve to each row, grouped in one pass.
 *
 * This is the answer to "where does my text go", which a global switch used to
 * give for free and this design has to earn back on purpose. Mirrors the
 * fallback in `ModelOverrides::merge_over` — and, in Rust, `Config::provider_id`
 * — so the rule is written here once rather than at each call site: an Action
 * naming no provider is on the default row.
 *
 * Grouped rather than filtered per row because the inventory draws a count on
 * every row on every notify, and a filter each is O(rows x actions) with a fresh
 * array per row, to display N numbers.
 */
export function actionsByProvider<T extends { model: { provider: string | null } }>(
  actions: T[],
  defaultProvider: string,
): Map<string, T[]> {
  const grouped = new Map<string, T[]>();
  for (const action of actions) {
    const id = action.model.provider ?? defaultProvider;
    const list = grouped.get(id);
    if (list) list.push(action);
    else grouped.set(id, [action]);
  }
  return grouped;
}

/** The Actions that resolve to one row — the count that row carries. */
export function actionsUsing<T extends { model: { provider: string | null } }>(
  providerId: string,
  actions: T[],
  defaultProvider: string,
): T[] {
  return actionsByProvider(actions, defaultProvider).get(providerId) ?? [];
}

/**
 * What is wrong with one row's credential, or `null` if nothing is.
 *
 * Four panes were each assembling this from `isLocal` plus a status lookup, and
 * they had already drifted — one counted only `no-credential` where the others
 * counted anything that was not `present`. Mirrors the split in
 * `commands::require_api_key`: a local endpoint wants no `Authorization` header,
 * so nothing stored for one is a working setup rather than a fault (ADR-0021),
 * and the two remaining outcomes stay two different things all the way to the UI
 * (ADR-0005).
 *
 * `null` while the status has not arrived: nothing is claimed before it is known.
 */
export function keyProblem(
  provider: Provider,
  status: KeyStatus | undefined,
): "missing" | "unreadable" | null {
  if (status === undefined) return null;
  // A store that cannot be read is a fault on a local row too: the answer is
  // unknown rather than "no header needed".
  if (status.kind === "read-error") return "unreadable";
  if (status.kind === "present" || isLocal(provider)) return null;
  return "missing";
}

/**
 * A model an Action names that its own endpoint's list does not offer.
 *
 * It is *kept*, never rewritten — the rule everywhere in this codebase — so this
 * exists to say so out loud beside a revert control. `null` while the catalog
 * has not arrived: nothing is claimed before it is known.
 */
export function strandedModel(
  model: string,
  options: { id: string }[] | undefined,
): string | null {
  if (model === "" || options === undefined || options.length === 0) return null;
  return options.some((one) => one.id === model) ? null : model;
}

/** A blank row: no reasoning wire, no key, which is a working local endpoint. */
export function blankProvider(existing: Provider[]): Provider {
  let n = existing.length + 1;
  // The id is the credential account, so it has to be unique — a collision
  // would hand a fresh row somebody else's key.
  while (existing.some((one) => one.id === `endpoint-${n}`)) n += 1;
  return {
    id: `endpoint-${n}`,
    label: `Endpoint ${n}`,
    base_url: "http://localhost:8000/v1",
    model: "",
    thinking: false,
    reasoning: "none",
  };
}
