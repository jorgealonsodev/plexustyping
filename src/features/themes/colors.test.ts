import { describe, expect, it } from "vitest";
import { SYNTAX_CLASSES } from "./colors";
import { applyTheme, contrastRatio, ensureContrast, mix, readableOn, themeById, typingColors } from "./index";

const KEYS = [
  "--bg",
  "--surface",
  "--surface-2",
  "--border",
  "--text",
  "--muted",
  "--accent",
  "--on-accent",
  "--success",
  "--warning",
  "--danger",
  "--text-pending",
  "--text-correct",
  "--text-error",
  "--error-bg",
  "--cursor",
  "--cursor-text",
  "--key-bg",
  "--key-fg",
  "--heat-base",
  "--key-border",
  "--key-active",
  "--finger-l-pinky",
  "--finger-l-ring",
  "--finger-l-middle",
  "--finger-l-index",
  "--finger-r-index",
  "--finger-r-middle",
  "--finger-r-ring",
  "--finger-r-pinky",
  "--finger-thumb",
  "--syntax-plain",
  "--syntax-keyword",
  "--syntax-string",
  "--syntax-number",
  "--syntax-comment",
  "--syntax-function",
  "--syntax-type",
  "--syntax-variable",
  "--syntax-constant",
  "--syntax-operator",
  "--syntax-punctuation",
  "--syntax-tag",
  "--syntax-attribute",
  "--syntax-property",
  ...SYNTAX_CLASSES.map((cls) => `--syntax-pending-${cls}`),
  "color-scheme",
];

/** A minimal stand-in for `document.documentElement` (vitest runs in node, without a DOM). */
function fakeRoot() {
  const properties = new Map<string, string>();
  const root = {
    style: { setProperty: (name: string, value: string) => void properties.set(name, value) },
    dataset: {} as Record<string, string>,
  };
  return { root: root as unknown as HTMLElement, properties, dataset: root.dataset };
}

describe("colour helpers", () => {
  it("mixes two colours by weight and ignores an alpha suffix", () => {
    expect(mix("#000000", "#ffffff", 0)).toBe("#000000");
    expect(mix("#000000", "#ffffff", 1)).toBe("#ffffff");
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
    expect(mix("#00000080", "#ff0000", 1)).toBe("#ff0000");
  });

  it("picks black on light backgrounds and white on dark ones", () => {
    expect(readableOn("#ffffff")).toBe("#000000");
    expect(readableOn("#89b4fa")).toBe("#000000");
    expect(readableOn("#1e1e2e")).toBe("#ffffff");
  });

  it("computes the WCAG contrast ratio", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#ffffff", "#000000")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777777", "#777777")).toBe(1);
  });
});

