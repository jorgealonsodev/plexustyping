// The "Ilimitado" length (unlimited S2, S3): a prose or words text that grows while it is typed.

import { type PracticeMode, isCodeMode } from "../code/api";
import { UNLIMITED_LENGTH } from "../settings/settings";

/** Chars of the first text, and about of each continuation, in unlimited mode. */
export const UNLIMITED_START = 600;
/** The continuation request carries at most this many chars of the current text (S4). */
export const TAIL_CHARS = 400;
/** After a failed continuation, the next one is asked this many chars later. */
export const RETRY_AFTER_CHARS = 50;

/** The length sent to `next_text`: 600 for "Ilimitado" (code ignores it either way). */
export const requestLength = (textLength: number) => (textLength === UNLIMITED_LENGTH ? UNLIMITED_START : textLength);

/** Whether a text grows while typed: "Ilimitado" in prose or words, never code or the diagnostic text. */
export const isUnlimited = (textLength: number, mode: PracticeMode, diagnostic: boolean) =>
  textLength === UNLIMITED_LENGTH && !isCodeMode(mode) && !diagnostic;

export interface ContinueState {
  cursor: number;
  /** Chars of the current text. */
  length: number;
  /** A continuation is under way. */
  pending: boolean;
  /** No new request before the cursor gets here (after a failure; 0 otherwise). */
  retryFrom: number;
}

/** Whether to ask for a continuation now: the cursor reached half of the current text, one at a time. */
export const shouldContinue = ({ cursor, length, pending, retryFrom }: ContinueState) =>
  length > 0 && !pending && cursor * 2 >= length && cursor >= retryFrom;

/** The last ≤ 400 chars (Unicode scalars) of the current text. */
export const continuationTail = (chars: readonly string[]) => chars.slice(-TAIL_CHARS).join("");

/** What to append: the continuation, trimmed, after a single space (none when the text already ends in one). */
export function continuationPiece(chars: readonly string[], more: string): string {
  const body = more.trim();
  if (!body) return "";
  const last = chars.at(-1);
  return last === undefined || /\s/.test(last) ? body : ` ${body}`;
}

/** The text a finished unlimited session is scored against: chars 0..cursor (S3). */
export const typedPrefix = (chars: readonly string[], cursor: number) => chars.slice(0, cursor).join("");

/** Ending with nothing typed discards the session unscored (S3). */
export const endAction = (cursor: number): "discard" | "score" => (cursor === 0 ? "discard" : "score");
