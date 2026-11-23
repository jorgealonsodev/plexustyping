import { describe, expect, it } from "vitest";
import type { GoalStatus } from "./api";
import {
  GOAL_KINDS,
  goalMetNotice,
  MINUTE_GOALS,
  TEXT_GOALS,
  amountsFor,
  chooseGoal,
  crossedGoal,
  goalHeaderText,
  localDay,
  tzOffsetMin,
} from "./goals";

const minutes = (amount: number) => ({ kind: "minutes" as const, amount });
const texts = (amount: number) => ({ kind: "texts" as const, amount });
const status = (goal: GoalStatus["goal"], today: number, met: boolean, streak: number): GoalStatus => ({
  goal,
  today,
  met,
  streak,
});

describe("daily goal choices (S1, S5)", () => {
  it("offers off, minutes and texts with the S1 amounts", () => {
    expect(GOAL_KINDS).toEqual([
      { id: "off", label: "Desactivada" },
      { id: "minutes", label: "Minutos al día" },
      { id: "texts", label: "Textos al día" },
    ]);
    expect(MINUTE_GOALS).toEqual([5, 10, 15, 20, 30, 45, 60, 90, 120]);
    expect(TEXT_GOALS).toEqual([1, 3, 5, 10, 15, 20, 30]);
    expect(amountsFor("minutes")).toEqual(MINUTE_GOALS);
    expect(amountsFor("texts")).toEqual(TEXT_GOALS);
    expect(amountsFor("off")).toEqual([]);
  });

  it("picks an allowed amount when the kind changes and keeps one that fits", () => {
    expect(chooseGoal("off", null)).toEqual({ kind: "off", amount: null });
    expect(chooseGoal("off", minutes(30))).toEqual({ kind: "off", amount: null });
    expect(chooseGoal("minutes", null)).toEqual({ kind: "minutes", amount: 20 });
    expect(chooseGoal("texts", null)).toEqual({ kind: "texts", amount: 5 });
    expect(chooseGoal("texts", minutes(30))).toEqual({ kind: "texts", amount: 5 });
    expect(chooseGoal("minutes", minutes(45))).toEqual({ kind: "minutes", amount: 45 });
    expect(chooseGoal("minutes", minutes(45), 90)).toEqual({ kind: "minutes", amount: 90 });
    // An amount outside the kind's list falls back to its default.
    expect(chooseGoal("texts", texts(3), 45)).toEqual({ kind: "texts", amount: 5 });
  });
});

describe("header progress text (S5)", () => {
  it("shows nothing without a goal or status", () => {
    expect(goalHeaderText(null)).toBeNull();
    expect(goalHeaderText(status(null, 0, false, 0))).toBeNull();
  });

  it("shows minutes and texts with the streak", () => {
    expect(goalHeaderText(status(minutes(20), 12, false, 3))).toBe("Hoy 12/20 min · racha 3");
    expect(goalHeaderText(status(texts(5), 2, false, 3))).toBe("Hoy 2/5 textos · racha 3");
  });

  it("omits a streak of 0", () => {
    expect(goalHeaderText(status(minutes(20), 0, false, 0))).toBe("Hoy 0/20 min");
    expect(goalHeaderText(status(texts(1), 0, false, 0))).toBe("Hoy 0/1 textos");
  });

  it("shows the met goal with the streak", () => {
    expect(goalHeaderText(status(minutes(20), 25, true, 4))).toBe("Meta cumplida · racha 4");
    expect(goalHeaderText(status(texts(3), 3, true, 1))).toBe("Meta cumplida · racha 1");
  });
});

describe("goal met notice (S5)", () => {
  const day = "2026-11-18";
  const at = (s: GoalStatus, d = day) => ({ status: s, day: d });

  it("is the exact Spanish text", () => {
    expect(goalMetNotice()).toBe("¡Meta diaria cumplida!");
  });

  it("shows only on the session that meets the goal", () => {
    const sessions = [
      at(status(texts(3), 0, false, 0)),
      at(status(texts(3), 1, false, 0)),
      at(status(texts(3), 2, false, 0)),
      at(status(texts(3), 3, true, 1)),
      at(status(texts(3), 4, true, 1)),
      at(status(texts(3), 5, true, 1)),
    ];
    const shown = sessions.slice(1).map((after, i) => crossedGoal(sessions[i], after));
    expect(shown).toEqual([false, false, true, false, false]);
  });

  it("is not shown without a goal, without a known previous status or after a goal change", () => {
    expect(crossedGoal(at(status(null, 0, false, 0)), at(status(null, 0, false, 0)))).toBe(false);
    expect(crossedGoal(null, at(status(texts(1), 1, true, 1)))).toBe(false);
    expect(crossedGoal(at(status(texts(3), 2, false, 0)), at(status(texts(1), 3, true, 1)))).toBe(false);
  });

  it("is shown again on the first session that meets the goal on a new day", () => {
    const yesterday = at(status(texts(1), 1, true, 1), "2026-11-17");
    expect(crossedGoal(yesterday, at(status(texts(1), 1, true, 2)))).toBe(true);
    expect(crossedGoal(yesterday, at(status(texts(3), 1, false, 1)))).toBe(false);
  });
});

describe("local day and offset (S2)", () => {
  it("sends minutes east of UTC and names the local day", () => {
    const d = new Date(2026, 10, 18, 0, 0, 0, 1);
    expect(tzOffsetMin(d)).toBe(-d.getTimezoneOffset());
    expect(localDay(d)).toBe("2026-11-18");
    expect(localDay(new Date(2026, 0, 5, 23, 59))).toBe("2026-01-05");
  });
});
