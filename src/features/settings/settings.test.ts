import { describe, expect, it } from "vitest";
import { ERROR_MODES, loadErrorMode, loadLiveStats, saveErrorMode, saveLiveStats } from "./settings";

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
});
