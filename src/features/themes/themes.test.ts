import { describe, expect, it } from "vitest";
import { DEFAULT_THEME_ID, THEMES, isThemeId, themeById } from "./index";

const ANSI = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "brightBlack",
  "brightRed",
  "brightGreen",
  "brightYellow",
  "brightBlue",
  "brightMagenta",
  "brightCyan",
  "brightWhite",
] as const;

describe("themes", () => {
  it("ships PlexusTerm's 57 schemes, each with a unique id", () => {
    const names = THEMES.map((t) => t.name);
    expect(THEMES).toHaveLength(57);
    expect(new Set(THEMES.map((t) => t.id)).size).toBe(57);
    for (const name of ["PlexusTerm", "Catppuccin Mocha", "Catppuccin Frappé", "Rosé Pine Dawn", "Laser", "3024 Day"]) {
      expect(names).toContain(name);
    }
  });

  it("lists the PlexusTerm palette first, then the imported schemes in PlexusTerm's order", () => {
    expect(THEMES[0]).toMatchObject({ id: "plexusterm", name: "PlexusTerm", dark: true });
    expect(THEMES[0].palette.background).toBe("#0f1419");
    expect(THEMES[1].id).toBe("catppuccin-mocha");
    expect(THEMES[56].id).toBe("laser");
  });

  it("leaves out the paid schemes", () => {
    const names = THEMES.map((t) => t.name.toLowerCase());
    expect(names.filter((n) => n.includes("pro"))).toEqual([]);
  });

  it("gives every palette a background, a foreground, a cursor and the 16 ANSI colours as hex", () => {
    const hex = /^#[0-9a-f]{6}([0-9a-f]{2})?$/;
    for (const { id, palette } of THEMES) {
      for (const key of ["background", "foreground", "cursor", "cursorAccent", ...ANSI] as const) {
        expect(palette[key], `${id}.${key}`).toMatch(hex);
      }
      if (palette.selectionBackground !== undefined) {
        expect(palette.selectionBackground, `${id}.selectionBackground`).toMatch(hex);
      }
    }
  });

  it("marks light and dark themes by their background", () => {
    expect(themeById("catppuccin-latte").dark).toBe(false);
    expect(themeById("solarized-light").dark).toBe(false);
    expect(themeById("catppuccin-mocha").dark).toBe(true);
    expect(themeById("plexusterm").dark).toBe(true);
  });

  it("uses Catppuccin Mocha by default and for unknown ids", () => {
    expect(DEFAULT_THEME_ID).toBe("catppuccin-mocha");
    expect(isThemeId(DEFAULT_THEME_ID)).toBe(true);
    expect(isThemeId("no-such-theme")).toBe(false);
    expect(themeById("no-such-theme").id).toBe("catppuccin-mocha");
    expect(themeById("catppuccin-mocha").palette.background).toBe("#1e1e2e");
  });
});
