import { formatNumber } from "../../i18n/i18n.svelte";
import { t } from "./messages";

/** `m:ss` from milliseconds. */
export function formatTime(ms: number): string {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export const formatWpm = (wpm: number) => String(Math.round(wpm));

/** A `0..=1` ratio as a percentage with at most one decimal, in the interface language (`90,5 %`, `90.5%`). */
export const formatPercent = (ratio: number) =>
  t("percent", { n: formatNumber(Math.round(ratio * 1000) / 10, { maximumFractionDigits: 1 }) });

const VISIBLE: Record<string, string> = { " ": "␣", "\n": "↵", "\t": "⇥" };

/** A char made visible in lists: a space shows as `␣`, a newline as `↵`, a tab as `⇥`. */
export const charLabel = (c: string) => VISIBLE[c] ?? c;
