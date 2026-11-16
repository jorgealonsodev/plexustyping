import { keyForChar } from "./keys";
import type { Layout } from "./types";

/** Score of one char, as the `heatmap` command returns it. */
export interface CharScore {
  ch: string;
  score: number;
}

/** Number of tint steps above zero; each has a `.heat-N` class. */
export const HEAT_LEVELS = 10;

/**
 * The highest score of the chars each key produces, on any layer; a composed char
 * (`á`, `ü`) counts for its base key. Chars no key produces are skipped.
 */
export function heatScores(layout: Layout, scores: CharScore[]): Map<string, number> {
  const heat = new Map<string, number>();
  for (const { ch, score } of scores) {
    const code = keyForChar(layout, ch);
    if (code) heat.set(code, Math.max(heat.get(code) ?? 0, score));
  }
  return heat;
}

/** Tint step of a 0..1 score: 0 (no tint) to `HEAT_LEVELS`. */
export function heatLevel(score: number): number {
  if (!(score > 0)) return 0;
  return Math.min(HEAT_LEVELS, Math.round(score * HEAT_LEVELS));
}
