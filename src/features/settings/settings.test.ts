import { describe, expect, it } from "vitest";
import {
  ERROR_MODES,
  loadErrorMode,
  loadFlashes,
  loadLiveStats,
  loadShowKeyboard,
  saveErrorMode,
  saveFlashes,
  saveLiveStats,
  saveShowKeyboard,
} from "./settings";

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

describe("settings", () => {
  it("labels the three error modes in Spanish, allow-continue first", () => {
    expect(ERROR_MODES).toEqual([
      { id: "allow-continue", label: "Permitir seguir" },
      { id: "stop-on-error", label: "Parar hasta corregir" },
      { id: "backspace-correct", label: "Borrar con Retroceso" },
    ]);
  });

  it("defaults to allow-continue and ignores unknown stored modes", () => {
    expect(loadErrorMode(memoryStorage())).toBe("allow-continue");
    expect(loadErrorMode(memoryStorage({ "plexustyping.errorMode": "nope" }))).toBe("allow-continue");
  });

  it("persists the error mode", () => {
    const storage = memoryStorage();
    saveErrorMode("stop-on-error", storage);
    expect(storage.getItem("plexustyping.errorMode")).toBe("stop-on-error");
    expect(loadErrorMode(storage)).toBe("stop-on-error");
  });

  it("shows live stats by default and persists the toggle", () => {
    const storage = memoryStorage();
    expect(loadLiveStats(storage)).toBe(true);
    saveLiveStats(false, storage);
    expect(storage.getItem("plexustyping.liveStats")).toBe("false");
    expect(loadLiveStats(storage)).toBe(false);
    saveLiveStats(true, storage);
    expect(loadLiveStats(storage)).toBe(true);
  });

  it("shows the keyboard by default and persists the toggle", () => {
    const storage = memoryStorage();
    expect(loadShowKeyboard(storage)).toBe(true);
    saveShowKeyboard(false, storage);
    expect(storage.getItem("plexustyping.keyboard")).toBe("false");
    expect(loadShowKeyboard(storage)).toBe(false);
    saveShowKeyboard(true, storage);
    expect(loadShowKeyboard(storage)).toBe(true);
  });

  it("flashes keys by default and persists the toggle", () => {
    const storage = memoryStorage();
    expect(loadFlashes(storage)).toBe(true);
    saveFlashes(false, storage);
    expect(storage.getItem("plexustyping.flashes")).toBe("false");
    expect(loadFlashes(storage)).toBe(false);
    saveFlashes(true, storage);
    expect(loadFlashes(storage)).toBe(true);
  });

  it("keeps each toggle under its own key", () => {
    const storage = memoryStorage();
    saveShowKeyboard(false, storage);
    expect([loadLiveStats(storage), loadFlashes(storage), loadShowKeyboard(storage)]).toEqual([true, true, false]);
  });
});
