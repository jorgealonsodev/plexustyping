import { invoke } from "@tauri-apps/api/core";
import type { CharScore } from "../keyboard/heat";
import type { ErrorMode, InputEvent, SessionResult } from "./engine";
import { localDay } from "./prefetch";

/** `plexustyping-app` `ScoredSession`: authoritative replay, chars to practise and storage outcome. */
export interface ScoredSession {
  result: SessionResult;
  failedChars: string[];
  saved: boolean;
  saveError: string | null;
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
export const LAYOUT = "es-qwerty";

export const firstText = () => invoke<PracticeText>("first_text", { language: LANGUAGE, day: localDay() });

export const nextText = (seed: number, targetCount: number, length: number, fallback: string[]) =>
  invoke<PracticeText>("next_text", {
    language: LANGUAGE,
    layout: LAYOUT,
    seed,
    targetCount,
    length,
    fallback,
    day: localDay(),
  });

export const scoreSession = (text: string, mode: ErrorMode, events: InputEvent[], startedAtMs: number) =>
  invoke<ScoredSession>("score_session", { text, mode, events, startedAtMs });

export const weaknesses = (limit: number) => invoke<Weakness[]>("weaknesses", { limit });

export const heatmap = () => invoke<CharScore[]>("heatmap");

export const storageStatus = () => invoke<StorageStatus>("storage_status");
