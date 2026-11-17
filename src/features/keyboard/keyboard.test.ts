import { describe, expect, it } from "vitest";
import { ANSI_ROWS, ISO_ROWS, NAMED_LABELS, fingerOf, handOf, rowsFor } from "./geometry";
import { highlightsFor, keyForChar, keyLabels } from "./keys";
import type { KeyPress, Layout } from "./types";

/** A slice of es-qwerty as `get_layout` serialises it. */
const LAYOUT: Layout = {
  id: "es-qwerty",
  name: "Español QWERTY",
  deadKeys: ["´", "`", "^", "¨"],
  keys: {
    Digit2: { normal: "2", shift: '"', altgr: "@" },
    KeyA: { normal: "a", shift: "A", altgr: null },
    KeyU: { normal: "u", shift: "U", altgr: null },
    Semicolon: { normal: "ñ", shift: "Ñ", altgr: null },
    Quote: { normal: "´", shift: "¨", altgr: "{" },
    Space: { normal: " ", shift: null, altgr: null },
  },
};

const press = (code: string, layer: KeyPress["layer"] = "normal"): KeyPress => ({ code, layer });

describe("ISO geometry", () => {
  it("has the five ISO rows in order, from Backquote to the Ctrl row", () => {
    expect(ISO_ROWS.map((row) => [row[0].code, row.at(-1)!.code])).toEqual([
      ["Backquote", "Backspace"],
      ["Tab", "Enter"],
      ["CapsLock", "Enter"],
      ["ShiftLeft", "ShiftRight"],
      ["ControlLeft", "ControlRight"],
    ]);
    expect(ISO_ROWS[2].at(-2)!.code).toBe("Backslash");
    expect(ISO_ROWS[3].slice(0, 3).map((k) => k.code)).toEqual(["ShiftLeft", "IntlBackslash", "KeyZ"]);
  });

  it("gives every row the same width", () => {
    for (const row of ISO_ROWS) expect(row.reduce((sum, k) => sum + k.width, 0)).toBe(15);
  });

  it("draws every key once, except the two-row ISO Enter", () => {
    const codes = ISO_ROWS.flat().map((k) => k.code);
    expect(codes.filter((c) => c === "Enter")).toHaveLength(2);
    expect(new Set(codes).size).toBe(codes.length - 1);
  });

  it("assigns a touch-typing finger to every key", () => {
    for (const { code } of ISO_ROWS.flat()) expect(fingerOf(code), code).not.toBeNull();
    expect(["KeyA", "KeyS", "KeyD", "KeyF", "KeyJ", "KeyK", "KeyL", "Semicolon", "Space"].map(fingerOf)).toEqual([
      "l-pinky",
      "l-ring",
      "l-middle",
      "l-index",
      "r-index",
      "r-middle",
      "r-ring",
      "r-pinky",
      "thumb",
    ]);
    expect(["KeyG", "KeyB", "Digit5", "KeyH", "Digit6", "KeyN"].map(fingerOf)).toEqual([
      "l-index",
      "l-index",
      "l-index",
      "r-index",
      "r-index",
      "r-index",
    ]);
    expect(["IntlBackslash", "Quote", "Backslash", "Enter", "ShiftRight"].map(fingerOf)).toEqual([
      "l-pinky",
      "r-pinky",
      "r-pinky",
      "r-pinky",
      "r-pinky",
    ]);
    expect(fingerOf("F2")).toBeNull();
  });

  it("marks only the lower part of the ISO Enter as unlabelled", () => {
    expect(ISO_ROWS.flat().filter((k) => k.unlabelled)).toEqual([{ code: "Enter", width: 1.25, unlabelled: true }]);
    expect(ANSI_ROWS.flat().some((k) => k.unlabelled)).toBe(false);
  });

  it("has an ANSI shape: one-row Enter, backslash above it and no key beside the left Shift", () => {
    expect(ANSI_ROWS.map((row) => [row[0].code, row.at(-1)!.code])).toEqual([
      ["Backquote", "Backspace"],
      ["Tab", "Backslash"],
      ["CapsLock", "Enter"],
      ["ShiftLeft", "ShiftRight"],
      ["ControlLeft", "ControlRight"],
    ]);
    for (const row of ANSI_ROWS) expect(row.reduce((sum, k) => sum + k.width, 0)).toBe(15);
    const codes = ANSI_ROWS.flat().map((k) => k.code);
    expect(new Set(codes).size).toBe(codes.length);
    expect(codes).not.toContain("IntlBackslash");
    expect(ANSI_ROWS[3].slice(0, 2)).toEqual([
      { code: "ShiftLeft", width: 2.25 },
      { code: "KeyZ", width: 1 },
    ]);
    expect(ANSI_ROWS[0]).toEqual(ISO_ROWS[0]);
    expect(ANSI_ROWS[4]).toEqual(ISO_ROWS[4]);
    for (const { code } of ANSI_ROWS.flat()) expect(fingerOf(code), code).not.toBeNull();
  });

  it("draws ANSI only for a loaded layout without the ISO key", () => {
    const { IntlBackslash: _, ...ansiKeys } = { ...LAYOUT.keys, IntlBackslash: { normal: "<", shift: ">", altgr: null } };
    expect(rowsFor(null)).toBe(ISO_ROWS);
    expect(rowsFor({ ...LAYOUT, keys: { ...ansiKeys, IntlBackslash: { normal: "<", shift: ">", altgr: null } } })).toBe(ISO_ROWS);
    expect(rowsFor({ ...LAYOUT, keys: ansiKeys })).toBe(ANSI_ROWS);
  });

  it("knows the hand of each finger", () => {
    expect([handOf("l-index"), handOf("r-pinky"), handOf("thumb")]).toEqual(["left", "right", null]);
  });

  it("names the keys the layout does not label, in Spanish", () => {
    expect(NAMED_LABELS.Enter).toBe("Intro");
    expect(NAMED_LABELS.ShiftLeft).toBe("Mayús");
    expect(NAMED_LABELS.AltRight).toBe("Alt Gr");
  });
});

