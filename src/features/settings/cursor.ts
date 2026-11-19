import { settingsStorage } from "./persist";

/** Ajustes → Texto select of the typing cursor's look, prose and code (cursor-style S1). */
export const CURSOR_SETTING = {
  label: "Cursor",
  hint: "Cómo se marca el siguiente carácter que hay que escribir.",
};

export const CURSOR_BLINK_SETTING = {
  label: "Cursor parpadeante",
  hint: "El cursor parpadea mientras esperas y se queda fijo al escribir; no lo hace con el movimiento reducido del sistema.",
};

export interface CursorStyle {
  id: string;
  name: string;
}

/** Display order; the first is today's look, the default. The practice text reads the id from `data-cursor`. */
export const CURSOR_STYLES: CursorStyle[] = [
  { id: "highlight", name: "Resaltado" },
  { id: "bar", name: "Barra" },
  { id: "block", name: "Bloque" },
  { id: "underline", name: "Subrayado" },
  { id: "outline", name: "Contorno" },
];

export const DEFAULT_CURSOR_STYLE = CURSOR_STYLES[0].id;

const STYLE_KEY = "plexustyping.cursorStyle";
const BLINK_KEY = "plexustyping.cursorBlink";

/** The style `id`, or the default for an unknown one. */
export const cursorStyleById = (id: string): CursorStyle => CURSOR_STYLES.find((s) => s.id === id) ?? CURSOR_STYLES[0];

export const loadCursorStyle = (storage: Storage = settingsStorage()): string =>
  cursorStyleById(storage.getItem(STYLE_KEY) ?? "").id;

export const saveCursorStyle = (id: string, storage: Storage = settingsStorage()) => storage.setItem(STYLE_KEY, id);

/** On unless turned off ("tiene que parpadear"). */
export const loadCursorBlink = (storage: Storage = settingsStorage()): boolean => storage.getItem(BLINK_KEY) !== "false";

export const saveCursorBlink = (on: boolean, storage: Storage = settingsStorage()) => storage.setItem(BLINK_KEY, String(on));
