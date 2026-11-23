import { formatNumber } from "../../i18n/i18n.svelte";
import { MAX_CODE_LINES, MIN_CODE_LINES, TEXT_LENGTHS, UNLIMITED_LENGTH, lengthLabel } from "../settings/settings";
import type { PracticeKind } from "./api";
import { t } from "./messages";

/** Maximum-lines presets of the header picker; Ajustes → Código still takes any value in range. */
export const CODE_LINE_PRESETS = [5, 8, 12, 16, 20].filter((n) => n >= MIN_CODE_LINES && n <= MAX_CODE_LINES);

export interface LengthChoice {
  value: number;
  label: string;
}

/**
 * The header length picker's options: the settings' text lengths for prose and words,
 * maximum lines for code (plus `codeLines` when it is not a preset, so the current value shows).
 */
export function lengthChoices(kind: PracticeKind, codeLines: number): LengthChoice[] {
  if (kind === "code") {
    const values = CODE_LINE_PRESETS.includes(codeLines)
      ? CODE_LINE_PRESETS
      : [...CODE_LINE_PRESETS, codeLines].sort((a, b) => a - b);
    return values.map((n) => ({ value: n, label: t("lines", { n, shown: formatNumber(n) }) }));
  }
  return TEXT_LENGTHS.map((n) => ({
    value: n,
    label: n === UNLIMITED_LENGTH ? lengthLabel(n) : t("chars", { n: formatNumber(n) }),
  }));
}
