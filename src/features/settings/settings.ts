import { MAX_ACCENT_EVERY, MAX_BPM, MIN_BPM, type MetronomeSettings, SOUNDS } from "../metronome/beat";
import type { PracticeKind } from "../code/api";
import type { ErrorMode, IndentMode } from "../practice/engine";

const MODE_KEY = "plexustyping.errorMode";
const LIVE_KEY = "plexustyping.liveStats";
const KEYBOARD_KEY = "plexustyping.keyboard";
const FLASHES_KEY = "plexustyping.flashes";
const HEATMAP_KEY = "plexustyping.heatmap";
const TARGETS_KEY = "plexustyping.targetCount";
const LENGTH_KEY = "plexustyping.textLength";
const LAYOUT_KEY = "plexustyping.layout";
const LANGUAGE_KEY = "plexustyping.language";

/** Built-in keyboard layouts (`layouts/*.toml`) in display order; the first is the default. */
export const LAYOUTS: { id: string; label: string }[] = [
  { id: "es-qwerty", label: "Español (España)" },
  { id: "latam-qwerty", label: "Español (Latinoamérica)" },
  { id: "us-qwerty", label: "English (US)" },
  { id: "pt-abnt2", label: "Português (Brasil)" },
  { id: "fr-azerty", label: "Français (France)" },
  { id: "de-qwertz", label: "Deutsch (Deutschland)" },
];

export const DEFAULT_LAYOUT = LAYOUTS[0].id;

/** The stored layout id if it is a built-in one, else Spain's. */
export function loadLayout(storage: Storage = localStorage): string {
  const id = storage.getItem(LAYOUT_KEY);
  return LAYOUTS.find((l) => l.id === id)?.id ?? DEFAULT_LAYOUT;
}

export const saveLayout = (id: string, storage: Storage = localStorage) => storage.setItem(LAYOUT_KEY, id);

/** Prose languages (corpus ids) in display order; the first is the default. */
export const LANGUAGES: { id: string; label: string }[] = [
  { id: "es", label: "Español" },
  { id: "en", label: "English" },
  { id: "pt", label: "Português (Brasil)" },
  { id: "fr", label: "Français" },
  { id: "de", label: "Deutsch" },
];

export const DEFAULT_LANGUAGE = LANGUAGES[0].id;

/** The stored prose language if it is a listed one, else Spanish. */
export function loadLanguage(storage: Storage = localStorage): string {
  const id = storage.getItem(LANGUAGE_KEY);
  return LANGUAGES.find((l) => l.id === id)?.id ?? DEFAULT_LANGUAGE;
}

export const saveLanguage = (id: string, storage: Storage = localStorage) => storage.setItem(LANGUAGE_KEY, id);

/** Error modes in display order; the first is the default (S3). */
export const ERROR_MODES: { id: ErrorMode; label: string }[] = [
  { id: "allow-continue", label: "Permitir seguir" },
  { id: "stop-on-error", label: "Parar hasta corregir" },
  { id: "backspace-correct", label: "Borrar con Retroceso" },
];

export function loadErrorMode(storage: Storage = localStorage): ErrorMode {
  const id = storage.getItem(MODE_KEY);
  return ERROR_MODES.find((m) => m.id === id)?.id ?? ERROR_MODES[0].id;
}

export function saveErrorMode(mode: ErrorMode, storage: Storage = localStorage): void {
  storage.setItem(MODE_KEY, mode);
}

/** A toggle that is on unless turned off. */
const loadFlag = (key: string, storage: Storage) => storage.getItem(key) !== "false";

const saveFlag = (key: string, on: boolean, storage: Storage) => storage.setItem(key, String(on));

/** Whether the live stats bar is shown (on unless turned off). */
export const loadLiveStats = (storage: Storage = localStorage) => loadFlag(LIVE_KEY, storage);

export const saveLiveStats = (on: boolean, storage: Storage = localStorage) => saveFlag(LIVE_KEY, on, storage);

/** Whether the on-screen keyboard is shown (on unless turned off). */
export const loadShowKeyboard = (storage: Storage = localStorage) => loadFlag(KEYBOARD_KEY, storage);

export const saveShowKeyboard = (on: boolean, storage: Storage = localStorage) => saveFlag(KEYBOARD_KEY, on, storage);

/** Whether typed keys flash green or red on the keyboard (on unless turned off). */
export const loadFlashes = (storage: Storage = localStorage) => loadFlag(FLASHES_KEY, storage);

export const saveFlashes = (on: boolean, storage: Storage = localStorage) => saveFlag(FLASHES_KEY, on, storage);

/** Whether the keyboard is tinted by error score (off unless turned on). */
export const loadHeatmap = (storage: Storage = localStorage) => storage.getItem(HEATMAP_KEY) === "true";

export const saveHeatmap = (on: boolean, storage: Storage = localStorage) => saveFlag(HEATMAP_KEY, on, storage);

/** Choices of weak units practised per text (S2: 3–5); the first is the default. */
export const TARGET_COUNTS = [3, 4, 5];

/** Choices of practice text length in chars; 300 is the default. */
export const TEXT_LENGTHS = [100, 200, 300, 450, 600];

/** The stored number if it is one of `choices`, else `fallback`. */
function loadChoice(key: string, choices: number[], fallback: number, storage: Storage): number {
  const value = Number(storage.getItem(key));
  return choices.includes(value) ? value : fallback;
}

