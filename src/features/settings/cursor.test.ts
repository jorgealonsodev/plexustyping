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
  it("lists the five styles with their Spanish names; the default is today's look", () => {
    // "Tambien quiero que se pueda personalizar el tipo de cursor tipo barra y demas"
    expect(CURSOR_SETTING.label).toBe("Cursor");
    expect(CURSOR_STYLES.map((s) => [s.id, s.name])).toEqual([
      ["highlight", "Resaltado"],
      ["bar", "Barra"],
      ["block", "Bloque"],
      ["underline", "Subrayado"],
      ["outline", "Contorno"],
    ]);
    expect(DEFAULT_CURSOR_STYLE).toBe("highlight");
  });

  it("falls back to the default for an unknown id", () => {
    expect(cursorStyleById("bar").id).toBe("bar");
    expect(cursorStyleById("nope").id).toBe("highlight");
  });

  it("persists the style and falls back for an unknown stored value", () => {
    const storage = memoryStorage();
    expect(loadCursorStyle(storage)).toBe("highlight");
    saveCursorStyle("block", storage);
    expect(loadCursorStyle(storage)).toBe("block");
    storage.setItem("plexustyping.cursorStyle", "zigzag");
    expect(loadCursorStyle(storage)).toBe("highlight");
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
