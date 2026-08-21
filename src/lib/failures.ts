// Kinds matter: a rejected key is not an unreachable API, and neither is a
// missing credential (ADR-0005). One map for every consumer of a `Failure.kind`
// — Settings' Connection banner and the Popover's failed turn — so a new kind
// cannot reach one and miss the other. The map itself is `failure` in the
// catalogs (ADR-0015); what lives here is the sentence built from it.
import type { Strings } from "./i18n";
import type { Failure } from "./types";

/** `{kind, message}` as one sentence, with the cause named first. */
export function describeFailure(failure: Failure, t: Strings, fallback?: string): string {
  const prefix = t.failure[failure.kind] ?? fallback ?? t.failure.fallback;
  return `${prefix}: ${failure.message}`;
}
