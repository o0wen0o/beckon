// Rust decides the option set — the catalog it derives from is the same table
// the request layer maps `thinking` with, so the dropdown can never offer a
// model that would then be refused. Nothing here re-derives it.
//
// Pure functions rather than component methods: the "the current value is
// always among the options" invariant is what stops a select from silently
// rewriting a configured model, and an invariant that load-bearing should be
// readable without a DOM around it.
import type { Strings } from "./i18n";
import type { ModelCatalog, ModelOption, Provider } from "./types";

/**
 * The options to render, with `current` guaranteed present. Rust already
 * appends a configured-but-unknown model; this only covers the moment before
 * the catalog has arrived, so a select is never rendered without its own value
 * in it — a select whose value is missing would silently reset it.
 */
export function modelOptions(current: string, catalog: ModelCatalog | null): ModelOption[] {
  const options = catalog?.options ?? [];
  if (current === "" || options.some((option) => option.id === current)) return options;
  return [
    // `null` on both readings: this is the moment before the catalog arrived,
    // so nothing is known about the model either way — and unknown is not a no
    // (ADR-0027).
    {
      id: current,
      label: current,
      description: "",
      thinking: null,
      search: null,
      origin: "configured",
    },
    ...options,
  ];
}

export function modelOption(id: string, catalog: ModelCatalog | null): ModelOption | undefined {
  return modelOptions(id, catalog).find((option) => option.id === id);
}

/** A model only the config vouches for: say so instead of dropping it. */
export function unknownModelHint(
  id: string | null,
  catalog: ModelCatalog | null,
  t: Strings,
): string | null {
  if (!id) return null;
  if (modelOption(id, catalog)?.origin !== "configured") return null;
  return t.controls.model.unknown(id);
}

/**
 * Whether turning thinking *off* is something Beckon can express at this
 * endpoint at all (ADR-0021). `none` is most endpoints: there is no field to
 * send, so neither direction reaches the wire.
 */
export function canSuppressThinking(provider: Provider | undefined): boolean {
  return provider !== undefined && provider.reasoning !== "none";
}

/**
 * A `thinking` setting that will not reach the wire as asked.
 *
 * Two different situations, and the endpoint's comes first because it is the
 * broader fact: an endpoint with no thinking control ignores the setting whatever
 * the model is. Only one of the three is a hard error in `llm/request.rs` — a
 * model that *always* thinks, asked to stop — and the rest are amber, because
 * thinking that cannot be expressed costs the feature and not the turn.
 */
export function thinkingWarning(
  provider: Provider | undefined,
  model: string,
  thinking: boolean,
  catalog: ModelCatalog | null,
  t: Strings,
): string | null {
  if (thinking && !canSuppressThinking(provider)) {
    return t.controls.model.noThinkingSwitch(provider?.label ?? "");
  }
  const support = modelOption(model, catalog)?.thinking;
  if (support === "always-on" && !thinking) return t.controls.model.alwaysThinks(model);
  if (support === "never" && thinking) return t.controls.model.neverThinks(model);
  return null;
}

/**
 * Whether asking this endpoint to search the web reaches a field at all
 * (ADR-0026). `none` is most endpoints, and for two different reasons the pane
 * does not distinguish: no search, or a search that takes a second round trip
 * Beckon does not make.
 */
export function canWebSearch(provider: Provider | undefined): boolean {
  return provider !== undefined && provider.search !== "none";
}

/**
 * What a web search switch should say and whether it should still open, for one
 * endpoint-and-model pairing (ADR-0026, ADR-0027).
 *
 * Both readings come off one lookup because they are one question asked twice:
 * whether an ask would reach a field. `warning` is that question in the on
 * direction — amber, never an error, since searching that cannot be expressed
 * costs the feature and not the turn. `offOnly` is the same fact in the off
 * direction, and it is deliberately `false` whenever the setting is already on:
 * a `true` inherited from the row, or written into an Action file before the
 * model changed, has to stay clearable, and this switch is the only control
 * that could clear it.
 *
 * The endpoint's fact is checked first because it is the broader one — an
 * endpoint with no search field ignores the setting whatever the model is — and
 * it is also the answer while the catalog is still empty, which is the one
 * moment the option cannot speak for itself. The model's is the vendor's own
 * word about one pairing, which Rust folded into `ModelOption.search` via
 * `Search::supports_model`; `null` there is "not documented either way", which
 * is not a no.
 */
export function webSearchState(
  provider: Provider | undefined,
  model: string,
  webSearch: boolean,
  catalog: ModelCatalog | null,
  t: Strings,
): { warning: string | null; offOnly: boolean } {
  const reason = !canWebSearch(provider)
    ? t.controls.model.noSearchSwitch(provider?.label ?? "")
    : modelOption(model, catalog)?.search === false
      ? t.controls.model.modelCannotSearch(model)
      : null;
  return { warning: webSearch ? reason : null, offOnly: !webSearch && reason !== null };
}
