// Decisions on late IPC replies and on settings changes while a text is under way.

/** Phases of the practice screen. */
export type Phase = "loading" | "typing" | "scoring" | "results" | "error";

/** A reply is applied only if it answers the latest request of its kind; a stale one is ignored. */
export const shouldApply = (replyRun: number, currentRun: number): boolean => replyRun === currentRun;

/**
 * Whether an indentation change restarts the current text: only a code text being typed.
 * A text loading, being scored or reviewed keeps that work; the new mode applies to the next one.
 */
export const indentChangeRestarts = (phase: Phase, isCode: boolean): boolean => isCode && phase === "typing";

type ShortcutKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat">;

/** A bare F5 loads a new text (instead of reloading the app window). */
export const isNewTextShortcut = (k: ShortcutKey) =>
  k.key === "F5" && !k.ctrlKey && !k.altKey && !k.metaKey && !k.shiftKey && !k.repeat;
