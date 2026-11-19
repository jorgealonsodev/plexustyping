import { afterEach, describe, expect, it } from "vitest";
import {
  escapeRestarts,
  isHeaderShortcut,
  isPointerClick,
  nextFocusIndex,
  returnFocus,
  setTypingField,
  tabTypes,
  typingHint,
} from "./focus";
import type { Phase } from "./replies";

const PHASES: Phase[] = ["loading", "typing", "scoring", "results", "error"];

type Key = Parameters<typeof isHeaderShortcut>[0];
const key = (k: string, extra: Partial<Key> = {}): Key => ({
  key: k,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
  repeat: false,
  ...extra,
});

/** A focusable stand-in that records how often it was focused. */
const target = (isConnected = true) => {
  const t = { isConnected, focused: 0, focus: () => void t.focused++ };
  return t;
};

afterEach(() => setTypingField(null));

describe("tabTypes", () => {
  it("types indentation on Tab in a code text being typed", () => {
    expect(tabTypes("typing", true)).toBe(true);
  });

  it("leaves Tab to the browser in prose and words texts, so focus moves on", () => {
    for (const phase of PHASES) expect(tabTypes(phase, false), phase).toBe(false);
  });

  it("leaves Tab to the browser in code outside typing", () => {
    for (const phase of PHASES.filter((p) => p !== "typing")) expect(tabTypes(phase, true), phase).toBe(false);
  });
});

describe("escapeRestarts", () => {
  it("restarts a text being typed or reviewed from the typing field", () => {
    expect(escapeRestarts(true, "typing")).toBe(true);
    expect(escapeRestarts(true, "results")).toBe(true);
  });

  it("never restarts from another control (e.g. the metronome toggle)", () => {
    for (const phase of PHASES) expect(escapeRestarts(false, phase), phase).toBe(false);
  });

  it("never restarts while a text loads, is scored or failed", () => {
    for (const phase of ["loading", "scoring", "error"] as Phase[]) expect(escapeRestarts(true, phase), phase).toBe(false);
  });
});

describe("isHeaderShortcut", () => {
  it("is a bare F6", () => {
    expect(isHeaderShortcut(key("F6"))).toBe(true);
  });

  it("ignores F6 with a modifier and other keys", () => {
    for (const mod of ["ctrlKey", "altKey", "metaKey", "shiftKey"] as const)
      expect(isHeaderShortcut(key("F6", { [mod]: true })), mod).toBe(false);
    for (const k of ["F5", "F2", "Tab", "Escape"]) expect(isHeaderShortcut(key(k)), k).toBe(false);
  });
});

describe("isPointerClick", () => {
  it("tells a mouse click from a keyboard activation (detail 0)", () => {
    expect(isPointerClick({ detail: 1 })).toBe(true);
    expect(isPointerClick({ detail: 2 })).toBe(true);
    expect(isPointerClick({ detail: 0 })).toBe(false);
  });
});

describe("nextFocusIndex", () => {
  it("moves forward and back, wrapping at both ends", () => {
    expect(nextFocusIndex(0, 3, false)).toBe(1);
    expect(nextFocusIndex(2, 3, false)).toBe(0);
    expect(nextFocusIndex(0, 3, true)).toBe(2);
    expect(nextFocusIndex(2, 3, true)).toBe(1);
  });

  it("enters at the first or last control from outside the list", () => {
    expect(nextFocusIndex(-1, 3, false)).toBe(0);
    expect(nextFocusIndex(-1, 3, true)).toBe(2);
  });

  it("has nowhere to go without controls", () => {
    expect(nextFocusIndex(-1, 0, false)).toBeNull();
  });
});

describe("returnFocus", () => {
  it("focuses the opener control when there is one", () => {
    const [opener, field] = [target(), target()];
    setTypingField(field as unknown as HTMLElement);
    returnFocus(opener);
    expect([opener.focused, field.focused]).toEqual([1, 0]);
  });

  it("focuses the typing field when a shortcut opened the dialog", () => {
    const field = target();
    setTypingField(field as unknown as HTMLElement);
    returnFocus(null);
    expect(field.focused).toBe(1);
  });

  it("focuses the typing field when the opener is gone", () => {
    const [opener, field] = [target(false), target()];
    setTypingField(field as unknown as HTMLElement);
    returnFocus(opener);
    expect([opener.focused, field.focused]).toEqual([0, 1]);
  });
});

describe("typingHint", () => {
  it("names F6 before and after the first keystroke, in every mode", () => {
    for (const started of [false, true]) for (const code of [false, true]) expect(typingHint(started, code)).toContain("F6");
  });

  it("keeps the existing keys: Esc, F4 and, in code, Intro and Tab", () => {
    expect(typingHint(false, false)).toBe("Empieza a escribir cuando quieras · Esc reinicia · F4 metrónomo · F6 menú");
    expect(typingHint(false, true)).toBe(
      "Empieza a escribir cuando quieras · Intro = salto de línea · Tab = sangría · Esc reinicia · F4 metrónomo · F6 menú",
    );
    expect(typingHint(true, false)).toBe("Esc reinicia el texto · F4 metrónomo · F6 menú");
  });
});
