import { describe, expect, it } from "vitest";
import {
  DEFAULT_FONT_ID,
  FONT_GROUPS,
  FONT_SETTING,
  PRACTICE_FONTS,
  fontById,
  fontStyle,
  loadPracticeFont,
  savePracticeFont,
} from "./fonts";

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
    setItem: (key, value) => void items.set(key, value),
  };
}

describe("practice font list (practice-font S1, S2)", () => {
  it("is labelled Fuente in Ajustes → Apariencia", () => {
    expect(FONT_SETTING.label).toBe("Fuente");
  });

  it("lists the default, the typing staples and the legibility fonts, in order", () => {
    expect(PRACTICE_FONTS.map((f) => [f.id, f.name, f.group])).toEqual([
      ["default", "Predeterminada", "Mecanografía"],
      ["roboto-mono", "Roboto Mono", "Mecanografía"],
      ["jetbrains-mono", "JetBrains Mono", "Mecanografía"],
      ["fira-code", "Fira Code", "Mecanografía"],
      ["source-code-pro", "Source Code Pro", "Mecanografía"],
      ["ibm-plex-mono", "IBM Plex Mono", "Mecanografía"],
      ["inconsolata", "Inconsolata", "Mecanografía"],
      ["atkinson-hyperlegible-mono", "Atkinson Hyperlegible Mono", "Legibilidad"],
      ["intel-one-mono", "Intel One Mono", "Legibilidad"],
      ["lexend-deca", "Lexend Deca", "Legibilidad"],
      ["opendyslexic", "OpenDyslexic", "Legibilidad"],
    ]);
    expect(FONT_GROUPS).toEqual(["Mecanografía", "Legibilidad"]);
    expect(new Set(PRACTICE_FONTS.map((f) => f.id)).size).toBe(PRACTICE_FONTS.length);
    for (const f of PRACTICE_FONTS) expect(f.hint.length).toBeGreaterThan(0);
  });

  it("defaults to today's practice font, the --mono stack", () => {
    expect(DEFAULT_FONT_ID).toBe("default");
    expect(fontById(DEFAULT_FONT_ID).family).toBe("var(--mono)");
  });

  it("names each bundled family first, with a fallback", () => {
    for (const f of PRACTICE_FONTS.slice(1)) expect(f.family).toMatch(new RegExp(`^"${f.name}", `));
  });

  it("falls back to the default for an unknown id", () => {
    expect(fontById("comic-sans").id).toBe("default");
    expect(fontById("").id).toBe("default");
  });

  it("does not claim a benefit for OpenDyslexic (L2)", () => {
    expect(fontById("opendyslexic").hint).not.toMatch(/mejora|ayuda|dislexia/i);
  });

  it("sets the practice font variables; Fira Code turns its ligatures off", () => {
    expect(fontStyle(fontById("default"))).toBe("--practice-font: var(--mono); --practice-features: normal");
    expect(fontStyle(fontById("fira-code"))).toBe(
      '--practice-font: "Fira Code", var(--mono); --practice-features: "liga" 0, "calt" 0',
    );
  });
});

describe("practice font persistence (practice-font S1)", () => {
  it("loads the default when nothing or an unknown id is stored", () => {
    expect(loadPracticeFont(memoryStorage())).toBe("default");
    expect(loadPracticeFont(memoryStorage({ "plexustyping.practiceFont": "nope" }))).toBe("default");
  });

  it("round-trips a chosen font", () => {
    const storage = memoryStorage();
    savePracticeFont("lexend-deca", storage);
    expect(storage.getItem("plexustyping.practiceFont")).toBe("lexend-deca");
    expect(loadPracticeFont(storage)).toBe("lexend-deca");
  });
});

describe("bundled font files (practice-font S2)", () => {
  const source = Object.values(
    import.meta.glob<string>("./fontFiles.ts", { query: "?raw", import: "default", eager: true }),
  )[0];
  const imports = [...source.matchAll(/^import "([^"]+)";$/gm)].map((m) => m[1]);

  it("imports only latin and latin-ext at 400 and 700 for every bundled font", () => {
    for (const f of PRACTICE_FONTS.slice(1)) {
      expect(imports).toContain(`@fontsource/${f.id}/latin-400.css`);
      expect(imports).toContain(`@fontsource/${f.id}/latin-700.css`);
    }
    for (const i of imports) expect(i).toMatch(/^@fontsource\/[a-z-]+\/latin(-ext)?-(400|700)\.css$/);
    expect(source).not.toMatch(/https?:/);
  });
});
