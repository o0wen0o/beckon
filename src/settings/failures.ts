// Kinds matter here: a rejected key is not an unreachable API, and neither is
// a missing credential (ADR-0005). One map for every consumer of a
// `Failure.kind`, so a new kind cannot reach one banner and miss another.
import type { Failure } from "../lib/types";

export const FAILURE_PREFIX: Record<string, string> = {
  auth: "The API rejected this key",
  network: "Could not reach the API",
  "no-credential": "No API key stored",
  "read-error": "The Credential Manager could not be read",
  empty: "The endpoint listed no models",
};

/** `{kind, message}` as one sentence, with the cause named first. */
export function describeFailure(failure: Failure, fallback = "Failed"): string {
  return `${FAILURE_PREFIX[failure.kind] ?? fallback}: ${failure.message}`;
}
