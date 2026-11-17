import { invoke } from "@tauri-apps/api/core";

/** A bundled programming language (`plexustyping-corpus::CodeLanguage`). */
export interface CodeLanguage {
  id: string;
  name: string;
}

export type PracticeKind = "prose" | "words" | "code";

/** What is practised, as the backend commands take it. Words use the prose language. */
export type PracticeMode = "prose" | "words" | { kind: "code"; lang: string };

export const practiceMode = (kind: PracticeKind, lang: string): PracticeMode =>
  kind === "code" ? { kind: "code", lang } : kind;

/** Code of a language; prose and words depend on the prose language instead. */
export const isCodeMode = (mode: PracticeMode): mode is { kind: "code"; lang: string } => typeof mode === "object";

/** Identifies a practice mode: `prose`, `words` or `code:<lang>`. */
export const practiceKey = (mode: PracticeMode) => (isCodeMode(mode) ? `code:${mode.lang}` : mode);

export const codeLanguages = () => invoke<CodeLanguage[]>("code_languages");
