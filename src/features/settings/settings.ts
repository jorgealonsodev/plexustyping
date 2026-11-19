import { MAX_ACCENT_EVERY, MAX_BPM, MIN_BPM, type MetronomeSettings, SOUNDS } from "../metronome/beat";
import { settingsStorage } from "./persist";
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
const GHOST_KEY = "plexustyping.ghost";

/** Built-in keyboard layouts (`layouts/*.toml`) in display order; the first is the default. */
export const LAYOUTS: { id: string; label: string }[] = [
  { id: "es-qwerty", label: "Español (España)" },
  { id: "latam-qwerty", label: "Español (Latinoamérica)" },
  { id: "us-qwerty", label: "English (US)" },
  { id: "pt-abnt2", label: "Português (Brasil)" },
  { id: "fr-azerty", label: "Français (France)" },
  { id: "de-qwertz", label: "Deutsch (Deutschland)" },
  { id: "us-dvorak", label: "English (Dvorak)" },
  { id: "us-colemak", label: "English (Colemak)" },
];

export const DEFAULT_LAYOUT = LAYOUTS[0].id;

/** The stored layout id if it is a built-in one, else Spain's. */
export function loadLayout(storage: Storage = settingsStorage()): string {
  const id = storage.getItem(LAYOUT_KEY);
  return LAYOUTS.find((l) => l.id === id)?.id ?? DEFAULT_LAYOUT;
}

export const saveLayout = (id: string, storage: Storage = settingsStorage()) => storage.setItem(LAYOUT_KEY, id);

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
export function loadLanguage(storage: Storage = settingsStorage()): string {
  const id = storage.getItem(LANGUAGE_KEY);
  return LANGUAGES.find((l) => l.id === id)?.id ?? DEFAULT_LANGUAGE;
}

export const saveLanguage = (id: string, storage: Storage = settingsStorage()) => storage.setItem(LANGUAGE_KEY, id);

/** Error modes in display order; the first is the default (S3). */
export const ERROR_MODES: { id: ErrorMode; label: string }[] = [
  { id: "allow-continue", label: "Permitir seguir" },
  { id: "stop-on-error", label: "Parar hasta corregir" },
  { id: "backspace-correct", label: "Borrar con Retroceso" },
];

export function loadErrorMode(storage: Storage = settingsStorage()): ErrorMode {
  const id = storage.getItem(MODE_KEY);
  return ERROR_MODES.find((m) => m.id === id)?.id ?? ERROR_MODES[0].id;
}

export function saveErrorMode(mode: ErrorMode, storage: Storage = settingsStorage()): void {
  storage.setItem(MODE_KEY, mode);
}

/** A toggle that is on unless turned off. */
const loadFlag = (key: string, storage: Storage) => storage.getItem(key) !== "false";

const saveFlag = (key: string, on: boolean, storage: Storage) => storage.setItem(key, String(on));

/** Whether the live stats bar is shown (on unless turned off). */
export const loadLiveStats = (storage: Storage = settingsStorage()) => loadFlag(LIVE_KEY, storage);

export const saveLiveStats = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(LIVE_KEY, on, storage);

/** Ajustes → Práctica toggle of the ghost session (ghost S3). */
export const GHOST_SETTING = {
  label: "Sesión fantasma",
  hint: "Corre contra tu mejor sesión con el mismo texto",
};

/** Whether a text with a record races its ghost session (on unless turned off). */
export const loadGhost = (storage: Storage = settingsStorage()) => loadFlag(GHOST_KEY, storage);

export const saveGhost = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(GHOST_KEY, on, storage);

/** Whether the on-screen keyboard is shown (on unless turned off). */
export const loadShowKeyboard = (storage: Storage = settingsStorage()) => loadFlag(KEYBOARD_KEY, storage);

export const saveShowKeyboard = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(KEYBOARD_KEY, on, storage);

/** Whether typed keys flash green or red on the keyboard (on unless turned off). */
export const loadFlashes = (storage: Storage = settingsStorage()) => loadFlag(FLASHES_KEY, storage);

export const saveFlashes = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(FLASHES_KEY, on, storage);

/** Whether the keyboard is tinted by error score (off unless turned on). */
export const loadHeatmap = (storage: Storage = settingsStorage()) => storage.getItem(HEATMAP_KEY) === "true";

export const saveHeatmap = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(HEATMAP_KEY, on, storage);

const COLOR_BLIND_KEY = "plexustyping.colorBlind";

