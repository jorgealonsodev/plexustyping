import { formatPercent, formatTime, formatWpm } from "../practice/format";
import type { CsvExport, ModeArg, SessionSummary } from "./api";

/** The history filter: every session, or prose, words or code only (S2). */
export type HistoryFilter = "all" | "prose" | "words" | "code";

export const FILTERS: { id: HistoryFilter; label: string }[] = [
  { id: "all", label: "Todo" },
  { id: "prose", label: "Prosa" },
  { id: "words", label: "Palabras" },
  { id: "code", label: "Código" },
];

export const modeArg = (filter: HistoryFilter): ModeArg => (filter === "all" ? null : filter);

type ShortcutKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat">;

/** Ctrl+H opens (or closes) the history window; only plain Ctrl counts, and never a held key. */
export const isHistoryShortcut = (e: ShortcutKey) =>
  e.key.toLowerCase() === "h" && e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey && !e.repeat;

export interface Summary {
  count: number;
  meanNetWpm: number;
  bestNetWpm: number;
  meanAccuracy: number;
}

/** Count, mean and best net WPM and mean accuracy (S3); `null` without sessions. */
export function summarize(sessions: SessionSummary[]): Summary | null {
  if (sessions.length === 0) return null;
  let wpm = 0;
  let acc = 0;
  let best = -Infinity;
  for (const s of sessions) {
    wpm += s.netWpm;
    acc += s.accuracy;
    best = Math.max(best, s.netWpm);
  }
  const n = sessions.length;
  return { count: n, meanNetWpm: wpm / n, bestNetWpm: best, meanAccuracy: acc / n };
}

export const formatSummary = (s: Summary) => [
  { label: "Sesiones", value: String(s.count) },
  { label: "PPM netas (media)", value: formatWpm(s.meanNetWpm) },
  { label: "PPM netas (mejor)", value: formatWpm(s.bestNetWpm) },
  { label: "Precisión media", value: formatPercent(s.meanAccuracy) },
];

const pad2 = (n: number) => String(n).padStart(2, "0");

/** Local date and time as `dd/mm/aaaa hh:mm`. */
export function formatDate(ms: number): string {
  const d = new Date(ms);
  return `${pad2(d.getDate())}/${pad2(d.getMonth() + 1)}/${d.getFullYear()} ${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

export interface TableRow {
  id: number;
  date: string;
  mode: string;
  netWpm: string;
  accuracy: string;
  time: string;
  bpm: string;
}

/** One table row (S5); `languageName` turns a code language id into its display name. */
export function tableRow(s: SessionSummary, languageName: (id: string) => string): TableRow {
  const code = s.codeLanguage ? `Código · ${languageName(s.codeLanguage)}` : "Código";
  const mode = s.mode === "code" ? code : s.mode === "words" ? "Palabras" : "Prosa";
  return {
    id: s.id,
    date: formatDate(s.startedAtMs),
    mode,
    netWpm: formatWpm(s.netWpm),
    accuracy: formatPercent(s.accuracy),
    time: formatTime(s.durationMs),
    bpm: s.bpm === null ? "" : String(s.bpm),
  };
}

/** Sessions drawn in the evolution chart (S4). */
export const CHART_LIMIT = 50;

/** SVG user-space size of the chart and the margins around its plot area. */
export interface ChartBox {
  width: number;
  height: number;
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export interface ChartPoint {
  id: number;
  x: number;
  y: number;
}

export interface ChartTick {
  y: number;
  wpm: string;
  accuracy: string;
}

export interface ChartGeometry {
  wpm: ChartPoint[];
  accuracy: ChartPoint[];
  /** `points` attribute of each polyline. */
  wpmLine: string;
  accuracyLine: string;
  /** Top of the net WPM scale (the bottom is 0); accuracy always spans 0..100 %. */
  wpmMax: number;
  ticks: ChartTick[];
}

const round = (n: number) => Math.round(n * 100) / 100;
const line = (points: ChartPoint[]) => points.map((p) => `${p.x},${p.y}`).join(" ");

/**
 * Points of the last `CHART_LIMIT` sessions, oldest left and newest right. `sessions` comes newest
 * first, as the backend returns it; a single session sits in the middle.
 */
export function chartGeometry(sessions: SessionSummary[], box: ChartBox): ChartGeometry {
  const shown = sessions.slice(0, CHART_LIMIT).reverse();
  const x0 = box.left;
  const x1 = box.width - box.right;
  const yTop = box.top;
  const yBottom = box.height - box.bottom;
  const wpmMax = Math.max(10, Math.ceil(Math.max(0, ...shown.map((s) => s.netWpm)) / 10) * 10);
  const xAt = (i: number) => round(shown.length === 1 ? (x0 + x1) / 2 : x0 + ((x1 - x0) * i) / (shown.length - 1));
  const yAt = (ratio: number) => round(yBottom - (yBottom - yTop) * Math.min(1, Math.max(0, ratio)));
  const wpm = shown.map((s, i) => ({ id: s.id, x: xAt(i), y: yAt(s.netWpm / wpmMax) }));
  const accuracy = shown.map((s, i) => ({ id: s.id, x: xAt(i), y: yAt(s.accuracy) }));
  const ticks = [0, 0.5, 1].map((r) => ({ y: yAt(r), wpm: formatWpm(wpmMax * r), accuracy: formatPercent(r) }));
  return { wpm, accuracy, wpmLine: line(wpm), accuracyLine: line(accuracy), wpmMax, ticks };
}

/** Success notice of the CSV export (S7). */
export const exportNotice = (r: CsvExport) =>
  `Exportado: ${r.path} (${r.rows} ${r.rows === 1 ? "sesión" : "sesiones"})`;

/** Caption of the evolution chart (S4): singular for one session, as the export notice (L4). */
export const chartCaption = (n: number) => (n === 1 ? "Última sesión" : `Últimas ${n} sesiones`);

/**
 * Lets only the newest request fill the window: `next()` starts one, `invalidate()` drops every
 * request in flight (filter change, close and reopen), `isCurrent(id)` says whether a reply applies.
 */
export function requestGuard() {
  let latest = 0;
  return {
    next: () => ++latest,
    invalidate: () => {
      latest++;
    },
    isCurrent: (id: number) => id === latest,
  };
}
