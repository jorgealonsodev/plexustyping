import { describe, expect, it } from "vitest";
import {
  CODE_INDENTS,
  ERROR_MODES,
  MAX_CODE_LINES,
  MIN_CODE_LINES,
  checkMetronome,
  loadCodeIndent,
  loadCodeLanguage,
  loadCodeLines,
  loadPracticeKind,
  saveCodeIndent,
  saveCodeLanguage,
  saveCodeLines,
  savePracticeKind,
  TARGET_COUNTS,
  TEXT_LENGTHS,
  loadErrorMode,
  loadFlashes,
  loadHeatmap,
  loadLiveStats,
  loadMetronome,
  loadShowKeyboard,
  loadTargetCount,
  loadTextLength,
  saveErrorMode,
  saveFlashes,
  saveHeatmap,
  saveLiveStats,
  saveMetronome,
  saveShowKeyboard,
  saveTargetCount,
  saveTextLength,
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

  it("hides the heatmap by default and persists the toggle", () => {
    const storage = memoryStorage();
    expect(loadHeatmap(storage)).toBe(false);
    saveHeatmap(true, storage);
    expect(storage.getItem("plexustyping.heatmap")).toBe("true");
    expect(loadHeatmap(storage)).toBe(true);
    saveHeatmap(false, storage);
    expect(loadHeatmap(storage)).toBe(false);
    expect(loadHeatmap(memoryStorage({ "plexustyping.heatmap": "yes" }))).toBe(false);
  });

  it("offers 3 to 5 targets, 3 by default, and persists the choice", () => {
    expect(TARGET_COUNTS).toEqual([3, 4, 5]);
    const storage = memoryStorage();
    expect(loadTargetCount(storage)).toBe(3);
    saveTargetCount(5, storage);
    expect(storage.getItem("plexustyping.targetCount")).toBe("5");
    expect(loadTargetCount(storage)).toBe(5);
    for (const bad of ["2", "6", "4.5", "x"])
      expect(loadTargetCount(memoryStorage({ "plexustyping.targetCount": bad }))).toBe(3);
  });

  it("offers five text lengths, 300 by default, and persists the choice", () => {
    expect(TEXT_LENGTHS).toEqual([100, 200, 300, 450, 600]);
    const storage = memoryStorage();
    expect(loadTextLength(storage)).toBe(300);
    saveTextLength(600, storage);
    expect(storage.getItem("plexustyping.textLength")).toBe("600");
    expect(loadTextLength(storage)).toBe(600);
    for (const bad of ["50", "301", "", "abc"])
      expect(loadTextLength(memoryStorage({ "plexustyping.textLength": bad }))).toBe(300);
  });

  it("keeps the new settings apart from the toggles", () => {
    const storage = memoryStorage();
    saveHeatmap(true, storage);
    saveTargetCount(4, storage);
    saveTextLength(100, storage);
    expect([loadLiveStats(storage), loadFlashes(storage), loadShowKeyboard(storage)]).toEqual([true, true, true]);
    expect([loadHeatmap(storage), loadTargetCount(storage), loadTextLength(storage)]).toEqual([true, 4, 100]);
  });

  it("keeps the metronome off at 60 BPM by default and persists every field", () => {
    const storage = memoryStorage();
    expect(loadMetronome(storage)).toEqual({ enabled: false, bpm: 60, sound: "click", volume: 80, accentEvery: 4 });
    const next = { enabled: true, bpm: 300, sound: "beep" as const, volume: 0, accentEvery: 16 };
    saveMetronome(next, storage);
    expect(loadMetronome(storage)).toEqual(next);
    saveMetronome({ ...next, bpm: 30, accentEvery: 0, volume: 100, sound: "wood" }, storage);
    expect(loadMetronome(storage)).toEqual({ ...next, bpm: 30, accentEvery: 0, volume: 100, sound: "wood" });
  });

  it("replaces an invalid stored metronome field by its default, keeping the others", () => {
    const load = (value: unknown) => loadMetronome(memoryStorage({ "plexustyping.metronome": JSON.stringify(value) }));
    expect(load({ enabled: true, bpm: 29, sound: "gong", volume: 101, accentEvery: 17 })).toEqual({
      enabled: true,
      bpm: 60,
      sound: "click",
      volume: 80,
      accentEvery: 4,
    });
    expect(load({ bpm: 120.5, volume: -1, accentEvery: 2 })).toEqual({
      enabled: false,
      bpm: 60,
      sound: "click",
      volume: 80,
      accentEvery: 2,
    });
    expect(loadMetronome(memoryStorage({ "plexustyping.metronome": "{nope" }))).toEqual(load({}));
    const current = { enabled: true, bpm: 90, sound: "wood" as const, volume: 30, accentEvery: 3 };
    expect(checkMetronome({ ...current, bpm: Number.NaN, volume: 50 }, current)).toEqual({ ...current, volume: 50 });
  });
});

describe("code settings", () => {
  it("defaults to prose, Python, auto indent on Enter and 12 lines", () => {
    const storage = memoryStorage();
    expect(loadPracticeKind(storage)).toBe("prose");
    expect(loadCodeLanguage(storage)).toBe("python");
    expect(loadCodeIndent(storage)).toBe("auto");
    expect(loadCodeLines(storage)).toBe(12);
    expect(CODE_INDENTS.map((i) => i.label)).toEqual(["Automática al pulsar Enter", "Manual"]);
    expect([MIN_CODE_LINES, MAX_CODE_LINES]).toEqual([5, 20]);
  });

  it("round-trips every code setting", () => {
    const storage = memoryStorage();
    savePracticeKind("code", storage);
    saveCodeLanguage("rust", storage);
    saveCodeIndent("manual", storage);
    saveCodeLines(20, storage);
    expect([loadPracticeKind(storage), loadCodeLanguage(storage), loadCodeIndent(storage), loadCodeLines(storage)]).toEqual([
      "code",
      "rust",
      "manual",
      20,
    ]);
    saveCodeLines(5, storage);
    expect(loadCodeLines(storage)).toBe(5);
  });

  it("falls back to the defaults for invalid stored values", () => {
    const storage = memoryStorage({
      "plexustyping.practiceKind": "poetry",
      "plexustyping.codeLanguage": "Not An Id!",
      "plexustyping.codeIndent": "tabs",
      "plexustyping.codeLines": "21",
    });
    expect([loadPracticeKind(storage), loadCodeLanguage(storage), loadCodeIndent(storage), loadCodeLines(storage)]).toEqual([
      "prose",
      "python",
      "auto",
      12,
    ]);
    for (const bad of ["4", "12.5", "x", ""]) {
      storage.setItem("plexustyping.codeLines", bad);
      expect(loadCodeLines(storage), bad).toBe(12);
    }
  });
});
