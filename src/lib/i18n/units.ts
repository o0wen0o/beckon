// Units both catalogs format the same way.
//
// Here rather than inline in each string, because `captureMeta` and
// `captureSet` describe the same bytes from two places (ADR-0017): a rounding
// that differed between them would read as two different sizes for one
// screenshot. Neither the unit nor the digits are translated — `KB` is the
// symbol in both catalogs — so this is arithmetic, not prose.

/** An encoded byte count as whole kilobytes. */
export const kb = (bytes: number) => Math.round(bytes / 1024);
