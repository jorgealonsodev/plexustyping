// Background prefetch of the next text (S6) and the label of a text's origin.

import type { PracticeText } from "./api";
import { t } from "./messages";

/** Short provider names, as the backend uses them in notices. */
export const PROVIDER_NAMES: Record<string, string> = {
  claude: "Claude",
  openai: "OpenAI",
  gemini: "Gemini",
  "opencode-go": "OpenCode Go",
  chatgpt: "ChatGPT",
};

/** Where a text came from, shown discreetly above it. */
export function sourceLabel(text: Pick<PracticeText, "source" | "provider" | "model" | "diagnostic">): string {
  if (text.diagnostic) return t("sourceDiagnostic");
  switch (text.source) {
    case "ai": {
      const provider = text.provider ? (PROVIDER_NAMES[text.provider] ?? text.provider) : null;
      return [t("sourceAi"), provider, text.model].filter(Boolean).join(" · ");
    }
    case "cache":
      return t("sourceCache");
    default:
      return t("sourceCorpus");
  }
}

/**
 * Identifies the settings a next text depends on; a prefetch made under another key is stale.
 * `practice` is a `practiceKey` (`prose`, `words` or `code:<lang>`), `codeLines` the snippet size,
 * `layout` the keyboard layout id, `language` the prose language id.
 */
export const generationKey = (
  targetCount: number,
  textLength: number,
  aiRevision: number,
  practice = "prose",
  codeLines = 12,
  layout = "es-qwerty",
  language = "es",
) => `${targetCount}|${textLength}|${aiRevision}|${practice}|${codeLines}|${layout}|${language}`;

/** Holds at most one background request, tagged with the generation key it was made for. */
export class Prefetcher<T> {
  private slot: { key: string; promise: Promise<T> } | null = null;

  /** Starts `request` for `key` unless one for that key is already held; replaces a stale one. */
  start(key: string, request: () => Promise<T>): void {
    if (this.slot?.key === key) return;
    const promise = request();
    // The taker sees a rejection; an untaken one must not be reported as unhandled.
    promise.catch(() => {});
    this.slot = { key, promise };
  }

  has(key: string): boolean {
    return this.slot?.key === key;
  }

  /** The held request if it was made for `key` (pending or settled), consumed; `null` otherwise. */
  take(key: string): Promise<T> | null {
    const slot = this.slot;
    this.slot = null;
    return slot && slot.key === key ? slot.promise : null;
  }

  clear(): void {
    this.slot = null;
  }
}

/** Expected chars of wrong keystrokes so far, most failed first (mirrors `failed_chars`). */
export function provisionalFailedChars(records: readonly { expected: string; correct: boolean }[]): string[] {
  const counts = new Map<string, number>();
  for (const r of records) if (!r.correct) counts.set(r.expected, (counts.get(r.expected) ?? 0) + 1);
  // Map keeps first-seen order and sort is stable, so ties stay in that order.
  return [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([ch]) => ch);
}

/** The local calendar day as `YYYY-MM-DD` (for the per-day usage counters). */
export function localDay(d: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Whether two `profile|mode|layout|language` request keys belong to different profiles. */
export const isProfileSwitch = (from: string, to: string): boolean => from.split("|")[0] !== to.split("|")[0];
