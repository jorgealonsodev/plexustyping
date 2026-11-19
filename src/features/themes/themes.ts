import { IMPORTED_THEMES } from "./palettes.generated";

/** A terminal-style colour palette: base colours plus the 16 ANSI colours, all `#rrggbb` (or `#rrggbbaa`). */
export interface Palette {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent?: string;
  selectionBackground?: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

/** A colour scheme the app can use; `dark` groups it in the settings. */
export interface ThemeEntry {
  id: string;
  name: string;
  dark: boolean;
  palette: Palette;
  /** A high-contrast theme (a11y S5): its derived colours reach 7:1 for text and 4.5:1 for UI. */
  highContrast?: boolean;
}

/** The PlexusTerm project's own dark palette (same author). */
const PLEXUSTERM: ThemeEntry = {
  id: "plexusterm",
  name: "PlexusTerm",
  dark: true,
  palette: {
    background: "#0f1419",
    foreground: "#d8dee9",
    cursor: "#5ec4ff",
    cursorAccent: "#0f1419",
    selectionBackground: "#2a3b4d",
    black: "#1b2229",
    red: "#ef6b73",
    green: "#8bd49c",
    yellow: "#ebcb8b",
    blue: "#5ec4ff",
    magenta: "#c594c5",
    cyan: "#70e1e8",
    white: "#d8dee9",
    brightBlack: "#4f5b66",
    brightRed: "#ff8a91",
    brightGreen: "#a6e3b4",
    brightYellow: "#f5dba5",
    brightBlue: "#8ad4ff",
    brightMagenta: "#dbb3db",
    brightCyan: "#9aeef2",
    brightWhite: "#ffffff",
  },
};

/** High contrast on black (a11y S5): white text and saturated, light ANSI colours. */
const HIGH_CONTRAST_DARK: ThemeEntry = {
  id: "alto-contraste-oscuro",
  name: "Alto contraste oscuro",
  dark: true,
  highContrast: true,
  palette: {
    background: "#000000",
    foreground: "#ffffff",
    cursor: "#ffff00",
    cursorAccent: "#000000",
    selectionBackground: "#1f3f66",
    black: "#000000",
    red: "#ff7070",
    green: "#5cff5c",
    yellow: "#ffff00",
    blue: "#70b8ff",
    magenta: "#ff8cff",
    cyan: "#00ffff",
    white: "#ffffff",
    brightBlack: "#b0b0b0",
    brightRed: "#ffa0a0",
    brightGreen: "#a0ffa0",
    brightYellow: "#ffffa0",
    brightBlue: "#a8d4ff",
    brightMagenta: "#ffb8ff",
    brightCyan: "#a0ffff",
    brightWhite: "#ffffff",
  },
};

/** High contrast on white (a11y S5): black text and dark, saturated ANSI colours. */
const HIGH_CONTRAST_LIGHT: ThemeEntry = {
  id: "alto-contraste-claro",
  name: "Alto contraste claro",
  dark: false,
  highContrast: true,
  palette: {
    background: "#ffffff",
    foreground: "#000000",
    cursor: "#0000cc",
    cursorAccent: "#ffffff",
    selectionBackground: "#b3d4ff",
    black: "#000000",
    red: "#a00000",
    green: "#005a00",
    yellow: "#5c4400",
    blue: "#0030a0",
    magenta: "#800080",
    cyan: "#00525c",
    white: "#4a4a4a",
    brightBlack: "#3a3a3a",
    brightRed: "#800000",
    brightGreen: "#004400",
    brightYellow: "#4a3700",
    brightBlue: "#002480",
    brightMagenta: "#660066",
    brightCyan: "#004048",
    brightWhite: "#ffffff",
  },
};

/** The same schemes, in the same order, as PlexusTerm, then the two high-contrast ones (a11y S5). */
export const THEMES: ThemeEntry[] = [PLEXUSTERM, ...IMPORTED_THEMES, HIGH_CONTRAST_DARK, HIGH_CONTRAST_LIGHT];

export const DEFAULT_THEME_ID = "catppuccin-mocha";

const byId = new Map(THEMES.map((entry) => [entry.id, entry]));

/** Schemes imported by the user (see `custom.ts`); their ids never collide with the built-in ones. */
let custom: ThemeEntry[] = [];

/** Replaces the imported schemes that `isThemeId` and `themeById` know besides the built-in ones. */
export function setCustomThemes(themes: ThemeEntry[]) {
  custom = themes;
}

const find = (id: string) => byId.get(id) ?? custom.find((entry) => entry.id === id);

export const isThemeId = (id: string) => find(id) !== undefined;

/** The scheme with this id, or the default one for an unknown id. */
export const themeById = (id: string): ThemeEntry => find(id) ?? byId.get(DEFAULT_THEME_ID)!;
