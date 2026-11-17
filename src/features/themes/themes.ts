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

/** The same schemes, in the same order, as PlexusTerm. */
export const THEMES: ThemeEntry[] = [PLEXUSTERM, ...IMPORTED_THEMES];

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
