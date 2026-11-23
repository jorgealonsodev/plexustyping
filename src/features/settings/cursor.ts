import { settingsStorage } from "./persist";
import { t } from "./textMessages";

/** Ajustes → Texto select of the typing cursor's look, prose and code (cursor-style S1). */
export const CURSOR_SETTING = {
  get label() {
    return t("cursorLabel");
  },
  get hint() {
    return t("cursorHint");
  },
};

export const CURSOR_BLINK_SETTING = {
  get label() {
    return t("cursorBlinkLabel");
  },
  get hint() {
    return t("cursorBlinkHint");
  },
};

export interface CursorStyle {
  readonly id: string;
  /** In the interface language. */
  readonly name: string;
}

const style = (id: string, key: Parameters<typeof t>[0]): CursorStyle => ({
  id,
  get name() {
    return t(key);
  },
});

/** Display order; the first, the Monkeytype-style bar, is the default. The practice text reads the id from `data-cursor`. */
export const CURSOR_STYLES: CursorStyle[] = [
  style("bar", "cursorBar"),
  style("highlight", "cursorHighlight"),
  style("block", "cursorBlock"),
  style("underline", "cursorUnderline"),
  style("outline", "cursorOutline"),
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
