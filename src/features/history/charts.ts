import { formatPercent, formatWpm } from "../practice/format";
import type { CharRow, DayRow } from "./api";
import type { ChartBox } from "./history";

/** The views of the history window (S1); "Sesiones" is the default. */
export type HistoryView = "sessions" | "days" | "chars";

export const VIEWS: { id: HistoryView; label: string }[] = [
  { id: "sessions", label: "Sesiones" },
  { id: "days", label: "Por día" },
  { id: "chars", label: "Por carácter" },
];

/** `YYYY-MM-DD` as `dd/mm/aaaa`. */
export function formatDay(day: string): string {
  const [y, m, d] = day.split("-");
  return `${d}/${m}/${y}`;
}

/** Whole minutes, rounded down. */
export const formatMinutes = (minutes: number) => String(Math.floor(minutes));

export interface DayTableRow {
  day: string;
  date: string;
  sessions: string;
  minutes: string;
  netWpm: string;
  accuracy: string;
}

/** One row of the "Por día" table (S2). */
export const dayTableRow = (r: DayRow): DayTableRow => ({
  day: r.day,
  date: formatDay(r.day),
  sessions: String(r.sessions),
  minutes: formatMinutes(r.minutes),
  netWpm: formatWpm(r.netWpm),
  accuracy: formatPercent(r.accuracy),
});

/** Title of one day's bar and point in the day chart. */
export const dayTooltip = (r: DayTableRow) =>
  `${r.date}: ${r.sessions} ${r.sessions === "1" ? "sesión" : "sesiones"}, ${r.minutes} min, ${r.netWpm} PPM netas, ${r.accuracy}`;

/** Caption of the day chart: how many days with sessions it shows. */
export const dayChartCaption = (n: number) => (n === 1 ? "Último día con sesiones" : `Últimos ${n} días con sesiones`);

/** Caption of the character chart: how many characters it shows. */
export const charCaption = (n: number) =>
  n === 1 ? "El carácter con más errores" : `Los ${n} caracteres con más errores`;

const UNIT_NAMES: Record<string, string> = { " ": "espacio", "\n": "Intro", "\t": "Tab" };

/** A character as shown in the "Por carácter" view: space, Enter and Tab by name. */
export const unitLabel = (unit: string) => UNIT_NAMES[unit] ?? unit;

/** The stored relative slowness with one decimal comma, e.g. `0,3`. */
export const formatSlowness = (slowness: number) => (Math.round(slowness * 10) / 10).toFixed(1).replace(".", ",");

export interface CharTableRow {
  unit: string;
  label: string;
  samples: string;
  error: string;
  slowness: string;
}

/** One row of the "Por carácter" table (S3). */
export const charTableRow = (r: CharRow): CharTableRow => ({
  unit: r.unit,
  label: unitLabel(r.unit),
  samples: String(r.samples),
  error: formatPercent(r.error),
  slowness: formatSlowness(r.slowness),
});

const round = (n: number) => Math.round(n * 100) / 100;
/** `max` rounded up to tens, at least 10. */
const scaleTop = (max: number) => Math.max(10, Math.ceil(max / 10) * 10);

/** Plot area of `box`: `x0..x1` left to right, `yTop..yBottom` top to bottom. */
function plot(box: ChartBox) {
  const x0 = box.left;
  const x1 = box.width - box.right;
  const yTop = box.top;
  const yBottom = box.height - box.bottom;
  const yAt = (ratio: number) => round(yBottom - (yBottom - yTop) * Math.min(1, Math.max(0, ratio)));
  return { x0, x1, yTop, yBottom, yAt };
}

export interface DayBar {
  day: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface DayPoint {
  day: string;
  x: number;
  y: number;
}

export interface DayTick {
  y: number;
  wpm: string;
  minutes: string;
}

export interface DayChartGeometry {
  /** Practised minutes, one bar per day. */
  bars: DayBar[];
  /** Mean net WPM, one point per day at the bar's centre. */
  points: DayPoint[];
  /** `points` attribute of the polyline. */
  line: string;
  /** Tops of the scales (bottoms are 0). */
  wpmMax: number;
  minutesMax: number;
  ticks: DayTick[];
}

/**
 * The "Por día" chart (S2): one slot per day, oldest left; bars for minutes, a line for net WPM.
 * `days` comes newest first, as the backend returns it.
 */
export function dayChartGeometry(days: DayRow[], box: ChartBox): DayChartGeometry {
  const shown = [...days].reverse();
  const { x0, x1, yBottom, yAt } = plot(box);
  const slot = shown.length === 0 ? 0 : (x1 - x0) / shown.length;
  const wpmMax = scaleTop(Math.max(0, ...shown.map((d) => d.netWpm)));
  const minutesMax = scaleTop(Math.max(0, ...shown.map((d) => d.minutes)));
  const centre = (i: number) => x0 + slot * (i + 0.5);
  const width = round(slot * 0.6);
  const bars = shown.map((d, i) => {
    const y = yAt(d.minutes / minutesMax);
    return { day: d.day, x: round(centre(i) - width / 2), y, width, height: round(yBottom - y) };
  });
  const points = shown.map((d, i) => ({ day: d.day, x: round(centre(i)), y: yAt(d.netWpm / wpmMax) }));
  const ticks = [0, 0.5, 1].map((r) => ({
    y: yAt(r),
    wpm: formatWpm(wpmMax * r),
    minutes: formatMinutes(minutesMax * r),
  }));
  return { bars, points, line: points.map((p) => `${p.x},${p.y}`).join(" "), wpmMax, minutesMax, ticks };
}

/** Characters drawn in the "Por carácter" chart (S3). */
export const CHAR_CHART_LIMIT = 20;

export interface CharBar {
  unit: string;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** Centre of the bar, where its label goes. */
  labelX: number;
}

export interface CharTick {
  y: number;
  error: string;
}

export interface CharChartGeometry {
  bars: CharBar[];
  /** Top of the error scale: the worst error rounded up to tenths, 10 %..100 %. */
  errorMax: number;
  ticks: CharTick[];
}

/** The "Por carácter" chart (S3): the error of the `CHAR_CHART_LIMIT` worst, worst left. */
export function charChartGeometry(rows: CharRow[], box: ChartBox): CharChartGeometry {
  const shown = rows.slice(0, CHAR_CHART_LIMIT);
  const { x0, x1, yBottom, yAt } = plot(box);
  const slot = (x1 - x0) / CHAR_CHART_LIMIT;
  const worst = Math.max(0, ...shown.map((r) => r.error));
  const errorMax = Math.min(1, Math.max(0.1, Math.ceil(round(worst * 10)) / 10));
  const width = round(slot * 0.7);
  const bars = shown.map((r, i) => {
    const y = yAt(r.error / errorMax);
    const labelX = round(x0 + slot * (i + 0.5));
    return { unit: r.unit, label: unitLabel(r.unit), x: round(labelX - width / 2), y, width, height: round(yBottom - y), labelX };
  });
  const ticks = [0, 0.5, 1].map((r) => ({ y: yAt(r), error: formatPercent(errorMax * r) }));
  return { bars, errorMax, ticks };
}