describe("highlightsFor", () => {
  it("highlights nothing for an untypeable or missing char", () => {
    expect(highlightsFor(null)).toEqual([]);
    expect(highlightsFor(undefined)).toEqual([]);
  });

  it("highlights the base key of a normal press", () => {
    expect(highlightsFor([press("KeyA")])).toEqual([{ code: "KeyA", role: "primary" }]);
  });

  it("adds the Shift of the opposite hand for the shift layer", () => {
    expect(highlightsFor([press("KeyA", "shift")])).toEqual([
      { code: "KeyA", role: "primary" },
      { code: "ShiftRight", role: "modifier" },
    ]);
    expect(highlightsFor([press("Semicolon", "shift")])).toEqual([
      { code: "Semicolon", role: "primary" },
      { code: "ShiftLeft", role: "modifier" },
    ]);
  });

  it("adds AltGr for the altgr layer", () => {
    expect(highlightsFor([press("Digit2", "altgr")])).toEqual([
      { code: "Digit2", role: "primary" },
      { code: "AltRight", role: "modifier" },
    ]);
  });

  it("highlights the dead key first and the base key as secondary", () => {
    expect(highlightsFor([press("Quote"), press("KeyA", "shift")])).toEqual([
      { code: "Quote", role: "primary" },
      { code: "KeyA", role: "secondary" },
      { code: "ShiftRight", role: "secondary" },
    ]);
    expect(highlightsFor([press("Quote", "shift"), press("KeyU")])).toEqual([
      { code: "Quote", role: "primary" },
      { code: "ShiftLeft", role: "modifier" },
      { code: "KeyU", role: "secondary" },
    ]);
  });

  it("lists a key once, with its first role", () => {
    expect(highlightsFor([press("Quote", "shift"), press("Quote", "shift")])).toEqual([
      { code: "Quote", role: "primary" },
      { code: "ShiftLeft", role: "modifier" },
    ]);
  });
});

describe("keyForChar", () => {
  it("finds the key that types a char directly, on any layer", () => {
    expect(["a", "A", "ñ", '"', "@", " "].map((c) => keyForChar(LAYOUT, c))).toEqual([
      "KeyA",
      "KeyA",
      "Semicolon",
      "Digit2",
      "Digit2",
      "Space",
    ]);
  });

  it("maps a composed char to its base key", () => {
    expect(["á", "Á", "ü", "Ü"].map((c) => keyForChar(LAYOUT, c))).toEqual(["KeyA", "KeyA", "KeyU", "KeyU"]);
  });

  it("finds a typed dead key and returns null for unknown chars", () => {
    expect(keyForChar(LAYOUT, "´")).toBe("Quote");
    expect(keyForChar(LAYOUT, "z")).toBeNull();
    expect(keyForChar(LAYOUT, "😀")).toBeNull();
  });
});

describe("keyLabels", () => {
  it("shows the normal char with the shift and altgr chars as small labels", () => {
    expect(keyLabels(LAYOUT.keys.Digit2, LAYOUT.deadKeys)).toEqual({
      main: { text: "2", dead: false },
      shift: { text: '"', dead: false },
      altgr: { text: "@", dead: false },
    });
  });

  it("omits a shift label that only capitalises the letter", () => {
    expect(keyLabels(LAYOUT.keys.Semicolon, LAYOUT.deadKeys)).toEqual({ main: { text: "ñ", dead: false }, shift: null, altgr: null });
  });

  it("marks dead keys", () => {
    expect(keyLabels(LAYOUT.keys.Quote, LAYOUT.deadKeys)).toEqual({
      main: { text: "´", dead: true },
      shift: { text: "¨", dead: true },
      altgr: { text: "{", dead: false },
    });
  });

  it("has no labels for whitespace or unmapped keys", () => {
    expect(keyLabels(LAYOUT.keys.Space, LAYOUT.deadKeys)).toEqual({ main: null, shift: null, altgr: null });
    expect(keyLabels(undefined, LAYOUT.deadKeys)).toEqual({ main: null, shift: null, altgr: null });
  });
});
