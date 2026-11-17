import { invoke } from "@tauri-apps/api/core";

/** `plexustyping-store` `SessionSummary`: one stored session (camelCase over IPC). */
export interface SessionSummary {
  id: number;
  /** Start time, Unix epoch milliseconds. */
  startedAtMs: number;
  mode: "prose" | "code" | string;
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
export type ModeArg = "prose" | "code" | null;

/** Sessions of the active profile, newest first; `limit: null` returns them all. */
export const sessionHistory = (mode: ModeArg, limit: number | null = null) =>
  invoke<SessionSummary[]>("session_history", { mode, limit });

/** Writes the sessions of `mode` to the Downloads directory, dated in local time. */
export const exportSessionsCsv = (mode: ModeArg) =>
  invoke<CsvExport>("export_sessions_csv", { mode, tzOffsetMin: -new Date().getTimezoneOffset() });
