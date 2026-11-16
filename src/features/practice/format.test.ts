import { describe, expect, it } from "vitest";
import { charLabel, formatPercent, formatTime, formatWpm } from "./format";

describe("result formatting", () => {
  it("formats durations as m:ss", () => {
    expect([formatTime(0), formatTime(9_999), formatTime(65_400), formatTime(600_000)]).toEqual([
      "0:00",
      "0:09",
      "1:05",
      "10:00",
    ]);
  });

  it("rounds words per minute to whole numbers", () => {
    expect([formatWpm(0), formatWpm(41.5), formatWpm(41.49)]).toEqual(["0", "42", "41"]);
  });

  it("formats accuracy as a Spanish percentage with at most one decimal", () => {
    expect([formatPercent(1), formatPercent(0.9), formatPercent(0.905), formatPercent(0)]).toEqual([
      "100 %",
      "90 %",
      "90,5 %",
      "0 %",
    ]);
  });

  it("shows a space as a visible symbol", () => {
    expect([charLabel(" "), charLabel("ñ")]).toEqual(["␣", "ñ"]);
  });
});
