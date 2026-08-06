// Kinds matter here: a rejected key is not an unreachable API, and neither is
// a missing credential (ADR-0005). One map for every consumer of a
// `Failure.kind`, so a new kind cannot reach one banner and miss another.
//
// It lives in `lib/` rather than under `settings/` because there are two
// consumers now: Settings' Connection banner and the Popover's failed turn.
// A provider's own string ("401 Unauthorized", a reqwest chain) is what the
// backend hands over, and a failure that reads one way in one window and
// another way in the next is the thing this file is for.
import type { Failure } from "./types";

export const FAILURE_PREFIX: Record<string, string> = {
  auth: "The API rejected this key",
  network: "Could not reach the API",
  http: "The API refused the request",
  "no-credential": "No API key stored",
  "read-error": "The Credential Manager could not be read",
  interrupted: "The answer stopped early",
  empty: "The endpoint listed no models",
  config: "Beckon is not configured for this",
};

/** `{kind, message}` as one sentence, with the cause named first. */
export function describeFailure(failure: Failure, fallback = "Failed"): string {
  return `${FAILURE_PREFIX[failure.kind] ?? fallback}: ${failure.message}`;
}
