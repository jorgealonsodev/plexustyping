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

/** AA minimum contrast for text (WCAG 1.4.3). */
export const AA_TEXT = 4.5;
/** AA minimum contrast for UI components and focus indicators (WCAG 1.4.11). */
export const AA_UI = 3;
/** Minimum contrast of the high-contrast themes (a11y S5): text (WCAG AAA 1.4.6) and UI components. */
export const HIGH_TEXT = 7;
export const HIGH_UI = 4.5;

const BLACK = "#000000";
const WHITE = "#ffffff";

/** Lowest contrast of `color` against any of `backgrounds`. */
const minContrast = (color: string, backgrounds: string[]) =>
  Math.min(...backgrounds.map((background) => contrastRatio(color, background)));

/** Steps of each leg of the correction path; small enough that a corrected colour stays close to the threshold. */
const STEPS = 100;

/**
 * `color` unchanged when `passes` it; otherwise the first colour that passes on the way from `color`
 * to `toward` (the theme foreground), and then on from `toward` to black or white, whichever reads
 * better on `backgrounds`. Black or white when even that is not enough.
 */
export function correctColor(color: string, passes: (candidate: string) => boolean, toward: string, backgrounds: string[]): string {
  if (passes(color)) return color;
  const extreme = minContrast(BLACK, backgrounds) >= minContrast(WHITE, backgrounds) ? BLACK : WHITE;
  for (const [from, to] of [[color, toward], [toward, extreme]]) {
    for (let step = 1; step <= STEPS; step++) {
      const candidate = mix(from, to, step / STEPS);
      if (passes(candidate)) return candidate;
    }
  }
  return extreme;
}

/**
 * `color` when it has at least `min` contrast on every one of `backgrounds` (byte-identical);
 * otherwise moved toward `toward`, then toward black or white, just until it does.
 */
export function ensureContrast(color: string, backgrounds: string[], min: number, toward: string): string {
  return correctColor(color, (candidate) => minContrast(candidate, backgrounds) >= min, toward, backgrounds);
}

/** Black or white for text on `background`: `readableOn` unless it falls short of AA, then the better one. */
const textOn = (background: string, min = AA_TEXT) => {
  const pick = readableOn(background);
  if (contrastRatio(pick, background) >= min) return pick;
  return contrastRatio(BLACK, background) >= contrastRatio(WHITE, background) ? BLACK : WHITE;
};

/** Highlight classes of code mode; each gets `--syntax-<class>` and its pending shade `--syntax-pending-<class>`. */
export const SYNTAX_CLASSES = [
  "plain",
  "keyword",
  "string",
  "number",
  "comment",
  "function",
  "type",
  "variable",
  "constant",
  "operator",
  "punctuation",
  "tag",
  "attribute",
  "property",
] as const;

/** Opacity of a highlighted modifier key on the on-screen keyboard (`Keyboard.svelte`). */
export const MODIFIER_KEY_OPACITY = 0.85;
/** Heatmap tints of the on-screen keyboard: `--heat-base` mixed into `--key-bg` by these weights (`Keyboard.svelte`). */
export const HEAT_TINTS = [0.06, 0.12, 0.18, 0.24, 0.3, 0.36, 0.42, 0.48, 0.54, 0.6];

/**
 * CSS custom properties of the app for a colour scheme, mapped to the typing roles
 * (background, pending text, correct text, error, cursor, keyboard, code syntax). Panels and borders
 * are layered from the background towards the text; status colours come from the ANSI palette.
 * Derived colours that fall short of WCAG AA on the layers they are drawn on (S1) are pushed toward
 * the foreground until they pass; the ones that already pass are kept as they are.
 */
