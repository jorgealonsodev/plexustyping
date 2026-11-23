// What the screen reader mode (a11y S3) says in its polite `aria-live` region. Pure, so the wording is tested.
import { formatPercent, formatTime, formatWpm } from "./format";
import { SPOKEN_CHARS, type SpokenChar, spoken, t } from "./messages";

const isSpoken = (c: string): c is SpokenChar => (SPOKEN_CHARS as readonly string[]).includes(c);

/** How a char is spoken: invisible chars and punctuation by name, a capital letter as "e mayúscula" in Spanish. */
export function spokenChar(c: string): string {
  if (isSpoken(c)) return spoken(c);
  const lower = c.toLowerCase();
  return lower !== c ? t("capital", { char: lower }) : c;
}

/** «siguiente: a»; nothing once the text has no next char. */
export const nextCharMessage = (next: string | undefined) => (next === undefined ? "" : t("next", { char: spokenChar(next) }));

/** «error: esperaba e, pulsaste r». */
export const errorMessage = (expected: string, typed: string) =>
  t("error", { expected: spokenChar(expected), typed: spokenChar(typed) });

/** After a keystroke: the error of a wrong one, then the next char (after a wrong one only when the cursor moved). */
export function keystrokeMessage(
  wrong: { expected: string; typed: string } | null,
  next: string | undefined,
  moved: boolean,
): string {
  const following = nextCharMessage(next);
  if (!wrong) return following;
  const error = errorMessage(wrong.expected, wrong.typed);
  return moved && following ? `${error}. ${following}` : error;
}

/** The start of a new text: its length and the first char to type. */
export function newTextMessage(chars: readonly string[]): string {
  const intro = t("newText", { count: chars.length });
  const first = nextCharMessage(chars[0]);
  return first ? `${intro} ${first}` : intro;
}

/** The result line, with the figures the results screen shows first. */
export function resultMessage(stats: { netWpm: number; accuracy: number; durationMs: number; errors: number }): string {
  return t("result", {
    wpm: formatWpm(stats.netWpm),
    accuracy: formatPercent(stats.accuracy),
    time: formatTime(stats.durationMs),
    errors: t("errorCount", { count: stats.errors }),
  });
}

/** `message`, changed invisibly when it repeats `previous`: a live region announces only changed text. */
export const freshen = (message: string, previous: string) => (message === previous ? `${message}\u00a0` : message);

/** The live region's text: nothing while the setting is off. */
export const liveText = (enabled: boolean, message: string) => (enabled ? message : "");
