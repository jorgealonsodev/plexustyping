import { describe, expect, it } from "vitest";
import { HEAT_LEVELS, heatLevel, heatScores } from "./heat";
import type { Layout } from "./types";

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

describe("heatScores", () => {
  it("maps each char to the key that produces it on any layer", () => {
    const heat = heatScores(LAYOUT, [
      { ch: "ñ", score: 0.4 },
      { ch: "@", score: 0.3 },
      { ch: " ", score: 0.1 },
      { ch: "¨", score: 0.2 },
    ]);
    expect([...heat]).toEqual([
      ["Semicolon", 0.4],
      ["Digit2", 0.3],
      ["Space", 0.1],
      ["Quote", 0.2],
    ]);
  });

  it("keeps the max score per key, counting compositions for the base key", () => {
    const heat = heatScores(LAYOUT, [
      { ch: "a", score: 0.2 },
      { ch: "Á", score: 0.7 },
      { ch: "A", score: 0.5 },
      { ch: "ü", score: 0.6 },
      { ch: "u", score: 0.9 },
    ]);
    expect(Object.fromEntries(heat)).toEqual({ KeyA: 0.7, KeyU: 0.9 });
  });

  it("skips chars no key produces and is empty without scores", () => {
    expect(heatScores(LAYOUT, [{ ch: "€", score: 1 }]).size).toBe(0);
    expect(heatScores(LAYOUT, []).size).toBe(0);
  });
});

describe("heatLevel", () => {
  it("buckets a 0..1 score into 0..HEAT_LEVELS, clamping out-of-range input", () => {
    expect(HEAT_LEVELS).toBe(10);
    expect([0, 0.04, 0.05, 0.5, 0.94, 1, 1.5, -1, Number.NaN].map(heatLevel)).toEqual([0, 0, 1, 5, 9, 10, 10, 0, 0]);
  });
});