/** Ajustes → Teclado toggle of the colour-blind flashes (a11y S5). */
export const COLOR_BLIND_SETTING = {
  label: "Modo daltónico",
  hint: "Destellos en azul y naranja en lugar de verde y rojo; una tecla errónea se subraya.",
};

/** Whether the keyboard flashes use the colour-blind pair (off unless turned on). */
export const loadColorBlind = (storage: Storage = settingsStorage()) => storage.getItem(COLOR_BLIND_KEY) === "true";

export const saveColorBlind = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(COLOR_BLIND_KEY, on, storage);

const DIM_PENDING_KEY = "plexustyping.dimPending";

/** Ajustes → Apariencia toggle of the dimmed pending text (a11y S7). */
export const DIM_PENDING_SETTING = {
  label: "Atenuar texto pendiente",
  hint: "Baja el contraste del texto por escribir por debajo del nivel AA para que destaque lo escrito; desactívalo para recuperarlo.",
};

/** Whether the practice text's pending chars use the dimmed colours (on unless turned off). */
export const loadDimPending = (storage: Storage = settingsStorage()) => loadFlag(DIM_PENDING_KEY, storage);

export const saveDimPending = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(DIM_PENDING_KEY, on, storage);

/** Choices of weak units practised per text (2 or 3); the first is the default. */
export const TARGET_COUNTS = [2, 3];

/** The "Ilimitado" length (unlimited S1, S2): a prose or words text that keeps growing while typed. */
export const UNLIMITED_LENGTH = 0;

/** Choices of practice text length in chars, then "Ilimitado"; 300 is the default. */
export const TEXT_LENGTHS = [100, 200, 300, 450, 600, 800, 1000, 1500, 2000, UNLIMITED_LENGTH];

/** How a length choice reads in "Longitud". */
export const lengthLabel = (length: number) => (length === UNLIMITED_LENGTH ? "Ilimitado" : String(length));

/** The stored number if it is one of `choices`, else `fallback` (also when missing or empty). */
function loadChoice(key: string, choices: number[], fallback: number, storage: Storage): number {
  const raw = storage.getItem(key);
  const value = raw === null || raw.trim() === "" ? NaN : Number(raw);
  return choices.includes(value) ? value : fallback;
}

export const loadTargetCount = (storage: Storage = settingsStorage()) =>
  loadChoice(TARGETS_KEY, TARGET_COUNTS, TARGET_COUNTS[0], storage);

export const saveTargetCount = (count: number, storage: Storage = settingsStorage()) =>
  storage.setItem(TARGETS_KEY, String(count));

export const loadTextLength = (storage: Storage = settingsStorage()) => loadChoice(LENGTH_KEY, TEXT_LENGTHS, 300, storage);

