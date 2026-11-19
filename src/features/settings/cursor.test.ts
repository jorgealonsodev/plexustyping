import { describe, expect, it } from "vitest";
import {
  CURSOR_BLINK_SETTING,
  CURSOR_SETTING,
  CURSOR_STYLES,
  DEFAULT_CURSOR_STYLE,
  cursorStyleById,
  loadCursorBlink,
  loadCursorStyle,
  saveCursorBlink,
  saveCursorStyle,
} from "./cursor";

const memoryStorage = (): Storage => {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => data.get(k) ?? null,
    key: (i) => [...data.keys()][i] ?? null,
    removeItem: (k) => void data.delete(k),
    setItem: (k, v) => void data.set(k, String(v)),
  };
};

describe("cursor style (cursor-style S1, S3)", () => {
  it("lists the five styles with their Spanish names; the bar is the default", () => {
    // "Tambien quiero que se pueda personalizar el tipo de cursor tipo barra y demas"
    // "la barra tiene que ser lo que aparece por defecto al instalar"
    expect(CURSOR_SETTING.label).toBe("Cursor");
    expect(CURSOR_STYLES.map((s) => [s.id, s.name])).toEqual([
      ["bar", "Barra"],
      ["highlight", "Resaltado"],
      ["block", "Bloque"],
      ["underline", "Subrayado"],
      ["outline", "Contorno"],
    ]);
    expect(DEFAULT_CURSOR_STYLE).toBe("bar");
  });

  it("falls back to the default for an unknown id", () => {
    expect(cursorStyleById("block").id).toBe("block");
    expect(cursorStyleById("nope").id).toBe("bar");
  });

  it("persists the style and falls back for an unknown stored value", () => {
    const storage = memoryStorage();
    expect(loadCursorStyle(storage)).toBe("bar");
    saveCursorStyle("block", storage);
    expect(loadCursorStyle(storage)).toBe("block");
    storage.setItem("plexustyping.cursorStyle", "zigzag");
    expect(loadCursorStyle(storage)).toBe("bar");
  });
});

describe("cursor blink (cursor-style S2, S3)", () => {
  it("is named in Spanish, on by default and persisted", () => {
    // "la barra tiene que ser como en monkey type, y tiene que parpadear"
    expect(CURSOR_BLINK_SETTING.label).toBe("Cursor parpadeante");
    const storage = memoryStorage();
    expect(loadCursorBlink(storage)).toBe(true);
    saveCursorBlink(false, storage);
    expect(loadCursorBlink(storage)).toBe(false);
    saveCursorBlink(true, storage);
    expect(loadCursorBlink(storage)).toBe(true);
  });
});
