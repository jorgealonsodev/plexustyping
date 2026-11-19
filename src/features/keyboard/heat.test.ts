import { describe, expect, it } from "vitest";
import { HEAT_LEGEND, HEAT_LEGEND_TITLE, HEAT_LEVELS, HEAT_RANKS, heatLegendText, heatLevel, heatRank, heatScores } from "./heat";
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

describe("heatRank (a11y S5)", () => {
  it("maps the 10 tint steps to the numeric levels 1–5, 0 without tint", () => {
    expect(HEAT_RANKS).toBe(5);
    expect([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map(heatRank)).toEqual([0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
  });

  it("clamps out-of-range steps", () => {
    expect([-1, 11, Number.NaN].map(heatRank)).toEqual([0, 5, 0]);
  });
});

describe("heat legend (a11y S5)", () => {
  it("names the five levels in Spanish, each with the tint step it is drawn with", () => {
    expect(HEAT_LEGEND_TITLE).toBe("Errores por tecla");
    expect(HEAT_LEGEND).toEqual([
      { rank: 1, name: "muy pocos", tint: 2 },
      { rank: 2, name: "pocos", tint: 4 },
      { rank: 3, name: "algunos", tint: 6 },
      { rank: 4, name: "bastantes", tint: 8 },
      { rank: 5, name: "muchos", tint: 10 },
    ]);
    for (const { rank, tint } of HEAT_LEGEND) expect(heatRank(tint)).toBe(rank);
  });

  it("reads the whole legend as one line", () => {
    expect(heatLegendText()).toBe("Errores por tecla: 1 muy pocos, 2 pocos, 3 algunos, 4 bastantes, 5 muchos");
  });
});
