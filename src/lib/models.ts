// Rust decides the option set — the catalog it derives from is the same table
// the request layer maps `thinking` with, so the dropdown can never offer a
// model that would then be refused. Nothing here re-derives it.
//
// Pure functions rather than component methods: the "the current value is
// always among the options" invariant is what stops a select from silently
// rewriting a configured model, and an invariant that load-bearing should be
// readable without a DOM around it.
import type { ModelCatalog, ModelOption } from "./types";

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
    { id: current, label: current, description: "", thinking: null, origin: "configured" },
    ...options,
  ];
}

export function modelOption(id: string, catalog: ModelCatalog | null): ModelOption | undefined {
  return modelOptions(id, catalog).find((option) => option.id === id);
}

/** A model only the config vouches for: say so instead of dropping it. */
export function unknownModelHint(id: string | null, catalog: ModelCatalog | null): string | null {
  if (!id) return null;
  if (modelOption(id, catalog)?.origin !== "configured") return null;
  const missing = catalog?.live
    ? "not in the endpoint's model list"
    : "not one of the models Beckon knows";
  return `${id} is ${missing}. Kept because your configuration names it.`;
}

/**
 * A `thinking` setting the model cannot honour. `deepseek.rs` makes this a hard
 * error at request time, so saying it here turns a failed Popover into a line
 * of amber text. It is a warning, not a block: the file is still valid, and the
 * base URL may point somewhere the catalog does not describe.
 */
export function thinkingWarning(
  model: string,
  thinking: boolean,
  catalog: ModelCatalog | null,
): string | null {
  const support = modelOption(model, catalog)?.thinking;
  if (support === "always-on" && !thinking) return `${model} always thinks; it cannot be turned off.`;
  if (support === "never" && thinking) return `${model} cannot think; the request would be refused.`;
  return null;
}