export const loadTargetCount = (storage: Storage = localStorage) =>
  loadChoice(TARGETS_KEY, TARGET_COUNTS, TARGET_COUNTS[0], storage);

export const saveTargetCount = (count: number, storage: Storage = localStorage) =>
  storage.setItem(TARGETS_KEY, String(count));

export const loadTextLength = (storage: Storage = localStorage) => loadChoice(LENGTH_KEY, TEXT_LENGTHS, 300, storage);

export const saveTextLength = (length: number, storage: Storage = localStorage) =>
  storage.setItem(LENGTH_KEY, String(length));

const METRONOME_KEY = "plexustyping.metronome";

/** Off, 60 BPM (S7: one beat = one expected keystroke), click, 80 %, accent every 4, fixed tempo. */
export const DEFAULT_METRONOME: MetronomeSettings = {
  enabled: false,
  bpm: 60,
  sound: SOUNDS[0].id,
  volume: 80,
  accentEvery: 4,
  adaptive: false,
};

const intIn = (value: unknown, min: number, max: number) =>
  Number.isInteger(value) && (value as number) >= min && (value as number) <= max;

/** The stored metronome settings; each missing or invalid field takes its default. */
export function loadMetronome(storage: Storage = localStorage): MetronomeSettings {
  let stored: Partial<Record<keyof MetronomeSettings, unknown>> = {};
  try {
    stored = JSON.parse(storage.getItem(METRONOME_KEY) ?? "{}") ?? {};
  } catch {
    // Corrupt value: defaults.
  }
  return checkMetronome(stored, DEFAULT_METRONOME);
}

/** `value` with each missing or invalid field taken from `d`. */
export function checkMetronome(
  stored: Partial<Record<keyof MetronomeSettings, unknown>>,
  d: MetronomeSettings,
): MetronomeSettings {
  return {
    enabled: typeof stored.enabled === "boolean" ? stored.enabled : d.enabled,
    bpm: intIn(stored.bpm, MIN_BPM, MAX_BPM) ? (stored.bpm as number) : d.bpm,
    sound: SOUNDS.find((s) => s.id === stored.sound)?.id ?? d.sound,
    volume: intIn(stored.volume, 0, 100) ? (stored.volume as number) : d.volume,
    accentEvery: intIn(stored.accentEvery, 0, MAX_ACCENT_EVERY) ? (stored.accentEvery as number) : d.accentEvery,
    adaptive: typeof stored.adaptive === "boolean" ? stored.adaptive : d.adaptive,
  };
}

export const saveMetronome = (settings: MetronomeSettings, storage: Storage = localStorage) =>
  storage.setItem(METRONOME_KEY, JSON.stringify(settings));

const KIND_KEY = "plexustyping.practiceKind";
const CODE_LANGUAGE_KEY = "plexustyping.codeLanguage";
const CODE_INDENT_KEY = "plexustyping.codeIndent";
const CODE_LINES_KEY = "plexustyping.codeLines";

/** Prose unless code was chosen (S1). */
export const loadPracticeKind = (storage: Storage = localStorage): PracticeKind =>
  storage.getItem(KIND_KEY) === "code" ? "code" : "prose";

export const savePracticeKind = (kind: PracticeKind, storage: Storage = localStorage) => storage.setItem(KIND_KEY, kind);

/** Default code language; the backend list decides which ids exist. */
export const DEFAULT_CODE_LANGUAGE = "python";

/** The stored language id if it looks like one (`rust`, `html-css`), else Python. */
export function loadCodeLanguage(storage: Storage = localStorage): string {
  const id = storage.getItem(CODE_LANGUAGE_KEY) ?? "";
  return /^[a-z][a-z-]*$/.test(id) ? id : DEFAULT_CODE_LANGUAGE;
}

export const saveCodeLanguage = (id: string, storage: Storage = localStorage) => storage.setItem(CODE_LANGUAGE_KEY, id);

/** Indentation modes in display order; the first is the default (S4). */
export const CODE_INDENTS: { id: IndentMode; label: string }[] = [
  { id: "auto", label: "Automática al pulsar Enter" },
  { id: "manual", label: "Manual" },
];

export function loadCodeIndent(storage: Storage = localStorage): IndentMode {
  const id = storage.getItem(CODE_INDENT_KEY);
  return CODE_INDENTS.find((m) => m.id === id)?.id ?? CODE_INDENTS[0].id;
}

export const saveCodeIndent = (indent: IndentMode, storage: Storage = localStorage) =>
  storage.setItem(CODE_INDENT_KEY, indent);

/** Snippet size in lines (S2: 5–20); 12 is the default. */
export const MIN_CODE_LINES = 5;
export const MAX_CODE_LINES = 20;
export const DEFAULT_CODE_LINES = 12;

export function loadCodeLines(storage: Storage = localStorage): number {
  const raw = storage.getItem(CODE_LINES_KEY);
  const value = raw === null || raw === "" ? NaN : Number(raw);
  return intIn(value, MIN_CODE_LINES, MAX_CODE_LINES) ? value : DEFAULT_CODE_LINES;
}

export const saveCodeLines = (lines: number, storage: Storage = localStorage) =>
  storage.setItem(CODE_LINES_KEY, String(lines));
