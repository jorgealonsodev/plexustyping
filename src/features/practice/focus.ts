// The practice screen's hidden typing field, shared so other controls can hand the keyboard back.

let field: HTMLElement | null = null;

/** Registers (or, with `null`, clears) the field that receives typed text. */
export function setTypingField(element: HTMLElement | null): void {
  field = element;
}

/** Gives the keyboard back to the typing field. */
export function focusTyping(): void {
  field?.focus({ preventScroll: true });
}
