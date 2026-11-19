import { describe, expect, it } from "vitest";
import { SECTIONS } from "./registry";
import { isSearchShortcut, isSettingsShortcut, matchesSettingQuery, moveSection } from "./sections";

const key = (k: string, mods: Partial<Record<"ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat", boolean>> = {}) => ({
  key: k,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
  repeat: false,
  ...mods,
});

describe("settings sections", () => {
  it("lists the reorganised sections in order, each with a unique id and a component", () => {
    // "Organiza bien el menu de ajustes": what changes the practice text has its own section, and
    // the accessibility aids sit together.
    expect(SECTIONS.map((s) => s.label)).toEqual([
      "Práctica",
      "Texto",
      "Teclado",
      "Metrónomo",
      "Código",
      "Apariencia",
      "Accesibilidad",
      "IA",
      "Perfiles",
    ]);
    expect(new Set(SECTIONS.map((s) => s.id)).size).toBe(SECTIONS.length);
    for (const s of SECTIONS) expect(s.component).toBeTypeOf("function");
  });

  it("moves through the vertical tab list with the arrows, wrapping, and jumps with Home/End", () => {
    const ids = ["a", "b", "c"];
    expect(moveSection(ids, "a", "ArrowDown")).toBe("b");
    expect(moveSection(ids, "c", "ArrowDown")).toBe("a");
    expect(moveSection(ids, "a", "ArrowUp")).toBe("c");
    expect(moveSection(ids, "b", "ArrowUp")).toBe("a");
    expect(moveSection(ids, "b", "Home")).toBe("a");
    expect(moveSection(ids, "a", "End")).toBe("c");
  });

  it("ignores other keys and an unknown current section", () => {
    expect(moveSection(["a", "b"], "a", "ArrowRight")).toBeNull();
    expect(moveSection(["a", "b"], "a", "Enter")).toBeNull();
    expect(moveSection(["a", "b"], "z", "ArrowDown")).toBeNull();
    expect(moveSection([], "a", "Home")).toBeNull();
  });
});

describe("settings shortcut", () => {
  it("opens with Ctrl+,", () => {
    expect(isSettingsShortcut(key(",", { ctrlKey: true }))).toBe(true);
  });

  it("ignores a bare comma, other modifiers, other keys and key repeat", () => {
    expect(isSettingsShortcut(key(","))).toBe(false);
    expect(isSettingsShortcut(key(",", { ctrlKey: true, altKey: true }))).toBe(false);
    expect(isSettingsShortcut(key(",", { ctrlKey: true, metaKey: true }))).toBe(false);
    expect(isSettingsShortcut(key(",", { ctrlKey: true, shiftKey: true }))).toBe(false);
    expect(isSettingsShortcut(key(".", { ctrlKey: true }))).toBe(false);
    expect(isSettingsShortcut(key(",", { ctrlKey: true, repeat: true }))).toBe(false);
  });
});

describe("settings search", () => {
  // "añade un buscador rapido"
  it("matches every word of the query, ignoring case and accents", () => {
    const row = "Interlineado Texto de práctica: 1,75 (de 1,2 a 2,4).";
    expect(matchesSettingQuery(row, "interlineado")).toBe(true);
    expect(matchesSettingQuery(row, "PRACTICA")).toBe(true);
    expect(matchesSettingQuery(row, "texto  inter")).toBe(true);
    expect(matchesSettingQuery(row, "texto tema")).toBe(false);
    expect(matchesSettingQuery("Modo daltónico", "daltonico")).toBe(true);
    expect(matchesSettingQuery("Tamaño de letra", "tamano")).toBe(true);
  });

  it("focuses the search with plain Ctrl+F only", () => {
    expect(isSearchShortcut(key("f", { ctrlKey: true }))).toBe(true);
    expect(isSearchShortcut(key("F", { ctrlKey: true }))).toBe(true);
    expect(isSearchShortcut(key("f"))).toBe(false);
    expect(isSearchShortcut(key("f", { ctrlKey: true, shiftKey: true }))).toBe(false);
    expect(isSearchShortcut(key("f", { ctrlKey: true, altKey: true }))).toBe(false);
  });

  it("matches everything for an empty or blank query", () => {
    expect(matchesSettingQuery("Cursor", "")).toBe(true);
    expect(matchesSettingQuery("Cursor", "   ")).toBe(true);
  });
});