export const saveTextLength = (length: number, storage: Storage = settingsStorage()) =>
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
export function loadMetronome(storage: Storage = settingsStorage()): MetronomeSettings {
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

export const saveMetronome = (settings: MetronomeSettings, storage: Storage = settingsStorage()) =>
  storage.setItem(METRONOME_KEY, JSON.stringify(settings));

const KIND_KEY = "plexustyping.practiceKind";
const CODE_LANGUAGE_KEY = "plexustyping.codeLanguage";
const CODE_INDENT_KEY = "plexustyping.codeIndent";
const CODE_LINES_KEY = "plexustyping.codeLines";

/** Words or code when chosen; prose for a missing or unknown value (S1). */
export function loadPracticeKind(storage: Storage = settingsStorage()): PracticeKind {
  const kind = storage.getItem(KIND_KEY);
  return kind === "code" || kind === "words" ? kind : "prose";
}

export const savePracticeKind = (kind: PracticeKind, storage: Storage = settingsStorage()) => storage.setItem(KIND_KEY, kind);

/** Default code language; the backend list decides which ids exist. */
export const DEFAULT_CODE_LANGUAGE = "python";

/** The stored language id if it looks like one (`rust`, `html-css`), else Python. */
export function loadCodeLanguage(storage: Storage = settingsStorage()): string {
  const id = storage.getItem(CODE_LANGUAGE_KEY) ?? "";
  return /^[a-z][a-z-]*$/.test(id) ? id : DEFAULT_CODE_LANGUAGE;
}

export const saveCodeLanguage = (id: string, storage: Storage = settingsStorage()) => storage.setItem(CODE_LANGUAGE_KEY, id);

/** Indentation modes in display order; the first is the default (S4). */
export const CODE_INDENTS: { id: IndentMode; label: string }[] = [
  { id: "auto", label: "Automática al pulsar Enter" },
  { id: "manual", label: "Manual" },
];

export function loadCodeIndent(storage: Storage = settingsStorage()): IndentMode {
  const id = storage.getItem(CODE_INDENT_KEY);
  return CODE_INDENTS.find((m) => m.id === id)?.id ?? CODE_INDENTS[0].id;
}

export const saveCodeIndent = (indent: IndentMode, storage: Storage = settingsStorage()) =>
  storage.setItem(CODE_INDENT_KEY, indent);

/** Snippet size in lines (S2: 5–20); 12 is the default. */
export const MIN_CODE_LINES = 5;
export const MAX_CODE_LINES = 20;
export const DEFAULT_CODE_LINES = 12;

export function loadCodeLines(storage: Storage = settingsStorage()): number {
  const raw = storage.getItem(CODE_LINES_KEY);
  const value = raw === null || raw === "" ? NaN : Number(raw);
  return intIn(value, MIN_CODE_LINES, MAX_CODE_LINES) ? value : DEFAULT_CODE_LINES;
}

export const saveCodeLines = (lines: number, storage: Storage = settingsStorage()) =>
  storage.setItem(CODE_LINES_KEY, String(lines));

const SCREEN_READER_KEY = "plexustyping.screenReader";

/** Ajustes → Práctica toggle of the screen reader mode (a11y S3). */
export const SCREEN_READER_SETTING = {
  label: "Anunciar con lector de pantalla",
  hint: "Anuncia el siguiente carácter, los errores, cada texto nuevo y el resultado (aria-live).",
};

/** Whether the practice announces to screen readers (off unless turned on). */
export const loadScreenReader = (storage: Storage = settingsStorage()) => storage.getItem(SCREEN_READER_KEY) === "true";

export const saveScreenReader = (on: boolean, storage: Storage = settingsStorage()) => saveFlag(SCREEN_READER_KEY, on, storage);

const TYPOGRAPHY_KEY = "plexustyping.typography";

/** Size (% of the default), letter spacing (em) and line height of the practice text, prose and code (a11y S4). */
export interface Typography {
  fontScale: number;
  letterSpacing: number;
  lineHeight: number;
}

/** 100 %, no extra spacing and the line height the practice text always had (L3). */
export const DEFAULT_TYPOGRAPHY: Typography = { fontScale: 100, letterSpacing: 0, lineHeight: 1.75 };

/** The Ajustes → Apariencia rows in display order, with each range and step (a11y S4). */
export const TYPOGRAPHY_FIELDS: { key: keyof Typography; label: string; min: number; max: number; step: number }[] = [
  { key: "fontScale", label: "Tamaño de letra", min: 80, max: 200, step: 10 },
  { key: "letterSpacing", label: "Espaciado entre letras", min: 0, max: 0.3, step: 0.02 },
  { key: "lineHeight", label: "Interlineado", min: 1.2, max: 2.4, step: 0.1 },
];

/** `stored` with each value clamped to its range (to 2 decimals) and each non-number taken from `d`. */
export function checkTypography(
  stored: Partial<Record<keyof Typography, unknown>>,
  d: Typography = DEFAULT_TYPOGRAPHY,
): Typography {
  const next = { ...d };
  for (const { key, min, max } of TYPOGRAPHY_FIELDS) {
    const value = stored[key];
    if (typeof value !== "number" || !Number.isFinite(value)) continue;
    next[key] = Math.round(Math.min(Math.max(value, min), max) * 100) / 100;
  }
  return next;
}

export function loadTypography(storage: Storage = settingsStorage()): Typography {
  let stored: Partial<Record<keyof Typography, unknown>> = {};
  try {
    stored = JSON.parse(storage.getItem(TYPOGRAPHY_KEY) ?? "{}") ?? {};
  } catch {
    // Corrupt value: defaults.
  }
  return checkTypography(typeof stored === "object" ? stored : {});
}

export const saveTypography = (typography: Typography, storage: Storage = settingsStorage()) =>
  storage.setItem(TYPOGRAPHY_KEY, JSON.stringify(typography));

/** The CSS variables the prose and code practice text read. */
export const typographyStyle = (t: Typography) =>
  `--text-scale: ${t.fontScale / 100}; --text-spacing: ${t.letterSpacing}em; --text-leading: ${t.lineHeight}`;
