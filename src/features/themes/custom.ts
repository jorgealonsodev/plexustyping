// Themes imported by the user from VS Code colour theme files, kept with the UI settings (settingsStorage).

import { settingsStorage } from "../settings/persist";
import { readableOn } from "./colors";
import { type Palette, type ThemeEntry, THEMES, setCustomThemes } from "./themes";

const KEY = "plexustyping.customThemes";
const ID_PREFIX = "custom-";
const MAX_NAME_CHARS = 60;
const FALLBACK_NAME = "Tema importado";

export const MAX_CUSTOM_THEMES = 30;
/** About 1 MB of text; real theme files are a few tens of KB. */
export const MAX_THEME_FILE_CHARS = 1_000_000;

export const TOO_LARGE = "El archivo es demasiado grande (máximo 1 MB).";
const NOT_JSON = "El archivo no es un JSON válido.";
const NOT_A_THEME = "El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.";
export const TOO_MANY = `Hay demasiados temas importados (máximo ${MAX_CUSTOM_THEMES}); elimina alguno.`;

/** Palette colour → the VS Code workbench colour it is read from. */
const ANSI = {
  black: "terminal.ansiBlack",
  red: "terminal.ansiRed",
  green: "terminal.ansiGreen",
  yellow: "terminal.ansiYellow",
  blue: "terminal.ansiBlue",
  magenta: "terminal.ansiMagenta",
  cyan: "terminal.ansiCyan",
  white: "terminal.ansiWhite",
  brightBlack: "terminal.ansiBrightBlack",
  brightRed: "terminal.ansiBrightRed",
  brightGreen: "terminal.ansiBrightGreen",
  brightYellow: "terminal.ansiBrightYellow",
  brightBlue: "terminal.ansiBrightBlue",
  brightMagenta: "terminal.ansiBrightMagenta",
  brightCyan: "terminal.ansiBrightCyan",
  brightWhite: "terminal.ansiBrightWhite",
} as const satisfies Partial<Record<keyof Palette, string>>;

type AnsiName = keyof typeof ANSI;
const ANSI_NAMES = Object.keys(ANSI) as AnsiName[];

/**
 * JSON with comments (`//`, `/* *\/`) and trailing commas, as VS Code writes themes, made plain
 * JSON. Comment markers and commas inside strings are kept.
 */
export function stripJsonComments(text: string): string {
  const out: string[] = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === '"') {
      const start = i++;
      while (i < text.length && text[i] !== '"') i += text[i] === "\\" ? 2 : 1;
      out.push(text.slice(start, ++i));
    } else if (c === "/" && text[i + 1] === "/") {
      while (i < text.length && text[i] !== "\n") i++;
    } else if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      i = end === -1 ? text.length : end + 2;
    } else {
      if (c === "}" || c === "]") {
        // A comma followed only by whitespace before the closing bracket is a trailing comma.
        let last = out.length - 1;
        while (last >= 0 && out[last].trim() === "") last--;
        if (out[last] === ",") out[last] = "";
      }
      out.push(c);
      i++;
    }
  }
  return out.join("");
}