describe("typingColors", () => {
  it("maps Catppuccin Mocha to exactly the typing roles", () => {
    expect(typingColors(themeById("catppuccin-mocha"))).toEqual({
      "--bg": "#1e1e2e",
      "--surface": "#252536",
      "--surface-2": "#2c2d3e",
      "--border": "#3a3b4e",
      "--text": "#cdd6f4",
      "--muted": "#8f94ae",
      "--accent": "#89b4fa",
      "--on-accent": "#000000",
      "--success": "#a6e3a1",
      "--warning": "#f9e2af",
      "--danger": "#f38ba8",
      "--text-pending": "#8f94ad",
      "--text-correct": "#cdd6f4",
      "--text-error": "#f38ba8",
      "--error-bg": "#4d3649",
      "--cursor": "#f5e0dc",
      "--cursor-text": "#1e1e2e",
      "--key-bg": "#2c2d3e",
      "--key-fg": "#cdd6f4",
      "--heat-base": "#ad6a83",
      "--key-border": "#414356",
      "--key-active": "#89b4fa",
      "--finger-l-pinky": "#f5c2e7",
      "--finger-l-ring": "#89b4fa",
      "--finger-l-middle": "#94e2d5",
      "--finger-l-index": "#a6e3a1",
      "--finger-r-index": "#f9e2af",
      "--finger-r-middle": "#b1eae1",
      "--finger-r-ring": "#aeccfc",
      "--finger-r-pinky": "#f398da",
      "--finger-thumb": "#bac2de",
      "--syntax-plain": "#cdd6f4",
      "--syntax-keyword": "#f5c2e7",
      "--syntax-string": "#a6e3a1",
      "--syntax-number": "#f9e2af",
      "--syntax-comment": "#878ca5",
      "--syntax-function": "#89b4fa",
      "--syntax-type": "#94e2d5",
      "--syntax-variable": "#b1eae1",
      "--syntax-constant": "#fcd682",
      "--syntax-operator": "#f398da",
      "--syntax-punctuation": "#a1a8c3",
      "--syntax-tag": "#f38ba8",
      "--syntax-attribute": "#f9e2af",
      "--syntax-property": "#aeccfc",
      "--syntax-pending-plain": "#878ca5",
      "--syntax-pending-keyword": "#9688a3",
      "--syntax-pending-string": "#7b928a",
      "--syntax-pending-number": "#938c87",
      "--syntax-pending-comment": "#888da6",
      "--syntax-pending-function": "#7d8db2",
      "--syntax-pending-type": "#75919c",
      "--syntax-pending-variable": "#7a909b",
      "--syntax-pending-constant": "#978c7d",
      "--syntax-pending-operator": "#9c85a9",
      "--syntax-pending-punctuation": "#868ca4",
      "--syntax-pending-tag": "#9f859e",
      "--syntax-pending-attribute": "#938c87",
      "--syntax-pending-property": "#818dab",
      "color-scheme": "dark",
    });
  });

  it("gives every theme the same keys, derived from its palette", () => {
    for (const entry of [themeById("plexusterm"), themeById("catppuccin-latte"), themeById("laser")]) {
      const colors = typingColors(entry);
      const { background: bg, foreground: fg, red } = entry.palette;
      const text = colors["--text"];
      const layers = [bg, colors["--surface"], colors["--surface-2"]];
      expect(Object.keys(colors).sort()).toEqual([...KEYS].sort());
      // Derived colours keep their mix, corrected only where it falls short of AA (S1).
      expect(text).toBe(ensureContrast(fg, layers, 4.5, fg));
      expect(colors["--text-pending"]).toBe(ensureContrast(mix(bg, fg, 0.45), layers, 4.5, text));
      expect(colors["--error-bg"]).toBe(mix(bg, red, 0.22));
      expect(colors["--accent"]).toBe(ensureContrast(entry.palette.blue, layers, 3, text));
      expect(colors["--key-active"]).toBe(colors["--accent"]);
      expect(colors["color-scheme"]).toBe(entry.dark ? "dark" : "light");
      // Code mode (S3): syntax colours from the ANSI palette, as PlexusTerm colours code.
      expect(colors["--syntax-plain"]).toBe(text);
      expect(colors["--syntax-keyword"]).toBe(entry.palette.magenta);
      expect(colors["--syntax-string"]).toBe(entry.palette.green);
      expect(colors["--syntax-function"]).toBe(entry.palette.blue);
      expect(colors["--syntax-tag"]).toBe(red);
      expect(colors["--syntax-comment"]).toBe(ensureContrast(mix(bg, fg, 0.6), [colors["--surface"]], 4.5, text));
      expect(colors["--syntax-punctuation"]).toBe(mix(bg, fg, 0.75));
    }
  });

  it("falls back to the background for the text under the cursor", () => {
    const entry = themeById("plexusterm");
    const { cursorAccent: _ignored, ...palette } = entry.palette;
    expect(typingColors({ ...entry, palette })["--cursor-text"]).toBe("#0f1419");
  });
});

describe("applyTheme", () => {
  it("writes every typing colour and data-theme on the given root", () => {
    const { root, properties, dataset } = fakeRoot();
    const dark = themeById("catppuccin-mocha");
    applyTheme(dark, root);
    expect(Object.fromEntries(properties)).toEqual(typingColors(dark));
    expect(dataset.theme).toBe("dark");
  });

  it("switches a root to another theme in place", () => {
    const { root, properties, dataset } = fakeRoot();
    applyTheme(themeById("catppuccin-mocha"), root);
    const light = themeById("catppuccin-latte");
    applyTheme(light, root);
    expect(Object.fromEntries(properties)).toEqual(typingColors(light));
    expect(dataset.theme).toBe("light");
  });
});
