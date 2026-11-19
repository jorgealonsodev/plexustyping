import { describe, expect, it } from "vitest";
import { contrastRatio, ensureContrast, mix, parseVsCodeTheme, THEMES, themeById, typingColors } from "./index";
import type { ThemeEntry } from "./index";
import { SYNTAX_CLASSES } from "./colors";

// WCAG AA (S1): text pairs need 4.5:1, UI component pairs 3:1.
const TEXT = 4.5;
const UI = 3;

const FINGERS = [
  "--finger-l-pinky",
  "--finger-l-ring",
  "--finger-l-middle",
  "--finger-l-index",
  "--finger-r-index",
  "--finger-r-middle",
  "--finger-r-ring",
  "--finger-r-pinky",
  "--finger-thumb",
];

const SYNTAX = SYNTAX_CLASSES;

/** Heatmap tints of `Keyboard.svelte`: `color-mix(in srgb, var(--heat-base) N%, var(--key-bg))`, N = 6…60. */
const HEAT = [0.06, 0.12, 0.18, 0.24, 0.3, 0.36, 0.42, 0.48, 0.54, 0.6];

/** Opacity of a highlighted modifier key in `Keyboard.svelte`, laid over the page background. */
const MODIFIER_OPACITY = 0.85;

type Pair = { label: string; fg: string; bg: string; min: number };

/** Every S1 pair of a theme, with the colours exactly as the app derives and layers them. */
function pairs(entry: ThemeEntry): Pair[] {
  const c = typingColors(entry);
  const out: Pair[] = [];
  const add = (fgVar: string, fg: string, bgVar: string, bg: string, min: number) =>
    out.push({ label: `${fgVar} on ${bgVar}`, fg, bg, min });
  const v = (name: string) => c[name];

  // Main, pending and muted text on the three background layers.
  for (const text of ["--text", "--text-pending", "--muted"])
    for (const layer of ["--bg", "--surface", "--surface-2"]) add(text, v(text), layer, v(layer), TEXT);
  // Error text on its tint.
  add("--text-error", v("--text-error"), "--error-bg", v("--error-bg"), TEXT);
  // Code mode: plain text and comments, typed and pending, on the code block (`--surface`); every pending class.
  add("--syntax-plain", v("--syntax-plain"), "--surface", v("--surface"), TEXT);
  add("--syntax-comment", v("--syntax-comment"), "--surface", v("--surface"), TEXT);
  for (const cls of SYNTAX) {
    const name = `--syntax-pending-${cls}`;
    add(name, v(name), "--surface", v("--surface"), TEXT);
  }
  // Key labels: highlighted (primary, modifier at its opacity, secondary), flashed and heatmap keys.
  for (const finger of FINGERS) {
    add("--bg", v("--bg"), finger, v(finger), TEXT);
    add("--bg", v("--bg"), `${finger} @${MODIFIER_OPACITY}`, mix(v("--bg"), v(finger), MODIFIER_OPACITY), TEXT);
    add(finger, v(finger), "--key-bg", v("--key-bg"), TEXT);
  }
  add("--bg", v("--bg"), "--success", v("--success"), TEXT);
  add("--bg", v("--bg"), "--danger", v("--danger"), TEXT);
  add("--key-fg", v("--key-fg"), "--key-bg", v("--key-bg"), TEXT);
  for (const heat of HEAT)
    add("--key-fg", v("--key-fg"), `heat ${heat}`, mix(v("--key-bg"), v("--heat-base"), heat), TEXT);
  // UI: the focus outline and the accent on --bg; the active tab and toggle against the inactive layers.
  for (const layer of ["--bg", "--surface", "--surface-2"]) add("--accent", v("--accent"), layer, v(layer), UI);
  add("--on-accent", v("--on-accent"), "--accent", v("--accent"), TEXT);
  return out;
}

/** The pairs short of their minimum; `high` raises text pairs to 7:1 and UI pairs to 4.5:1 (a11y S5). */
const failures = (entry: ThemeEntry, high = false) =>
  pairs(entry)
    .map((p) => ({ ...p, min: high ? (p.min === TEXT ? HIGH_TEXT : HIGH_UI) : p.min }))
    .map((p) => ({ ...p, ratio: contrastRatio(p.fg, p.bg) }))
    .filter((p) => !(p.ratio >= p.min))
    .map((p) => `${p.label}: ${p.fg} / ${p.bg} = ${p.ratio.toFixed(2)} < ${p.min}`);

// High contrast (a11y S5): text pairs need 7:1, UI pairs 4.5:1.
const HIGH_TEXT = 7;
const HIGH_UI = 4.5;

