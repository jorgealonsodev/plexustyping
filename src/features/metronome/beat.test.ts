import { describe, expect, it } from "vitest";
import { adaptTempo, beatAt, tempoChangeFor, beatOffsetMs, isAccent, isMetronomeShortcut, rhythmSummary, tempoNotice, toConfig } from "./beat";

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
    expect(toConfig({ enabled: true, bpm: 90, sound: "wood", volume: 40, accentEvery: 3, adaptive: true })).toEqual({
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

describe("tempoChangeFor", () => {
  it("adapts a text typed entirely to one tempo", () => {
    expect(tempoChangeFor(60, 0.97, [150, -150])).toEqual({ from: 60, to: 63 });
    expect(tempoChangeFor(60, 0.969, [10, -10])).toEqual({ from: 60, to: 60 });
  });

  it("does not apply without one tempo, without keystrokes or when the metronome was switched mid-text", () => {
    expect(tempoChangeFor(null, 1, [10, -10])).toBeNull();
    expect(tempoChangeFor(60, 1, [])).toBeNull();
    expect(tempoChangeFor(60, 1, [null, null])).toBeNull();
    expect(tempoChangeFor(60, 1, [null, 10, -10])).toBeNull();
    expect(tempoChangeFor(60, 1, [10, -10, null])).toBeNull();
  });
});

describe("adaptTempo", () => {
  // 60 BPM: interval 1000 ms, so 15 % = 150 ms and 20 % = 200 ms.
  const at = (ms: number) => [ms, -ms, ms, -ms];

  it("raises the tempo by 5 % (at least 2 BPM) when accurate and on the beat", () => {
    expect(adaptTempo(60, 0.97, at(150))).toBe(63);
    expect(adaptTempo(200, 1, at(10))).toBe(210);
    expect(adaptTempo(30, 1, at(10))).toBe(32);
  });

  it("lowers the tempo when accuracy is under 90 % or the rhythm is lost", () => {
    expect(adaptTempo(60, 0.899, at(10))).toBe(57);
    expect(adaptTempo(60, 1, at(201))).toBe(57);
    expect(adaptTempo(200, 0.5, at(10))).toBe(190);
  });

  it("keeps the tempo between the two rules", () => {
    expect(adaptTempo(60, 0.969, at(10))).toBe(60);
    expect(adaptTempo(60, 0.9, at(200))).toBe(60);
    expect(adaptTempo(60, 1, at(151))).toBe(60);
  });

  it("stays within 30–300 BPM", () => {
    expect(adaptTempo(299, 1, at(1))).toBe(300);
    expect(adaptTempo(300, 1, at(1))).toBe(300);
    expect(adaptTempo(31, 0, at(1))).toBe(30);
    expect(adaptTempo(30, 0, at(1))).toBe(30);
  });

  it("does not change without measured offsets", () => {
    expect(adaptTempo(60, 1, [])).toBe(60);
    expect(adaptTempo(60, 0, [null, null])).toBe(60);
  });
});

describe("tempoNotice", () => {
  it("tells the change, or that the tempo stays", () => {
    expect(tempoNotice({ from: 60, to: 63 })).toBe("Tempo: 60 → 63 BPM");
    expect(tempoNotice({ from: 60, to: 60 })).toBe("Tempo: se mantiene en 60 BPM");
  });
});
