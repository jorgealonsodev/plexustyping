import type { ErrorMode } from "../practice/engine";

const MODE_KEY = "plexustyping.errorMode";
const LIVE_KEY = "plexustyping.liveStats";
const KEYBOARD_KEY = "plexustyping.keyboard";
const FLASHES_KEY = "plexustyping.flashes";

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