describe("WCAG AA contrast of every built-in theme (S1)", () => {
  it.each(THEMES.map((entry) => [entry.id, entry] as const))("%s meets AA on every S1 pair", (_id, entry) => {
    expect(failures(entry)).toEqual([]);
  });
});

describe("high-contrast themes (a11y S5)", () => {
  const high = ["alto-contraste-oscuro", "alto-contraste-claro"];

  it("ships both as built-in themes", () => {
    expect(high.map((id) => THEMES.find((t) => t.id === id)?.name)).toEqual([
      "Alto contraste oscuro",
      "Alto contraste claro",
    ]);
  });

  it.each(high)("%s reaches 7:1 on every text pair and 4.5:1 on every UI pair", (id) => {
    const entry = THEMES.find((t) => t.id === id)!;
    expect(entry.highContrast).toBe(true);
    expect(failures(entry, true)).toEqual([]);
  });
});

describe("ensureContrast", () => {
  it("returns a colour that already passes byte-identical", () => {
    expect(ensureContrast("#cdd6f4", ["#1e1e2e"], TEXT, "#ffffff")).toBe("#cdd6f4");
  });

  it("moves a failing colour toward the target just until it passes", () => {
    const out = ensureContrast("#6d7187", ["#1e1e2e", "#2c2d3e"], TEXT, "#cdd6f4");
    expect(out).not.toBe("#6d7187");
    expect(contrastRatio(out, "#1e1e2e")).toBeGreaterThanOrEqual(TEXT);
    expect(contrastRatio(out, "#2c2d3e")).toBeGreaterThanOrEqual(TEXT);
    // Still short of the target: only as far as needed.
    expect(contrastRatio(out, "#2c2d3e")).toBeLessThan(contrastRatio("#cdd6f4", "#2c2d3e"));
  });

  it("goes past an insufficient target toward black or white", () => {
    // Solarized Light: its foreground is 4.13:1 on its background.
    const out = ensureContrast("#657b83", ["#fdf6e3"], TEXT, "#657b83");
    expect(contrastRatio(out, "#fdf6e3")).toBeGreaterThanOrEqual(TEXT);
  });
});

describe("visual identity", () => {
  it("keeps the colours of Catppuccin Mocha that already pass", () => {
    const c = typingColors(themeById("catppuccin-mocha"));
    expect(c["--text"]).toBe("#cdd6f4");
    expect(c["--success"]).toBe("#a6e3a1");
    expect(c["--danger"]).toBe("#f38ba8");
    expect(c["--error-bg"]).toBe("#4d3649");
    expect(c["--key-fg"]).toBe("#cdd6f4");
    expect(c["--accent"]).toBe("#89b4fa");
    expect(c["--on-accent"]).toBe("#000000");
    expect(c["--syntax-plain"]).toBe("#cdd6f4");
    expect(c["--finger-l-ring"]).toBe("#89b4fa");
  });

  it("corrects only the failing Mocha pending text, toward the foreground", () => {
    const c = typingColors(themeById("catppuccin-mocha"));
    expect(c["--text-pending"]).not.toBe("#6d7187");
    expect(contrastRatio(c["--text-pending"], c["--surface-2"])).toBeGreaterThanOrEqual(TEXT);
  });

  it("weakens the heatmap base only where the key labels need it", () => {
    // Mocha: --danger is too strong at the heaviest tints, so its heat base moves toward --key-bg.
    const mocha = typingColors(themeById("catppuccin-mocha"));
    expect(mocha["--heat-base"]).not.toBe(mocha["--danger"]);
    // A theme whose tints already read keeps --danger as its heat base.
    const passing = THEMES.map(typingColors).find((c) =>
      HEAT.every((w) => contrastRatio(c["--key-fg"], mix(c["--key-bg"], c["--danger"], w)) >= TEXT),
    )!;
    expect(passing["--heat-base"]).toBe(passing["--danger"]);
  });

  it("gives an imported VS Code theme the same correction", () => {
    // A low-contrast light theme: grey text and pale ANSI colours on off-white.
    const file = JSON.stringify({
      name: "Pale",
      type: "light",
      colors: {
        "editor.background": "#f4f1ea",
        "editor.foreground": "#8a8a8a",
        "terminal.ansiRed": "#f0a0a0",
        "terminal.ansiGreen": "#a0e0a0",
        "terminal.ansiBlue": "#a0c0f0",
      },
    });
    const parsed = parseVsCodeTheme(file, "pale.json");
    if (!parsed.ok) throw new Error(parsed.error);
    expect(failures(parsed.theme)).toEqual([]);
  });
});
