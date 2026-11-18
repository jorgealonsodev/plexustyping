import { invoke } from "@tauri-apps/api/core";

/** `plexustyping-store` `SessionSummary`: one stored session (camelCase over IPC). */
export interface SessionSummary {
  id: number;
  /** Start time, Unix epoch milliseconds. */
  startedAtMs: number;
  mode: "prose" | "words" | "code" | string;
  codeLanguage: string | null;
  language: string | null;
  layout: string | null;
  durationMs: number;
  grossWpm: number;
  netWpm: number;
  /** `0..=1`. */
  accuracy: number;
  keystrokes: number;
  bpm: number | null;
}

/** Where the CSV was written and how many sessions it holds. */
export interface CsvExport {
  path: string;
  rows: number;
}

/** Backend mode filter: `null` is every session. */
export type ModeArg = "prose" | "words" | "code" | null;

/** Sessions of the active profile, newest first; `limit: null` returns them all. */
export const sessionHistory = (mode: ModeArg, limit: number | null = null) =>
  invoke<SessionSummary[]>("session_history", { mode, limit });

/** Writes the sessions of `mode` to the Downloads directory, dated in local time. */
export const exportSessionsCsv = (mode: ModeArg) =>
  invoke<CsvExport>("export_sessions_csv", { mode, tzOffsetMin: -new Date().getTimezoneOffset() });

/** Practice of one local day (camelCase over IPC); `minutes` is not rounded. */
export interface DayRow {
  /** Local date, `YYYY-MM-DD`. */
  day: string;
  sessions: number;
  minutes: number;
  /** Duration-weighted means; accuracy in `0..=1`. */
  netWpm: number;
  accuracy: number;
}

/** Statistics of one character with at least 5 samples; `error` and `slowness` in `0..=1`. */
export interface CharRow {
  unit: string;
  samples: number;
  error: number;
  slowness: number;
}

/** The last `days` local days with sessions of `mode` (default 30, at most 366), newest first. */
export const historyByDay = (mode: ModeArg, days: number | null = null) =>
  invoke<DayRow[]>("history_by_day", { mode, tzOffsetMin: -new Date().getTimezoneOffset(), days });

/** Characters of the statistics of `mode` (`null` merges prose and code), worst first. */
export const historyByChar = (mode: ModeArg) => invoke<CharRow[]>("history_by_char", { mode });
