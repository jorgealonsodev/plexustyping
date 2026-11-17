import { afterEach, describe, expect, it } from "vitest";
import {
  MAX_CUSTOM_THEMES,
  MAX_THEME_FILE_CHARS,
  addCustomTheme,
  loadCustomThemes,
  parseVsCodeTheme,
  saveCustomThemes,
} from "./custom";
import { DEFAULT_THEME_ID, THEMES, isThemeId, setCustomThemes, themeById } from "./themes";

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

const DARK_BASE = THEMES.find((t) => t.dark)!;
const LIGHT_BASE = THEMES.find((t) => !t.dark)!;

const NIGHT = `{
  // A comment with "quotes" and a brace }
  "name": "Night // Owl",
  "type": "dark",
  /* block
     comment */
  "colors": {
    "editor.background": "#011627",
    "editor.foreground": "#D6DEEB",
    "editorCursor.foreground": "#80a4c2",
    "editor.selectionBackground": "#1d3b53cc",
    "terminal.ansiBlack": "#011627",
    "terminal.ansiRed": "#EF5350",
    "terminal.ansiGreen": "#22da6e",
    "terminal.ansiYellow": "#addb67",
    "terminal.ansiBlue": "#82aaff",
    "terminal.ansiMagenta": "#c792ea",
    "terminal.ansiCyan": "#21c7a8",
    "terminal.ansiWhite": "#ffffff",
    "terminal.ansiBrightBlack": "#575656",
    "terminal.ansiBrightRed": "#ef5350",
    "terminal.ansiBrightGreen": "#22da6e",
    "terminal.ansiBrightYellow": "#ffeb95",
    "terminal.ansiBrightBlue": "#82aaff",
    "terminal.ansiBrightMagenta": "#c792ea",
    "terminal.ansiBrightCyan": "#7fdbca",
    "terminal.ansiBrightWhite": "#fff",
  },
  "tokenColors": [],
}`;

const parsed = (text: string, file = "theme.json") => {
  const result = parseVsCodeTheme(text, file);
  if (!result.ok) throw new Error(result.error);
  return result.theme;
};

afterEach(() => setCustomThemes([]));

describe("parseVsCodeTheme", () => {
  it("maps a VS Code theme with comments and trailing commas to a palette", () => {
    expect(parsed(NIGHT)).toEqual({
      id: "custom-night-owl",
      name: "Night // Owl",
      dark: true,
      palette: {
        background: "#011627",
        foreground: "#d6deeb",
        cursor: "#80a4c2",
        cursorAccent: "#011627",
        selectionBackground: "#1d3b53",
        black: "#011627",
        red: "#ef5350",
        green: "#22da6e",
        yellow: "#addb67",
        blue: "#82aaff",
        magenta: "#c792ea",
        cyan: "#21c7a8",
        white: "#ffffff",
        brightBlack: "#575656",
        brightRed: "#ef5350",
        brightGreen: "#22da6e",
        brightYellow: "#ffeb95",
        brightBlue: "#82aaff",
        brightMagenta: "#c792ea",
        brightCyan: "#7fdbca",
        brightWhite: "#ffffff",
      },
    });
  });

  it("fills missing or invalid ANSI colours, the cursor and the name from the base scheme and the file", () => {
    const light = parsed(
      '{"colors": {"editor.background": "#fafafa", "editor.foreground": "#333", "terminal.ansiRed": "red", "terminal.ansiBlue": "#00f8"}}',
      "Mi tema claro.jsonc",
    );
    expect(light.id).toBe("custom-mi-tema-claro");
    expect(light.name).toBe("Mi tema claro");
    expect(light.dark).toBe(false);
    expect(light.palette.foreground).toBe("#333333");
    expect(light.palette.cursor).toBe("#333333");
    expect(light.palette.selectionBackground).toBeUndefined();
    expect(light.palette.red).toBe(LIGHT_BASE.palette.red);
    expect(light.palette.blue).toBe("#0000ff");
    expect(light.palette.green).toBe(LIGHT_BASE.palette.green);

    const dark = parsed('{"name": "  ", "colors": {"editor.background": "#101010", "editor.foreground": "#eeeeee"}}', ".json");
    expect(dark.dark).toBe(true);
    expect(dark.name).toBe("Tema importado");
    expect(dark.id).toBe("custom-tema-importado");
    expect(dark.palette.magenta).toBe(DARK_BASE.palette.magenta);
  });

  it("takes dark or light from the declared type before the background", () => {
    const of = (type: string, bg: string) =>
      parsed(`{"type": "${type}", "colors": {"editor.background": "${bg}", "editor.foreground": "#888888"}}`).dark;
    expect(of("light", "#000000")).toBe(false);
    expect(of("hc-light", "#000000")).toBe(false);
    expect(of("dark", "#ffffff")).toBe(true);
    expect(of("hc-black", "#ffffff")).toBe(true);
    expect(of("other", "#ffffff")).toBe(false);
  });

  it("caps the name at 60 characters", () => {
    const theme = parsed(`{"name": "${"a".repeat(80)}", "colors": {"editor.background": "#000", "editor.foreground": "#fff"}}`);
    expect(theme.name).toHaveLength(60);
  });

  it("rejects files that are too large, not JSON or not a theme", () => {
    const error = (text: string) => {
      const result = parseVsCodeTheme(text, "x.json");
      return result.ok ? null : result.error;
    };
    expect(error(" ".repeat(MAX_THEME_FILE_CHARS + 1))).toBe("El archivo es demasiado grande (máximo 1 MB).");
    expect(error("{nope")).toBe("El archivo no es un JSON válido.");
    expect(error('"/* not a comment"')).toBe("El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.");
    expect(error("[]")).toBe("El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.");
    expect(error('{"colors": {"editor.background": "#000000"}}')).toBe(
      "El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.",
    );
    expect(error('{"colors": {"editor.background": "#00000", "editor.foreground": "#ffffff"}}')).toBe(
      "El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.",
    );
  });
});

