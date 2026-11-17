import { invoke } from "@tauri-apps/api/core";

/** A bundled programming language (`plexustyping-corpus::CodeLanguage`). */
export interface CodeLanguage {
  id: string;
  name: string;
}

export type PracticeKind = "prose" | "code";

/** What is practised, as the backend commands take it. */
export type PracticeMode = "prose" | { kind: "code"; lang: string };

export const practiceMode = (kind: PracticeKind, lang: string): PracticeMode =>
  kind === "code" ? { kind: "code", lang } : "prose";

/** Identifies a practice mode: `prose` or `code:<lang>`. */
export const practiceKey = (mode: PracticeMode) => (mode === "prose" ? "prose" : `code:${mode.lang}`);

export const codeLanguages = () => invoke<CodeLanguage[]>("code_languages");
