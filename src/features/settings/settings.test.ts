import { describe, expect, it } from "vitest";
import {
  CODE_INDENTS,
  DEFAULT_LANGUAGE,
  DEFAULT_LAYOUT,
  ERROR_MODES,
  LANGUAGES,
  LAYOUTS,
  loadLanguage,
  loadLayout,
  saveLanguage,
  saveLayout,
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
  DEFAULT_TYPOGRAPHY,
  SCREEN_READER_SETTING,
  COLOR_BLIND_SETTING,
  loadColorBlind,
  saveColorBlind,
  TYPOGRAPHY_FIELDS,
  checkTypography,
  loadScreenReader,
  loadTypography,
  saveScreenReader,
  saveTypography,
  typographyStyle,
  GHOST_SETTING,
  loadErrorMode,
  loadFlashes,
  loadGhost,
  loadHeatmap,
  loadLiveStats,
  loadMetronome,
  loadShowKeyboard,
  loadTargetCount,
  loadTextLength,
  saveErrorMode,
  saveFlashes,
  saveGhost,
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

  it("races the ghost session by default and persists the toggle (ghost S3)", () => {
    const storage = memoryStorage();
    expect(loadGhost(storage)).toBe(true);
    saveGhost(false, storage);
    expect(storage.getItem("plexustyping.ghost")).toBe("false");
    expect(loadGhost(storage)).toBe(false);
    saveGhost(true, storage);
    expect(storage.getItem("plexustyping.ghost")).toBe("true");
    expect(loadGhost(storage)).toBe(true);
    expect([loadLiveStats(storage), loadFlashes(storage), loadShowKeyboard(storage)]).toEqual([true, true, true]);
  });

  it("names the ghost setting of Ajustes → Práctica (ghost S3)", () => {
    expect(GHOST_SETTING).toEqual({
      label: "Sesión fantasma",
      hint: "Corre contra tu mejor sesión con el mismo texto",
    });
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

  it("offers 2 or 3 targets, 2 by default, and persists the choice", () => {
    expect(TARGET_COUNTS).toEqual([2, 3]);
    const storage = memoryStorage();
    expect(loadTargetCount(storage)).toBe(2);
    saveTargetCount(3, storage);
    expect(storage.getItem("plexustyping.targetCount")).toBe("3");
    expect(loadTargetCount(storage)).toBe(3);
    // Earlier choices of 4 or 5 fall back to the default.
    for (const bad of ["1", "4", "5", "2.5", "x"])
      expect(loadTargetCount(memoryStorage({ "plexustyping.targetCount": bad }))).toBe(2);
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
    saveTargetCount(3, storage);
    saveTextLength(100, storage);
    expect([loadLiveStats(storage), loadFlashes(storage), loadShowKeyboard(storage)]).toEqual([true, true, true]);
    expect([loadHeatmap(storage), loadTargetCount(storage), loadTextLength(storage)]).toEqual([true, 3, 100]);
  });

  it("keeps the metronome off at 60 BPM by default and persists every field", () => {
    const storage = memoryStorage();
    expect(loadMetronome(storage)).toEqual({ enabled: false, bpm: 60, sound: "click", volume: 80, accentEvery: 4, adaptive: false });
    const next = { enabled: true, bpm: 300, sound: "beep" as const, volume: 0, accentEvery: 16, adaptive: true };
    saveMetronome(next, storage);
    expect(loadMetronome(storage)).toEqual(next);
    saveMetronome({ ...next, bpm: 30, accentEvery: 0, volume: 100, sound: "wood" }, storage);
    expect(loadMetronome(storage)).toEqual({ ...next, bpm: 30, accentEvery: 0, volume: 100, sound: "wood" });
  });

  it("replaces an invalid stored metronome field by its default, keeping the others", () => {
    const load = (value: unknown) => loadMetronome(memoryStorage({ "plexustyping.metronome": JSON.stringify(value) }));
    expect(load({ enabled: true, bpm: 29, sound: "gong", volume: 101, accentEvery: 17, adaptive: "yes" })).toEqual({
      enabled: true,
      bpm: 60,
      sound: "click",
      volume: 80,
      accentEvery: 4,
      adaptive: false,
    });
    expect(load({ bpm: 120.5, volume: -1, accentEvery: 2, adaptive: true })).toEqual({
      enabled: false,
      bpm: 60,
      sound: "click",
      volume: 80,
      accentEvery: 2,
      adaptive: true,
    });
    expect(loadMetronome(memoryStorage({ "plexustyping.metronome": "{nope" }))).toEqual(load({}));
    const current = { enabled: true, bpm: 90, sound: "wood" as const, volume: 30, accentEvery: 3, adaptive: true };
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

  it("stores the words kind like the others", () => {
    const storage = memoryStorage();
    savePracticeKind("words", storage);
    expect(storage.getItem("plexustyping.practiceKind")).toBe("words");
    expect(loadPracticeKind(storage)).toBe("words");
    savePracticeKind("prose", storage);
    expect(loadPracticeKind(storage)).toBe("prose");
    for (const bad of ["Words", "palabras", ""]) {
      storage.setItem("plexustyping.practiceKind", bad);
      expect(loadPracticeKind(storage), bad).toBe("prose");
    }
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

describe("keyboard layout setting", () => {
  it("lists the bundled layout files with their names, Spain first", () => {
    const files = import.meta.glob<string>("../../../layouts/*.toml", { query: "?raw", import: "default", eager: true });
    const bundled = Object.entries(files).map(([path, toml]) => ({
      id: path.replace(/^.*\/|\.toml$/g, ""),
      label: /^name = "(.*)"$/m.exec(toml)?.[1],
    }));
    expect(LAYOUTS).toEqual([
      { id: "es-qwerty", label: "Español (España)" },
      { id: "latam-qwerty", label: "Español (Latinoamérica)" },
      { id: "us-qwerty", label: "English (US)" },
      { id: "pt-abnt2", label: "Português (Brasil)" },
      { id: "fr-azerty", label: "Français (France)" },
      { id: "de-qwertz", label: "Deutsch (Deutschland)" },
      { id: "us-dvorak", label: "English (Dvorak)" },
      { id: "us-colemak", label: "English (Colemak)" },
    ]);
    expect([...LAYOUTS].sort((a, b) => a.id.localeCompare(b.id))).toEqual(bundled.sort((a, b) => a.id.localeCompare(b.id)));
    expect(DEFAULT_LAYOUT).toBe("es-qwerty");
  });

  it("defaults to the Spanish layout, ignores unknown stored ids and persists the choice", () => {
    const storage = memoryStorage();
    expect(loadLayout(storage)).toBe("es-qwerty");
    expect(loadLayout(memoryStorage({ "plexustyping.layout": "dvorak" }))).toBe("es-qwerty");
    saveLayout("us-qwerty", storage);
    expect(storage.getItem("plexustyping.layout")).toBe("us-qwerty");
    expect(loadLayout(storage)).toBe("us-qwerty");
  });
});

describe("language setting", () => {
  it("lists the five prose languages, Spanish first and default", () => {
    expect(LANGUAGES).toEqual([
      { id: "es", label: "Español" },
      { id: "en", label: "English" },
      { id: "pt", label: "Português (Brasil)" },
      { id: "fr", label: "Français" },
      { id: "de", label: "Deutsch" },
    ]);
    expect(DEFAULT_LANGUAGE).toBe("es");
  });

  it("defaults to Spanish, ignores unknown stored ids and persists the choice", () => {
    const storage = memoryStorage();
    expect(loadLanguage(storage)).toBe("es");
    expect(loadLanguage(memoryStorage({ "plexustyping.language": "xx" }))).toBe("es");
    saveLanguage("fr", storage);
    expect(storage.getItem("plexustyping.language")).toBe("fr");
    expect(loadLanguage(storage)).toBe("fr");
  });
});

describe("screen reader setting (a11y S3)", () => {
  it("names the setting in Spanish", () => {
    expect(SCREEN_READER_SETTING.label).toBe("Anunciar con lector de pantalla");
  });

  it("is off by default and persists the toggle, apart from the other toggles", () => {
    const storage = memoryStorage();
    expect(loadScreenReader(storage)).toBe(false);
    expect(loadScreenReader(memoryStorage({ "plexustyping.screenReader": "yes" }))).toBe(false);
    saveScreenReader(true, storage);
    expect(storage.getItem("plexustyping.screenReader")).toBe("true");
    expect(loadScreenReader(storage)).toBe(true);
    expect([loadLiveStats(storage), loadFlashes(storage), loadHeatmap(storage)]).toEqual([true, true, false]);
    saveScreenReader(false, storage);
    expect(loadScreenReader(storage)).toBe(false);
  });
});

describe("colour-blind keyboard setting (a11y S5)", () => {
  it("names the setting in Spanish", () => {
    expect(COLOR_BLIND_SETTING.label).toBe("Modo daltónico");
  });

  it("is off by default and persists the toggle, apart from the other keyboard toggles", () => {
    const storage = memoryStorage();
    expect(loadColorBlind(storage)).toBe(false);
    expect(loadColorBlind(memoryStorage({ "plexustyping.colorBlind": "yes" }))).toBe(false);
    saveColorBlind(true, storage);
    expect(storage.getItem("plexustyping.colorBlind")).toBe("true");
    expect(loadColorBlind(storage)).toBe(true);
    expect([loadFlashes(storage), loadHeatmap(storage), loadShowKeyboard(storage)]).toEqual([true, false, true]);
    saveColorBlind(false, storage);
    expect(storage.getItem("plexustyping.colorBlind")).toBe("false");
    expect(loadColorBlind(storage)).toBe(false);
  });
});

describe("typography settings (a11y S4)", () => {
  it("names the three settings with their ranges, steps and defaults", () => {
    expect(TYPOGRAPHY_FIELDS).toEqual([
      { key: "fontScale", label: "Tamaño de letra", min: 80, max: 200, step: 10 },
      { key: "letterSpacing", label: "Espaciado entre letras", min: 0, max: 0.3, step: 0.02 },
      { key: "lineHeight", label: "Interlineado", min: 1.2, max: 2.4, step: 0.1 },
    ]);
    expect(DEFAULT_TYPOGRAPHY).toEqual({ fontScale: 100, letterSpacing: 0, lineHeight: 1.75 });
  });

  it("defaults to 100 %, 0 em and the current 1.75 line height", () => {
    expect(loadTypography(memoryStorage())).toEqual({ fontScale: 100, letterSpacing: 0, lineHeight: 1.75 });
    expect(loadTypography(memoryStorage({ "plexustyping.typography": "{nope" }))).toEqual(DEFAULT_TYPOGRAPHY);
  });

  it("survives a reload through the storage helpers", () => {
    const storage = memoryStorage();
    const next = { fontScale: 150, letterSpacing: 0.12, lineHeight: 2.1 };
    saveTypography(next, storage);
    expect(loadTypography(storage)).toEqual(next);
    saveTypography({ fontScale: 80, letterSpacing: 0.3, lineHeight: 1.2 }, storage);
    expect(loadTypography(storage)).toEqual({ fontScale: 80, letterSpacing: 0.3, lineHeight: 1.2 });
  });

  it("clamps a stored value outside its range, and replaces a missing or non-numeric one by its default", () => {
    const load = (value: unknown) => loadTypography(memoryStorage({ "plexustyping.typography": JSON.stringify(value) }));
    expect(load({ fontScale: 300, letterSpacing: -1, lineHeight: 9 })).toEqual({ fontScale: 200, letterSpacing: 0, lineHeight: 2.4 });
    expect(load({ fontScale: 10, letterSpacing: 0.5, lineHeight: 0.5 })).toEqual({ fontScale: 80, letterSpacing: 0.3, lineHeight: 1.2 });
    expect(load({ fontScale: "big", lineHeight: null })).toEqual(DEFAULT_TYPOGRAPHY);
    expect(load({ letterSpacing: 0.1 })).toEqual({ ...DEFAULT_TYPOGRAPHY, letterSpacing: 0.1 });
  });

  it("clamps a changed value and keeps the current one for an invalid one", () => {
    const current = { fontScale: 120, letterSpacing: 0.04, lineHeight: 2 };
    expect(checkTypography({ ...current, fontScale: 250 }, current)).toEqual({ ...current, fontScale: 200 });
    expect(checkTypography({ ...current, lineHeight: Number.NaN }, current)).toEqual(current);
    expect(checkTypography({ ...current, letterSpacing: 0.1 + 0.2 }, current)).toEqual({ ...current, letterSpacing: 0.3 });
  });

  it("turns the values into the practice text's CSS variables", () => {
    expect(typographyStyle(DEFAULT_TYPOGRAPHY)).toBe("--text-scale: 1; --text-spacing: 0em; --text-leading: 1.75");
    expect(typographyStyle({ fontScale: 150, letterSpacing: 0.12, lineHeight: 2.1 })).toBe(
      "--text-scale: 1.5; --text-spacing: 0.12em; --text-leading: 2.1",
    );
  });
});