describe("custom theme storage", () => {
  it("round-trips imported themes and ignores invalid stored entries", () => {
    const storage = memoryStorage();
    expect(loadCustomThemes(storage)).toEqual([]);
    const night = parsed(NIGHT);
    saveCustomThemes([night], storage);
    expect(loadCustomThemes(storage)).toEqual([night]);

    const broken = [
      night,
      { ...night, id: "catppuccin-mocha" },
      { ...night, id: "custom-x", palette: { ...night.palette, red: "red" } },
      { ...night, id: "custom-y", dark: "yes" },
      { ...night, id: "custom-z", name: "" },
      null,
    ];
    expect(loadCustomThemes(memoryStorage({ "plexustyping.customThemes": JSON.stringify(broken) }))).toEqual([night]);
    expect(loadCustomThemes(memoryStorage({ "plexustyping.customThemes": "{nope" }))).toEqual([]);
    expect(loadCustomThemes(memoryStorage({ "plexustyping.customThemes": '{"a": 1}' }))).toEqual([]);
  });

  it("replaces an import with the same name, appends a new one and refuses more than the maximum", () => {
    const night = parsed(NIGHT);
    const other = { ...night, id: "custom-other", name: "Other" };
    const retuned = { ...night, dark: false };
    expect(addCustomTheme([night, other], retuned)).toEqual({ themes: [retuned, other], theme: retuned });
    expect(addCustomTheme([night], other)).toEqual({ themes: [night, other], theme: other });
    const full = Array.from({ length: MAX_CUSTOM_THEMES }, (_, i) => ({ ...night, id: `custom-${i}`, name: `T${i}` }));
    expect(addCustomTheme(full, other)).toBeNull();
    const again = { ...night, id: "custom-0", name: "T0", dark: false };
    expect(addCustomTheme(full, again)?.themes).toHaveLength(MAX_CUSTOM_THEMES);
    expect(addCustomTheme(full, again)?.themes[0]).toEqual(again);
  });

  it("keeps two names that slug to the same id apart", () => {
    const owl = { ...parsed(NIGHT), id: "custom-night-owl", name: "Night Owl" };
    const dashed = { ...owl, name: "Night-Owl" };
    expect(parsed(NIGHT.replace(/"name": "[^"]*"/, '"name": "Night-Owl"')).id).toBe(owl.id);
    const added = addCustomTheme([owl], dashed)!;
    expect(added.theme).toEqual({ ...dashed, id: `${owl.id}-2` });
    expect(added.themes).toEqual([owl, added.theme]);
    const third = addCustomTheme(added.themes, { ...owl, name: "night owl" })!;
    expect(third.theme.id).toBe(`${owl.id}-3`);
  });
});

describe("custom themes in the registry", () => {
  it("finds registered imports by id and still falls back to the default theme", () => {
    const night = parsed(NIGHT);
    expect(isThemeId(night.id)).toBe(false);
    expect(themeById(night.id).id).toBe(DEFAULT_THEME_ID);
    setCustomThemes([night]);
    expect(isThemeId(night.id)).toBe(true);
    expect(themeById(night.id)).toEqual(night);
    expect(themeById("catppuccin-latte").id).toBe("catppuccin-latte");
    expect(themeById("missing").id).toBe(DEFAULT_THEME_ID);
    setCustomThemes([]);
    expect(isThemeId(night.id)).toBe(false);
  });
});
