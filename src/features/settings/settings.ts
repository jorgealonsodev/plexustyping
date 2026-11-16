import type { ErrorMode } from "../practice/engine";

const MODE_KEY = "plexustyping.errorMode";
const LIVE_KEY = "plexustyping.liveStats";
const KEYBOARD_KEY = "plexustyping.keyboard";
const FLASHES_KEY = "plexustyping.flashes";
const HEATMAP_KEY = "plexustyping.heatmap";
const TARGETS_KEY = "plexustyping.targetCount";
const LENGTH_KEY = "plexustyping.textLength";

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
