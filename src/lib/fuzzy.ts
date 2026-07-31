// Fuzzy matching for the Launcher. A dozen-odd Actions is the scale the README
// designs for, so a subsequence match with a few bonuses beats a dependency.

/**
 * Score `needle` against `haystack`. Returns null when the needle is not a
 * subsequence at all. Higher is better.
 */
export function fuzzyMatch(haystack: string, needle: string): number | null {
  if (needle.length === 0) return 0;

  const hay = haystack.toLowerCase();
  const pin = needle.toLowerCase();
  let score = 0;
  let at = 0;
  let previous = -2;

  for (const ch of pin) {
    if (ch === " ") continue; // spaces only separate; they never have to match
    const found = hay.indexOf(ch, at);
    if (found === -1) return null;

    // Consecutive characters and word starts are what people actually type.
    if (found === previous + 1) score += 8;
    if (found === 0 || /[\s\-_/]/.test(hay[found - 1])) score += 6;
    // Prefer matches near the start.
    score += Math.max(0, 4 - Math.floor(found / 4));

    previous = found;
    at = found + 1;
  }

  // A short haystack that matched is a tighter match than a long one.
  return score + Math.max(0, 12 - haystack.length / 4);
}

/** Rank items by the best match across their searchable fields. */
export function rank<T>(items: T[], query: string, fields: (item: T) => string[]): T[] {
  if (query.trim() === "") return items;
  const scored: { item: T; score: number }[] = [];

  for (const item of items) {
    let best: number | null = null;
    for (const [index, field] of fields(item).entries()) {
      const matched = fuzzyMatch(field, query);
      if (matched === null) continue;
      // Earlier fields (the name) outweigh later ones (the description).
      const score = matched - index * 5;
      if (best === null || score > best) best = score;
    }
    if (best !== null) scored.push({ item, score: best });
  }

  return scored.sort((a, b) => b.score - a.score).map((entry) => entry.item);
}