/** `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` as lowercase `#rrggbb` (alpha dropped); `null` for anything else. */
function hexColor(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const digits = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(value.trim())?.[1].toLowerCase();
  if (!digits) return null;
  return `#${digits.length <= 4 ? [...digits.slice(0, 3)].map((d) => d + d).join("") : digits.slice(0, 6)}`;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const themeName = (declared: unknown, fileName: string) => {
  const fromFile = fileName.replace(/\.[^.]*$/, "");
  const name = (typeof declared === "string" && declared.trim()) || fromFile.trim() || FALLBACK_NAME;
  return name.slice(0, MAX_NAME_CHARS).trim();
};

const themeId = (name: string) => {
  const slug = name
    .normalize("NFD")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return ID_PREFIX + (slug || "tema");
};

const isDark = (type: unknown, background: string) => {
  if (type === "light" || type === "hc-light") return false;
  if (type === "dark" || type === "hc" || type === "hc-black") return true;
  return readableOn(background) === "#ffffff";
};

export type ParsedTheme = { ok: true; theme: ThemeEntry } | { ok: false; error: string };

/**
 * The app scheme for the text of a VS Code colour theme file: editor colours become the base
 * colours and `terminal.ansi*` the ANSI palette; each ANSI colour the file lacks comes from the
 * built-in base scheme of the same kind (dark or light).
 */
export function parseVsCodeTheme(text: string, fileName: string): ParsedTheme {
  if (text.length > MAX_THEME_FILE_CHARS) return { ok: false, error: TOO_LARGE };
  let file: unknown;
  try {
    file = JSON.parse(stripJsonComments(text));
  } catch {
    return { ok: false, error: NOT_JSON };
  }
  const colors = isRecord(file) && isRecord(file.colors) ? file.colors : {};
  const background = hexColor(colors["editor.background"]);
  const foreground = hexColor(colors["editor.foreground"]);
  if (!isRecord(file) || !background || !foreground) return { ok: false, error: NOT_A_THEME };

  const dark = isDark(file.type, background);
  const base = THEMES.find((t) => t.dark === dark)!.palette;
  const ansi = Object.fromEntries(ANSI_NAMES.map((name) => [name, hexColor(colors[ANSI[name]]) ?? base[name]]));
  const selection = hexColor(colors["editor.selectionBackground"]);
  const name = themeName(file.name, fileName);
  return {
    ok: true,
    theme: {
      id: themeId(name),
      name,
      dark,
      palette: {
        background,
        foreground,
        cursor: hexColor(colors["editorCursor.foreground"]) ?? foreground,
        cursorAccent: background,
        ...(selection ? { selectionBackground: selection } : {}),
        ...(ansi as Record<AnsiName, string>),
      },
    },
  };
}

const HEX = /^#[0-9a-f]{6}$/;
const isHex = (value: unknown) => typeof value === "string" && HEX.test(value);
const REQUIRED: (keyof Palette)[] = ["background", "foreground", "cursor", ...ANSI_NAMES];
const OPTIONAL: (keyof Palette)[] = ["cursorAccent", "selectionBackground"];

/** Whether a stored value is an imported theme this module could have written. */
function isCustomTheme(value: unknown): value is ThemeEntry {
  if (!isRecord(value) || !isRecord(value.palette)) return false;
  const palette = value.palette;
  return (
    typeof value.id === "string" &&
    value.id.startsWith(ID_PREFIX) &&
    typeof value.name === "string" &&
    value.name.trim() !== "" &&
    typeof value.dark === "boolean" &&
    REQUIRED.every((key) => isHex(palette[key])) &&
    OPTIONAL.every((key) => palette[key] === undefined || isHex(palette[key]))
  );
}

/** The stored imported themes; invalid entries (and a corrupt value) are ignored. */
export function loadCustomThemes(storage: Storage = settingsStorage()): ThemeEntry[] {
  try {
    const stored: unknown = JSON.parse(storage.getItem(KEY) ?? "[]");
    return Array.isArray(stored) ? stored.filter(isCustomTheme).slice(0, MAX_CUSTOM_THEMES) : [];
  } catch {
    return [];
  }
}

export const saveCustomThemes = (themes: ThemeEntry[], storage: Storage = settingsStorage()) =>
  storage.setItem(KEY, JSON.stringify(themes));

/**
 * `themes` with `theme` replacing the import of the same name, or appended, and the theme as
 * stored: a different name whose id is taken gets the next free `-2`, `-3`… id.
 * `null` when the list is full.
 */
export function addCustomTheme(
  themes: ThemeEntry[],
  theme: ThemeEntry,
): { themes: ThemeEntry[]; theme: ThemeEntry } | null {
  const same = themes.find((t) => t.name === theme.name);
  if (same) {
    const stored = { ...theme, id: same.id };
    return { themes: themes.map((t) => (t === same ? stored : t)), theme: stored };
  }
  if (themes.length >= MAX_CUSTOM_THEMES) return null;
  const taken = new Set(themes.map((t) => t.id));
  let id = theme.id;
  for (let n = 2; taken.has(id); n++) id = `${theme.id}-${n}`;
  const stored = { ...theme, id };
  return { themes: [...themes, stored], theme: stored };
}

/** Loads the stored imports and makes them known to `themeById`; call before reading the saved theme id. */
export function initCustomThemes(storage: Storage = settingsStorage()): ThemeEntry[] {
  const themes = loadCustomThemes(storage);
  setCustomThemes(themes);
  return themes;
}
