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
