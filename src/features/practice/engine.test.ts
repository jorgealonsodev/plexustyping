import { describe, expect, it } from "vitest";
import cases from "../../../crates/plexustyping-core/tests/fixtures/replay.json";
import {
  type ErrorMode,
  type InputEvent,
  type KeyLike,
  TypingSession,
  acceptsInput,
  keyAction,
  replay,
  textToEvents,
} from "./engine";

const close = (actual: number, expected: number, label: string) =>
  expect(Math.abs(actual - expected) < 1e-9, `${label}: ${actual} vs ${expected}`).toBe(true);

describe("replay mirrors plexustyping-core fixtures", () => {
  it("has the shared fixture set", () => expect(cases.length).toBeGreaterThanOrEqual(15));

  for (const c of cases) {
    it(c.name, () => {
      const result = replay(c.text, c.mode as ErrorMode, c.events as InputEvent[]);
      const exp = c.expected;
      expect(result.cursor).toBe(exp.cursor);
      expect(result.completed).toBe(exp.completed);
      expect(result.records.length).toBe(exp.records.length);
      result.records.forEach((r, i) => {
        const w = exp.records[i];
        expect([r.index, r.expected, r.typed, r.correct]).toEqual([w.index, w.expected, w.typed, w.correct]);
        if (w.msSincePrevious === null) expect(r.msSincePrevious).toBeNull();
        else close(r.msSincePrevious!, w.msSincePrevious, `records[${i}].ms`);
      });
      const { stats: s } = result;
      const e = exp.stats;
      for (const k of ["keystrokes", "charKeystrokes", "correctKeystrokes", "errors", "uncorrectedErrors", "charsTyped"] as const)
        expect(s[k], k).toBe(e[k]);
      for (const k of ["durationMs", "accuracy", "grossWpm", "netWpm", "kpm"] as const) close(s[k], e[k], k);
    });
  }
});

const ch = (c: string, tMs = 0): InputEvent => ({ kind: "char", ch: c, tMs });
const bs = (tMs = 0): InputEvent => ({ kind: "backspace", tMs });
const states = (s: TypingSession) => s.chars.map((_, i) => s.state(i));

describe("TypingSession display state", () => {
  it("marks typed positions correct or wrong and the rest pending (allow-continue)", () => {
    const s = new TypingSession("hola", "allow-continue");
    expect([s.apply(ch("h", 5)), s.apply(ch("x", 9))]).toEqual([true, true]);
    expect(states(s)).toEqual(["correct", "error", "pending", "pending"]);
    expect(s.cursor).toBe(2);
    expect(s.firstT).toBe(5);
    expect(s.apply(bs())).toBe(false);
    expect(s.events).toEqual([ch("h", 5), ch("x", 9), bs()]);
  });

  it("keeps the cursor on a wrong key and flags it until corrected (stop-on-error)", () => {
    const s = new TypingSession("ab", "stop-on-error");
    s.apply(ch("x"));
    expect([s.cursor, s.state(0)]).toEqual([0, "error"]);
    s.apply(ch("a"));
    expect(states(s)).toEqual(["correct", "pending"]);
  });

  it("reopens the previous position on Backspace (backspace-correct)", () => {
    const s = new TypingSession("ab", "backspace-correct");
    s.apply(ch("x"));
    expect(s.apply(bs())).toBe(true);
    expect([s.cursor, s.state(0)]).toEqual([0, "pending"]);
    s.apply(ch("a"));
    s.apply(ch("b"));
    expect(s.completed).toBe(true);
    expect(s.apply(ch("c"))).toBe(false);
  });

  it("result() equals replay over the same events", () => {
    const events = [ch("h", 0), ch("x", 100), bs(150), ch("o", 200)];
    const s = new TypingSession("hola", "backspace-correct");
    events.forEach((e) => s.apply(e));
    expect(s.result()).toEqual(replay("hola", "backspace-correct", events));
  });
});

const key = (k: string, mods: Partial<KeyLike> & { altGraph?: boolean } = {}): KeyLike => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  isComposing: false,
  ...mods,
  getModifierState: (m: string) => m === "AltGraph" && !!mods.altGraph,
});

describe("textToEvents", () => {
  it("emits one char event per Unicode scalar, composed accents included", () => {
    expect(textToEvents("á", 5)).toEqual([ch("á", 5)]);
    expect(textToEvents("Ñ ü", 6)).toEqual([ch("Ñ", 6), ch(" ", 6), ch("ü", 6)]);
    expect(textToEvents("😀", 7)).toEqual([ch("😀", 7)]);
    expect(textToEvents("", 8)).toEqual([]);
  });

  it("keeps a dead key that composed nothing as its own char", () => {
    expect(textToEvents("´q", 9)).toEqual([ch("´", 9), ch("q", 9)]);
  });

  it("composes a decomposed accent into one char", () => {
    expect(textToEvents("a\u0301", 10)).toEqual([ch("á", 10)]);
  });
});

describe("acceptsInput", () => {
  it("accepts typed and composed text", () => {
    for (const t of ["insertText", "insertCompositionText", "insertReplacementText", ""]) expect(acceptsInput(t), t).toBe(true);
  });

  it("rejects paste, drop, undo, deletions and line breaks", () => {
    for (const t of ["insertFromPaste", "insertFromDrop", "insertFromYank", "historyUndo", "historyRedo", "deleteContentBackward", "insertLineBreak", "insertParagraph"])
      expect(acceptsInput(t), t).toBe(false);
  });
});

describe("keyAction", () => {
  it("maps Backspace, Escape, Enter and Tab", () => {
    expect(["Backspace", "Escape", "Enter", "Tab"].map((k) => keyAction(key(k)))).toEqual(["backspace", "escape", "enter", "tab"]);
  });

  it("leaves printable, dead, modifier and function keys to the text input", () => {
    for (const k of ["a", "á", " ", "Dead", "Shift", "Control", "AltGraph", "CapsLock", "ArrowLeft", "F2"]) expect(keyAction(key(k)), k).toBeNull();
  });

  it("ignores every key while composing", () => {
    for (const k of ["Backspace", "Escape", "Enter", "Tab"]) expect(keyAction(key(k, { isComposing: true })), k).toBeNull();
  });

  it("ignores Ctrl and Meta Backspace but accepts it with AltGr", () => {
    expect(keyAction(key("Backspace", { ctrlKey: true }))).toBeNull();
    expect(keyAction(key("Backspace", { metaKey: true }))).toBeNull();
    expect(keyAction(key("Backspace", { ctrlKey: true, altKey: true, altGraph: true }))).toBe("backspace");
  });
});
