import { invoke } from "@tauri-apps/api/core";
import type { PracticeMode } from "../code/api";
import type { Token } from "../code/syntax";
import type { CharScore } from "../keyboard/heat";
import type { ErrorMode, IndentMode, InputEvent, SessionResult } from "./engine";
import { localDay } from "./prefetch";

/** `plexustyping-app` `ScoredSession`: authoritative replay, chars to practise and storage outcome. */
export interface ScoredSession {
  result: SessionResult;
  failedChars: string[];
  saved: boolean;
  saveError: string | null;
  /** Metronome offset of each record, ms (negative = early); empty without a metronome. */
  offsetsMs: (number | null)[];
}

/** Where a practice text came from: generated now, the local cache of AI texts, or the corpus. */
export type TextSource = "ai" | "cache" | "corpus";

/** A practice text, the weak units (chars or bigrams) it really practises and its origin. */
export interface PracticeText {
  text: string;
  targets: string[];
  source: TextSource;
  /** Provider id and model of `ai` and `cache` texts. */
  provider?: string;
  model?: string;
  /** Why the AI was not used although it is enabled. */
  notice?: string;
  /** The fixed diagnostic text, shown before any session is saved. */
  diagnostic?: boolean;
  /** Code texts: highlighter syntax id and contiguous tokens over the chars. */
  syntax?: string;
  tokens?: Token[];
}

export interface Weakness {
  unit: string;
  score: number;
}

export interface StorageStatus {
  persistent: boolean;
  error: string | null;
}

export const LANGUAGE = "es";

/** `layout` decides the chars a text may contain; `mode` is prose or code; `maxLines` sizes code snippets. */
export const firstText = (layout: string, mode: PracticeMode = "prose", maxLines = 12) =>
  invoke<PracticeText>("first_text", { language: LANGUAGE, layout, day: localDay(), mode, maxLines });

export const nextText = (
  layout: string,
  seed: number,
  targetCount: number,
  length: number,
  fallback: string[],
  mode: PracticeMode = "prose",
  maxLines = 12,
) =>
  invoke<PracticeText>("next_text", {
    language: LANGUAGE,
    layout,
    seed,
    targetCount,
    length,
    fallback,
    day: localDay(),
    mode,
    maxLines,
  });

/** How a text was typed: prose or code of a language, and the code indentation. */
export interface Practice {
  mode: PracticeMode;
  indent: IndentMode;
}

/**
 * `offsetsMs` has one entry per event; `bpm` is the one metronome tempo of the text, if any.
 * Code sessions are stored with their language and folded into the code statistics (S5);
 * every session records the `layout` it was typed with.
 */
export const scoreSession = (
  layout: string,
  text: string,
  mode: ErrorMode,
  events: InputEvent[],
  startedAtMs: number,
  offsetsMs: (number | null)[] | null = null,
  bpm: number | null = null,
  practice: Practice = { mode: "prose", indent: "manual" },
) =>
  invoke<ScoredSession>("score_session", {
    text,
    mode,
    events,
    startedAtMs,
    offsetsMs,
    bpm,
    practice: practice.mode,
    indent: practice.indent,
    layout,
  });

/** Of the statistics model of `mode` (prose and code are kept apart). */
export const weaknesses = (limit: number, mode: PracticeMode = "prose") => invoke<Weakness[]>("weaknesses", { limit, mode });

export const heatmap = (mode: PracticeMode = "prose") => invoke<CharScore[]>("heatmap", { mode });

export const storageStatus = () => invoke<StorageStatus>("storage_status");
