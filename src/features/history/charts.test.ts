import { describe, expect, it } from "vitest";
import type { CharRow, DayRow } from "./api";
import {
  CHAR_CHART_LIMIT,
  VIEWS,
  charCaption,
  charChartGeometry,
  dayChartCaption,
  dayTooltip,
  charTableRow,
  dayChartGeometry,
  dayTableRow,
  formatDay,
  formatMinutes,
  formatSlowness,
  unitLabel,
} from "./charts";
import type { ChartBox } from "./history";

const BOX: ChartBox = { width: 600, height: 200, left: 40, right: 40, top: 10, bottom: 30 };

const day = (over: Partial<DayRow> = {}): DayRow => ({
  day: "2026-11-17",
  sessions: 3,
  minutes: 12.9,
  netWpm: 41.5,
  accuracy: 0.905,
  ...over,
});

const char = (over: Partial<CharRow> = {}): CharRow => ({ unit: "a", samples: 12, error: 0.042, slowness: 0.25, ...over });

describe("history views", () => {
  it("lists Sesiones, Por día and Por carácter, sessions first", () => {
    expect(VIEWS).toEqual([
      { id: "sessions", label: "Sesiones" },
      { id: "days", label: "Por día" },
      { id: "chars", label: "Por carácter" },
    ]);
  });
});

describe("day table", () => {
  it("formats the day as dd/mm/aaaa", () => {
    expect(formatDay("2026-11-17")).toBe("17/11/2026");
    expect(formatDay("2027-01-05")).toBe("05/01/2027");
  });

  it("rounds minutes down", () => {
    expect(formatMinutes(12.9)).toBe("12");
    expect(formatMinutes(0.99)).toBe("0");
    expect(formatMinutes(4)).toBe("4");
  });

  it("formats a row in Spanish: integer PPM and one-decimal precision", () => {
    expect(dayTableRow(day())).toEqual({
      day: "2026-11-17",
      date: "17/11/2026",
      sessions: "3",
      minutes: "12",
      netWpm: "42",
      accuracy: "90,5 %",
    });
  });
});

describe("character table", () => {
  it("names space, Enter and Tab and keeps other characters", () => {
    expect(unitLabel(" ")).toBe("espacio");
    expect(unitLabel("\n")).toBe("Intro");
    expect(unitLabel("\t")).toBe("Tab");
    expect(unitLabel("ñ")).toBe("ñ");
    expect(unitLabel("{")).toBe("{");
  });

  it("formats slowness with one decimal comma", () => {
    expect(formatSlowness(0.25)).toBe("0,3");
    expect(formatSlowness(0)).toBe("0,0");
    expect(formatSlowness(1)).toBe("1,0");
    expect(formatSlowness(0.04)).toBe("0,0");
  });

  it("formats a row: error as a percentage with one decimal", () => {
    expect(charTableRow(char())).toEqual({ unit: "a", label: "a", samples: "12", error: "4,2 %", slowness: "0,3" });
    expect(charTableRow(char({ unit: " ", error: 0.5, slowness: 0 }))).toEqual({
      unit: " ",
      label: "espacio",
      samples: "12",
      error: "50 %",
      slowness: "0,0",
    });
  });
});