export function typingColors({ dark, palette, highContrast = false }: ThemeEntry): Record<string, string> {
  // High-contrast themes (a11y S5) hold every pair to 7:1 for text and 4.5:1 for UI instead of AA.
  const minText = highContrast ? HIGH_TEXT : AA_TEXT;
  const minUi = highContrast ? HIGH_UI : AA_UI;
  const bg = palette.background;
  const fg = palette.foreground;
  const surface = mix(bg, fg, 0.04);
  const surface2 = mix(bg, fg, 0.08);
  const keyBg = surface2;
  const layers = [bg, surface, surface2];
  // The text colour: the foreground, or toward black or white when even it is short of AA.
  const text = ensureContrast(fg, layers, minText, fg);
  const onLayers = (color: string, min = minText) => ensureContrast(color, layers, min, text);
  const accent = onLayers(palette.blue, minUi);
  const errorBg = mix(bg, palette.red, 0.22);
  // Flashed keys show their label in --bg on these.
  const success = ensureContrast(palette.green, [bg], minText, text);
  const danger = ensureContrast(palette.red, [bg], minText, text);
  const keyFg = ensureContrast(fg, [keyBg], minText, text);
  // Heatmap tint base: --danger, weakened toward the key only as far as the labels need to read on every tint.
  const heatBase = correctColor(
    danger,
    (candidate) => HEAT_TINTS.every((weight) => contrastRatio(keyFg, mix(keyBg, candidate, weight)) >= minText),
    keyBg,
    [keyBg],
  );
  // A highlighted key: its label in --bg on the finger colour (a modifier at its opacity), or the
  // finger colour as the label on the key.
  const finger = (color: string) =>
    correctColor(
      color,
      (candidate) =>
        contrastRatio(candidate, bg) >= minText &&
        contrastRatio(bg, mix(bg, candidate, MODIFIER_KEY_OPACITY)) >= minText &&
        contrastRatio(candidate, keyBg) >= minText,
      text,
      [bg, keyBg],
    );
  const syntax: Record<(typeof SYNTAX_CLASSES)[number], string> = {
    plain: text,
    keyword: palette.magenta,
    string: palette.green,
    number: palette.yellow,
    comment: ensureContrast(mix(bg, fg, 0.6), [surface], minText, text),
    function: palette.blue,
    type: palette.cyan,
    variable: palette.brightCyan,
    constant: palette.brightYellow,
    operator: palette.brightMagenta,
    punctuation: mix(bg, fg, 0.75),
    tag: palette.red,
    attribute: palette.yellow,
    property: palette.brightBlue,
  };
  const colors: Record<string, string> = {
    "--bg": bg,
    "--surface": surface,
    "--surface-2": surface2,
    "--border": mix(bg, fg, 0.16),
    "--text": text,
    "--muted": onLayers(mix(bg, fg, 0.6)),
    "--accent": accent,
    "--on-accent": textOn(accent, minText),
    "--success": success,
    "--warning": palette.yellow,
    "--danger": danger,
    // Practice text: still to type, typed right, typed wrong.
    "--text-pending": onLayers(mix(bg, fg, 0.45)),
    "--text-correct": text,
    "--text-error": ensureContrast(palette.red, [errorBg], minText, text),
    "--error-bg": errorBg,
    "--cursor": palette.cursor,
    "--cursor-text": palette.cursorAccent ?? bg,
    // On-screen keyboard; the heatmap tints keys from --heat-base, flashes keep --danger.
    "--key-bg": keyBg,
    "--key-fg": keyFg,
    "--heat-base": heatBase,
    "--key-border": mix(bg, fg, 0.2),
    "--key-active": accent,
    // Touch-typing fingers, coloured on the on-screen keyboard (S11).
    "--finger-l-pinky": finger(palette.magenta),
    "--finger-l-ring": finger(palette.blue),
    "--finger-l-middle": finger(palette.cyan),
    "--finger-l-index": finger(palette.green),
    "--finger-r-index": finger(palette.yellow),
    "--finger-r-middle": finger(palette.brightCyan),
    "--finger-r-ring": finger(palette.brightBlue),
    "--finger-r-pinky": finger(palette.brightMagenta),
    "--finger-thumb": finger(dark ? palette.white : palette.black),
  };
  // Code mode (S3): one colour per highlight class, from the ANSI palette as PlexusTerm colours code;
  // still to type, the class colour dimmed toward the code block (`--surface`), kept readable on it.
  for (const cls of SYNTAX_CLASSES) colors[`--syntax-${cls}`] = syntax[cls];
  for (const cls of SYNTAX_CLASSES)
    colors[`--syntax-pending-${cls}`] = ensureContrast(mix(surface, syntax[cls], 0.45), [surface], minText, text);
  colors["color-scheme"] = dark ? "dark" : "light";
  return colors;
}

/** Applies a scheme's colours to `root` (the whole document by default); safe to call again to switch live. */
export function applyTheme(entry: ThemeEntry, root: HTMLElement = document.documentElement) {
  for (const [name, value] of Object.entries(typingColors(entry))) root.style.setProperty(name, value);
  root.dataset.theme = entry.dark ? "dark" : "light";
}
