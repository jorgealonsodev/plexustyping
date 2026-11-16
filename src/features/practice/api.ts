import { invoke } from "@tauri-apps/api/core";
import type { ErrorMode, InputEvent, SessionResult } from "./engine";

/** `plexustyping-app` `ScoredSession`: authoritative replay plus chars to practice next. */
export interface ScoredSession {
  result: SessionResult;
  failedChars: string[];
}

export const LANGUAGE = "es";
export const LAYOUT = "es-qwerty";

export const diagnosticText = () => invoke<string>("diagnostic_text", { language: LANGUAGE });

export const nextText = (targets: string[], seed: number) =>
  invoke<string>("next_text", { language: LANGUAGE, layout: LAYOUT, targets, seed });

export const scoreSession = (text: string, mode: ErrorMode, events: InputEvent[]) =>
  invoke<ScoredSession>("score_session", { text, mode, events });
