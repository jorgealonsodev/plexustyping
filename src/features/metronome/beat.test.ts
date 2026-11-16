import { describe, expect, it } from "vitest";
import { beatAt, beatOffsetMs, isAccent, isMetronomeShortcut, rhythmSummary, toConfig } from "./beat";

describe("beatOffsetMs", () => {
  it("is the time from the nearest beat, negative when early", () => {
    expect(beatOffsetMs(1000, 1000, 500)).toBe(0);
    expect(beatOffsetMs(1480, 1000, 500)).toBe(-20);
    expect(beatOffsetMs(1530, 1000, 500)).toBe(30);
    expect(beatOffsetMs(2490, 1000, 500)).toBe(-10);
    expect(beatOffsetMs(2240, 1000, 500)).toBe(240);
  });

  it("counts a key halfway between beats as late and one before beat 0 against beat 0", () => {
    expect(beatOffsetMs(1250, 1000, 500)).toBe(250);
    expect(beatOffsetMs(900, 1000, 500)).toBe(-100);
  });
});

describe("beatAt", () => {
  it("gives the beat index and the time since it, or null before beat 0", () => {
    expect(beatAt(1000, 1000, 500)).toEqual({ index: 0, sinceMs: 0 });
    expect(beatAt(2130, 1000, 500)).toEqual({ index: 2, sinceMs: 130 });
    expect(beatAt(999, 1000, 500)).toBeNull();
  });

  it("accents beat 0 and every N-th beat; 0 means no accent", () => {
    expect([0, 1, 3, 4, 8].map((i) => isAccent(i, 4))).toEqual([true, false, false, true, true]);
    expect([0, 1, 2].map((i) => isAccent(i, 0))).toEqual([false, false, false]);
  });
});

describe("rhythmSummary", () => {
  it("is null without measured offsets", () => {
    expect(rhythmSummary([])).toBeNull();
    expect(rhythmSummary([null, null])).toBeNull();
  });

  it("gives the mean absolute offset and the early and late shares of measured keys", () => {
    expect(rhythmSummary([-20, 10, null, 0, 30])).toEqual({ meanAbsMs: 15, earlyPct: 25, latePct: 50 });
  });
});

describe("toConfig", () => {
  it("maps the settings to the backend config, volume as a 0–1 gain", () => {
    expect(toConfig({ enabled: true, bpm: 90, sound: "wood", volume: 40, accentEvery: 3 })).toEqual({
      bpm: 90,
      sound: "wood",
      volume: 0.4,
      accentEvery: 3,
    });
  });
});

describe("isMetronomeShortcut", () => {
  const key = (k: string, mods: Partial<Record<"ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat", boolean>> = {}) => ({
    key: k,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    repeat: false,
    ...mods,
  });

  it("toggles with a bare F4 only", () => {
    expect(isMetronomeShortcut(key("F4"))).toBe(true);
    expect(isMetronomeShortcut(key("F4", { repeat: true }))).toBe(false);
    expect(isMetronomeShortcut(key("F4", { altKey: true }))).toBe(false);
    expect(isMetronomeShortcut(key("F4", { ctrlKey: true }))).toBe(false);
    expect(isMetronomeShortcut(key("F2"))).toBe(false);
  });
});