describe("day chart", () => {
  it("is empty without days", () => {
    const g = dayChartGeometry([], BOX);
    expect(g.bars).toEqual([]);
    expect(g.points).toEqual([]);
    expect(g.line).toBe("");
  });

  it("draws days oldest left, bars for minutes and a line for PPM, on 0..max scales", () => {
    // Backend order: newest first.
    const days = [
      day({ day: "2026-11-18", minutes: 20, netWpm: 50 }),
      day({ day: "2026-11-16", minutes: 5, netWpm: 25 }),
    ];
    const g = dayChartGeometry(days, BOX);
    expect(g.wpmMax).toBe(50);
    expect(g.minutesMax).toBe(20);
    // Plot area x 40..560 (two 260-wide slots), y 10..170.
    expect(g.points).toEqual([
      { day: "2026-11-16", x: 170, y: 90 },
      { day: "2026-11-18", x: 430, y: 10 },
    ]);
    expect(g.line).toBe("170,90 430,10");
    expect(g.bars).toEqual([
      { day: "2026-11-16", x: 92, y: 130, width: 156, height: 40 },
      { day: "2026-11-18", x: 352, y: 10, width: 156, height: 160 },
    ]);
    expect(g.ticks).toEqual([
      { y: 170, wpm: "0", minutes: "0" },
      { y: 90, wpm: "25", minutes: "10" },
      { y: 10, wpm: "50", minutes: "20" },
    ]);
  });

  it("uses at least 0..10 on both scales, rounded up to tens", () => {
    const small = dayChartGeometry([day({ minutes: 0.5, netWpm: 3 })], BOX);
    expect(small.wpmMax).toBe(10);
    expect(small.minutesMax).toBe(10);
    const big = dayChartGeometry([day({ minutes: 61, netWpm: 71 })], BOX);
    expect(big.wpmMax).toBe(80);
    expect(big.minutesMax).toBe(70);
    // A single day sits in the middle.
    expect(big.points[0].x).toBe(300);
  });
});

describe("character chart", () => {
  it("draws the 20 worst characters left to right with their labels", () => {
    const rows = Array.from({ length: 25 }, (_, i) => char({ unit: String.fromCharCode(97 + i), error: (25 - i) / 100 }));
    const g = charChartGeometry(rows, BOX);
    expect(CHAR_CHART_LIMIT).toBe(20);
    expect(g.bars.map((b) => b.unit)).toEqual(rows.slice(0, 20).map((r) => r.unit));
    expect(g.bars[0].label).toBe("a");
    // Worst error 25 % → scale 0..30 %.
    expect(g.errorMax).toBe(0.3);
    expect(g.ticks.map((t) => t.error)).toEqual(["0 %", "15 %", "30 %"]);
    // 20 slots of 26 over x 40..560.
    expect(g.bars[0]).toMatchObject({ x: 43.9, width: 18.2, labelX: 53 });
    expect(g.bars[19].labelX).toBe(547);
  });

  it("scales bar height to the error and labels space as espacio", () => {
    const g = charChartGeometry([char({ unit: " ", error: 0.5 }), char({ unit: "x", error: 0.125 })], BOX);
    expect(g.errorMax).toBe(0.5);
    expect(g.bars[0]).toMatchObject({ unit: " ", label: "espacio", y: 10, height: 160 });
    expect(g.bars[1]).toMatchObject({ unit: "x", y: 130, height: 40 });
  });

  it("uses at least a 0..10 % scale and never more than 100 %", () => {
    expect(charChartGeometry([char({ error: 0 })], BOX).errorMax).toBe(0.1);
    expect(charChartGeometry([char({ error: 1 })], BOX).errorMax).toBe(1);
    expect(charChartGeometry([], BOX).bars).toEqual([]);
  });
});

describe("chart captions and tooltips", () => {
  it("counts days and characters, singular for one", () => {
    expect(dayChartCaption(1)).toBe("Último día con sesiones");
    expect(dayChartCaption(30)).toBe("Últimos 30 días con sesiones");
    expect(charCaption(1)).toBe("El carácter con más errores");
    expect(charCaption(20)).toBe("Los 20 caracteres con más errores");
  });

  it("describes a day in its bar and point titles", () => {
    expect(dayTooltip(dayTableRow(day()))).toBe("17/11/2026: 3 sesiones, 12 min, 42 PPM netas, 90,5 %");
    expect(dayTooltip(dayTableRow(day({ sessions: 1 })))).toBe("17/11/2026: 1 sesión, 12 min, 42 PPM netas, 90,5 %");
  });
});
