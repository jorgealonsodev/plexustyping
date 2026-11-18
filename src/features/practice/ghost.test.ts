import { describe, expect, it } from "vitest";
import { GhostRace, formatDelta, ghostPosition, recordLines } from "./ghost";

describe("ghost delta (S4)", () => {
  it("shows behind with +, ahead with − (U+2212), one decimal with a Spanish comma", () => {
    expect([formatDelta(1_200), formatDelta(-800), formatDelta(0)]).toEqual(["+1,2 s", "−0,8 s", "0,0 s"]);
  });

  it("always shows the sign and one decimal, and rounds a near-zero delta to 0,0 s", () => {
    expect([formatDelta(1_000), formatDelta(-12_340), formatDelta(49), formatDelta(-49), formatDelta(150)]).toEqual([
      "+1,0 s",
      "−12,3 s",
      "0,0 s",
      "0,0 s",
      "+0,2 s",
    ]);
    // Halves round away from zero, the same for both signs.
    expect([formatDelta(250), formatDelta(-250)]).toEqual(["+0,3 s", "−0,3 s"]);
  });
});

describe("ghost position (S4)", () => {
  const times = [0, 100, 250, 400];

  it("waits at the start before the first keystroke", () => {
    expect(ghostPosition(times, null)).toBe(0);
  });

  it("is past every position the record had passed at the elapsed time", () => {
    expect([0, 99, 100, 249, 250, 399].map((t) => ghostPosition(times, t))).toEqual([1, 1, 2, 2, 3, 3]);
  });

  it("stays at the end once it reaches it", () => {
    expect([400, 401, 10_000].map((t) => ghostPosition(times, t))).toEqual([4, 4, 4]);
  });

  it("stops before a position the record never reached", () => {
    expect(ghostPosition([0, 100, null, null], 5_000)).toBe(2);
    expect(ghostPosition([null, null], 5_000)).toBe(0);
  });
});

describe("ghost race (S4)", () => {
  it("has no delta before the user passes a position", () => {
    const race = new GhostRace([0, 100, 250]);
    expect(race.delta).toBeNull();
    race.passed(0, 0);
    expect(race.delta).toBeNull();
  });

  it("compares the user's time at the last passed position with the record's", () => {
    const race = new GhostRace([0, 100, 250]);
    race.passed(1, 0);
    expect(race.delta).toBe(0);
    race.passed(2, 300);
    expect(race.delta).toBe(200);
    race.passed(3, 50);
    expect(race.delta).toBe(-200);
  });

  it("keeps the first pass: moving back and forth again changes nothing", () => {
    const race = new GhostRace([0, 100, 250]);
    race.passed(2, 120);
    expect(race.delta).toBe(20);
    race.passed(1, 500);
    race.passed(2, 600);
    expect(race.delta).toBe(20);
  });

  it("has no delta at a position the record never reached", () => {
    const race = new GhostRace([0, null]);
    race.passed(2, 300);
    expect(race.delta).toBeNull();
  });
});

describe("record lines (S5)", () => {
  it("shows the previous record", () => {
    expect(recordLines(52, 40, true)).toEqual(["Récord anterior: 52 PPM"]);
    expect(recordLines(51.6, 51.6, true)).toEqual(["Récord anterior: 52 PPM"]);
  });

  it("celebrates a beaten record", () => {
    expect(recordLines(52, 60.2, true)).toEqual(["Récord anterior: 52 PPM", "¡Nuevo récord para este texto!"]);
  });

  it("is no record when the session was not saved", () => {
    expect(recordLines(52, 60, false)).toEqual(["Récord anterior: 52 PPM"]);
  });
});
