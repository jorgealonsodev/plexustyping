import type { ThemeEntry } from "./themes";

const channels = (hex: string) => [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));

const toHex = (values: number[]) => `#${values.map((v) => Math.round(v).toString(16).padStart(2, "0")).join("")}`;

/** `weight` of the way from `from` to `to` (both #rrggbb, an alpha suffix is ignored). */
export function mix(from: string, to: string, weight: number): string {
  const a = channels(from);
  const b = channels(to);
  return toHex(a.map((v, i) => v + (b[i] - v) * weight));
}

const luminance = (hex: string) => {
  const [r, g, b] = channels(hex).map((v) => v / 255);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

/** Black or white, whichever reads better on `background`. */
export const readableOn = (background: string) => (luminance(background) > 0.4 ? "#000000" : "#ffffff");

/** WCAG relative luminance of a #rrggbb colour. */
const relativeLuminance = (hex: string) => {
  const [r, g, b] = channels(hex).map((v) => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

/** WCAG contrast ratio of two #rrggbb colours, from 1 to 21. */
export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/**
 * CSS custom properties of the app for a colour scheme, mapped to the typing roles
 * (background, pending text, correct text, error, cursor, keyboard, code syntax). Panels and borders
 * are layered from the background towards the text; status colours come from the ANSI palette.
 */
export function typingColors({ dark, palette }: ThemeEntry): Record<string, string> {
  const bg = palette.background;
  const fg = palette.foreground;
  const accent = palette.blue;
  return {
    "--bg": bg,
    "--surface": mix(bg, fg, 0.04),
    "--surface-2": mix(bg, fg, 0.08),
    "--border": mix(bg, fg, 0.16),
    "--text": fg,
    "--muted": mix(bg, fg, 0.6),
    "--accent": accent,
    "--on-accent": readableOn(accent),
    "--success": palette.green,
    "--warning": palette.yellow,
    "--danger": palette.red,
    // Practice text: still to type, typed right, typed wrong.
    "--text-pending": mix(bg, fg, 0.45),
    "--text-correct": fg,
    "--text-error": palette.red,
    "--error-bg": mix(bg, palette.red, 0.22),
    "--cursor": palette.cursor,
    "--cursor-text": palette.cursorAccent ?? bg,
    // On-screen keyboard.
    "--key-bg": mix(bg, fg, 0.08),
    "--key-fg": fg,
    "--key-border": mix(bg, fg, 0.2),
    "--key-active": accent,
    // Touch-typing fingers, coloured on the on-screen keyboard (S11).
    "--finger-l-pinky": palette.magenta,
    "--finger-l-ring": palette.blue,
    "--finger-l-middle": palette.cyan,
    "--finger-l-index": palette.green,
    "--finger-r-index": palette.yellow,
    "--finger-r-middle": palette.brightCyan,
    "--finger-r-ring": palette.brightBlue,
    "--finger-r-pinky": palette.brightMagenta,
    "--finger-thumb": dark ? palette.white : palette.black,
    // Code mode (S3): one colour per highlight class, from the ANSI palette as PlexusTerm colours code.
    "--syntax-plain": fg,
    "--syntax-keyword": palette.magenta,
    "--syntax-string": palette.green,
    "--syntax-number": palette.yellow,
    "--syntax-comment": mix(bg, fg, 0.6),
    "--syntax-function": palette.blue,
    "--syntax-type": palette.cyan,
    "--syntax-variable": palette.brightCyan,
    "--syntax-constant": palette.brightYellow,
    "--syntax-operator": palette.brightMagenta,
    "--syntax-punctuation": mix(bg, fg, 0.75),
    "--syntax-tag": palette.red,
    "--syntax-attribute": palette.yellow,
    "--syntax-property": palette.brightBlue,
    "color-scheme": dark ? "dark" : "light",
  };
}

/** Applies a scheme's colours to `root` (the whole document by default); safe to call again to switch live. */
export function applyTheme(entry: ThemeEntry, root: HTMLElement = document.documentElement) {
  for (const [name, value] of Object.entries(typingColors(entry))) root.style.setProperty(name, value);
  root.dataset.theme = entry.dark ? "dark" : "light";
}
