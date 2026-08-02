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

/**
 * Which characters of `haystack` the needle matched, for highlighting. Walks
 * the same subsequence as `fuzzyMatch` and returns the positions instead of a
 * score; returns null on the same non-match.
 *
 * Kept separate rather than folded into `fuzzyMatch`: ranking runs over every
 * Action on every keystroke and has no use for the positions, while
 * highlighting runs only over the rows actually rendered.
 */
export function fuzzyMatchIndices(haystack: string, needle: string): number[] | null {
  if (needle.length === 0) return [];

  const hay = haystack.toLowerCase();
  const pin = needle.toLowerCase();
  const indices: number[] = [];
  let at = 0;

  for (const ch of pin) {
    if (ch === " ") continue;
    const found = hay.indexOf(ch, at);
    if (found === -1) return null;
    indices.push(found);
    at = found + 1;
  }

  return indices;
}

/**
 * Split `text` into runs, flagging the ones the needle matched. One array walk
 * in the component instead of a per-character element.
 */
export function highlight(text: string, needle: string): { text: string; hit: boolean }[] {
  const indices = fuzzyMatchIndices(text, needle);
  if (indices === null || indices.length === 0) return [{ text, hit: false }];

  const hits = new Set(indices);
  const runs: { text: string; hit: boolean }[] = [];

  for (let index = 0; index < text.length; index += 1) {
    const hit = hits.has(index);
    const last = runs[runs.length - 1];
    if (last && last.hit === hit) last.text += text[index];
    else runs.push({ text: text[index], hit });
  }

  return runs;
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
