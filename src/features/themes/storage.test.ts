import { describe, expect, it } from "vitest";
import { DEFAULT_THEME_ID, loadThemeId, saveThemeId } from "./index";

/** An in-memory `Storage` (vitest runs in node, without `localStorage`). */
function memoryStorage(initial: Record<string, string> = {}): Storage {
  const items = new Map(Object.entries(initial));
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, String(value)),
  };
}

describe("theme storage", () => {
  it("uses the default theme when nothing is stored", () => {
    expect(loadThemeId(memoryStorage())).toBe(DEFAULT_THEME_ID);
  });

  it("saves the chosen theme under plexustyping.theme and loads it back", () => {
    const storage = memoryStorage();
    saveThemeId("nord", storage);
    expect(storage.getItem("plexustyping.theme")).toBe("nord");
    expect(loadThemeId(storage)).toBe("nord");
  });

  it("falls back to the default for an unknown stored id", () => {
    expect(loadThemeId(memoryStorage({ "plexustyping.theme": "no-such-theme" }))).toBe(DEFAULT_THEME_ID);
  });
});
