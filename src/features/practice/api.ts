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
  /** The key progressive unlock added after this session (unlock S7); `null` when none. */
  unlocked?: string | null;
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

/**
 * `language` is the prose language; `layout` decides the chars a text may contain;
 * `mode` is prose, words or code; `maxLines` sizes code snippets.
 */
export const firstText = (language: string, layout: string, mode: PracticeMode = "prose", maxLines = 12) =>
  invoke<PracticeText>("first_text", { language, layout, day: localDay(), mode, maxLines });

export const nextText = (
  language: string,
  layout: string,
  seed: number,
  targetCount: number,
  length: number,
  fallback: string[],
  mode: PracticeMode = "prose",
  maxLines = 12,
) =>
  invoke<PracticeText>("next_text", {
    language,
    layout,
    seed,
    targetCount,
    length,
    fallback,
    day: localDay(),
    mode,
    maxLines,
  });

/**
 * The continuation of an unlimited text (unlimited S2, S4): about 600 more chars after `tail`, the
 * last ≤ 400 chars of the current (generated, never typed) text. Not cached by the backend.
 */
export const continueText = (
  language: string,
  layout: string,
  mode: "prose" | "words",
  tail: string,
  targetCount: number,
  seed: number,
  /** Skip the AI and chain a corpus text (after a failed or slow continuation). */
  offline = false,
) => invoke<PracticeText>("continue_text", { language, layout, mode, tail, targetCount, seed, day: localDay(), offline });

/** How a text was typed: prose, words or code of a language, and the code indentation. */
export interface Practice {
  mode: PracticeMode;
  indent: IndentMode;
}

/**
 * `offsetsMs` has one entry per event; `bpm` is the one metronome tempo of the text, if any.
 * Code sessions are stored with their language and folded into the code statistics (S5);
 * every session records the `language` of its text and the `layout` it was typed with (S5).
 */
export const scoreSession = (
  /** `null` (code texts) records the profile's language. */
  language: string | null,
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
    language,
  });

/** Of the statistics model of `mode` (prose and code are kept apart; words use prose). */
export const weaknesses = (limit: number, mode: PracticeMode = "prose") => invoke<Weakness[]>("weaknesses", { limit, mode });

export const heatmap = (mode: PracticeMode = "prose") => invoke<CharScore[]>("heatmap", { mode });

export const storageStatus = () => invoke<StorageStatus>("storage_status");
