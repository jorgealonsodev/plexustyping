import type { ErrorMode } from "../practice/engine";

const MODE_KEY = "plexustyping.errorMode";
const LIVE_KEY = "plexustyping.liveStats";

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

/** Whether the live stats bar is shown (on unless turned off). */
export function loadLiveStats(storage: Storage = localStorage): boolean {
  return storage.getItem(LIVE_KEY) !== "false";
}

export function saveLiveStats(on: boolean, storage: Storage = localStorage): void {
  storage.setItem(LIVE_KEY, String(on));
}
