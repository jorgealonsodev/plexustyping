// The practice screen's hidden typing field, shared so other controls can hand the keyboard back,
// and the keyboard-navigation rules around it (accessibility S2).

import { t } from "./messages";
import type { Phase } from "./replies";

let field: HTMLElement | null = null;

/** Registers (or, with `null`, clears) the field that receives typed text. */
export function setTypingField(element: HTMLElement | null): void {
  field = element;
}

/** Gives the keyboard back to the typing field. */
export function focusTyping(): void {
  field?.focus({ preventScroll: true });
}

/** Whether the typing field has the keyboard. */
export function isTypingFocused(): boolean {
  return field !== null && document.activeElement === field;
}

/** Something focus can go back to (a control that opened a dialog). */
export type FocusTarget = { readonly isConnected: boolean; focus(options?: FocusOptions): void };

/** Gives focus back to the control that opened a dialog, or to the typing field when a shortcut did (or it is gone). */
export function returnFocus(opener: FocusTarget | null | undefined): void {
  if (opener?.isConnected) opener.focus({ preventScroll: true });
  else focusTyping();
}

/**
 * Whether Tab on the typing field types (indentation in a code text being typed). In prose and
 * words, and outside typing, Tab and Shift+Tab move focus as usual.
 */
export const tabTypes = (phase: Phase, isCode: boolean): boolean => isCode && phase === "typing";

/** Escape restarts a text being typed or reviewed, and only from the typing field. */
export const escapeRestarts = (onField: boolean, phase: Phase): boolean =>
  onField && (phase === "typing" || phase === "results");

/**
 * What Escape on the typing field does: an unlimited text being typed ends (unlimited S3); otherwise
 * it restarts as `escapeRestarts` says, and `null` does nothing.
 */
export function escapeAction(onField: boolean, phase: Phase, unlimited: boolean): "restart" | "end" | null {
  if (!escapeRestarts(onField, phase)) return null;
  return unlimited && phase === "typing" ? "end" : "restart";
}

type ShortcutKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat">;

/** A bare F6 moves focus from the typing field to the first header control, in every mode. */
export const isHeaderShortcut = (k: ShortcutKey): boolean =>
  k.key === "F6" && !k.ctrlKey && !k.altKey && !k.metaKey && !k.shiftKey;

/** A mouse click, not a keyboard activation (Enter or Space on a control reports `detail` 0). */
export const isPointerClick = (e: Pick<MouseEvent, "detail">): boolean => e.detail > 0;

/**
 * The control Tab (or Shift+Tab, `back`) moves to inside a panel that keeps the focus, wrapping at
 * both ends; from outside the list (`current` -1) it enters at the first (or last) one.
 */
export function nextFocusIndex(current: number, count: number, back: boolean): number | null {
  if (count === 0) return null;
  if (current < 0) return back ? count - 1 : 0;
  return (current + (back ? -1 : 1) + count) % count;
}

/** The hint under the practice text; names the keys that act on it (F6 leaves it for the header; unlimited S3: Esc ends). */
export function typingHint(started: boolean, isCode: boolean, unlimited = false): string {
  if (unlimited) return `${started ? "" : t("hintStart")}${t("hintUnlimited")}`;
  if (started) return t("hintStarted");
  return `${t("hintStart")}${isCode ? t("hintCode") : ""}${t("hintRestart")}`;
}
