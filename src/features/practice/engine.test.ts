import { describe, expect, it } from "vitest";
import cases from "../../../crates/plexustyping-core/tests/fixtures/replay.json";
import { type ErrorMode, type InputEvent, type KeyLike, TypingSession, keyToEvent, replay } from "./engine";

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

describe("keyToEvent", () => {
  it("turns printable keys, including composed accents, into one char event", () => {
    expect(keyToEvent(key("a"), 3)).toEqual(ch("a", 3));
    expect(keyToEvent(key(" "), 4)).toEqual(ch(" ", 4));
    expect(keyToEvent(key("á"), 5)).toEqual(ch("á", 5));
    expect(keyToEvent(key("Ñ"), 6)).toEqual(ch("Ñ", 6));
  });

  it("turns Backspace into a backspace event", () => {
    expect(keyToEvent(key("Backspace"), 7)).toEqual(bs(7));
  });

  it("ignores dead keys, modifiers, named keys and IME composition", () => {
    for (const k of ["Dead", "Shift", "Control", "Alt", "AltGraph", "Meta", "CapsLock", "Enter", "Tab", "Escape", "ArrowLeft"])
      expect(keyToEvent(key(k), 0), k).toBeNull();
    expect(keyToEvent(key("a", { isComposing: true }), 0)).toBeNull();
  });

  it("ignores Ctrl and Meta shortcuts but accepts AltGr characters", () => {
    expect(keyToEvent(key("c", { ctrlKey: true }), 0)).toBeNull();
    expect(keyToEvent(key("Backspace", { ctrlKey: true }), 0)).toBeNull();
    expect(keyToEvent(key("v", { metaKey: true }), 0)).toBeNull();
    expect(keyToEvent(key("@", { ctrlKey: true, altKey: true, altGraph: true }), 1)).toEqual(ch("@", 1));
    expect(keyToEvent(key("€", { altGraph: true }), 2)).toEqual(ch("€", 2));
  });
});
