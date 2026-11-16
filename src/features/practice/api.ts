import { invoke } from "@tauri-apps/api/core";
import type { CharScore } from "../keyboard/heat";
import type { ErrorMode, InputEvent, SessionResult } from "./engine";

/** `plexustyping-app` `ScoredSession`: authoritative replay, chars to practise and storage outcome. */
export interface ScoredSession {
  result: SessionResult;
  failedChars: string[];
  saved: boolean;
  saveError: string | null;
}

/** A practice text and the weak units (chars or bigrams) it really practises. */
export interface PracticeText {
  text: string;
  targets: string[];
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

export const firstText = () => invoke<PracticeText>("first_text", { language: LANGUAGE });

export const nextText = (seed: number, targetCount: number, length: number, fallback: string[]) =>
  invoke<PracticeText>("next_text", { language: LANGUAGE, layout: LAYOUT, seed, targetCount, length, fallback });

export const scoreSession = (text: string, mode: ErrorMode, events: InputEvent[], startedAtMs: number) =>
  invoke<ScoredSession>("score_session", { text, mode, events, startedAtMs });

export const weaknesses = (limit: number) => invoke<Weakness[]>("weaknesses", { limit });

export const heatmap = () => invoke<CharScore[]>("heatmap");

export const storageStatus = () => invoke<StorageStatus>("storage_status");
