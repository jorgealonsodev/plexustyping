import { settingsStorage } from "./persist";
import { t } from "./textMessages";

/** Ajustes → Apariencia select of the practice text's typeface, prose and code (practice-font S1). */
export const FONT_SETTING = {
  get label() {
    return t("fontLabel");
  },
  get hint() {
    return t("fontHint");
  },
};

/** Group ids in display order; `fontGroupLabel` gives the shown name. */
export const FONT_GROUPS = ["Mecanografía", "Legibilidad"] as const;
export type FontGroup = (typeof FONT_GROUPS)[number];

/** A font group as the select shows it, in the interface language. */
export const fontGroupLabel = (g: FontGroup) => t(g === "Mecanografía" ? "fontGroupTyping" : "fontGroupLegibility");

export interface PracticeFont {
  id: string;
  /** The family's own name; the default font's is translated. */
  readonly name: string;
  /** CSS `font-family` stack: the bundled family first, then today's monospace stack. */
  family: string;
  group: FontGroup;
  /** In the interface language. */
  readonly hint: string;
  /** Turns the font's ligatures off: each typed char stays its own glyph. */
  noLigatures?: boolean;
}

export const DEFAULT_FONT_ID = "default";

const mono = (name: string) => `"${name}", var(--mono)`;

/** Display order; the first is today's font (the `--mono` stack), the default (S1). Files bundled by fontFiles.ts (S2). */
export const PRACTICE_FONTS: PracticeFont[] = [
  {
    id: DEFAULT_FONT_ID,
    get name() {
      return t("fontDefault");
    },
    family: "var(--mono)",
    group: "Mecanografía",
    get hint() {
      return t("fontHintDefault");
    },
  },
  {
    id: "roboto-mono",
    name: "Roboto Mono",
    family: mono("Roboto Mono"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintRobotoMono");
    },
  },
  {
    id: "jetbrains-mono",
    name: "JetBrains Mono",
    family: mono("JetBrains Mono"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintJetBrainsMono");
    },
  },
  {
    id: "fira-code",
    name: "Fira Code",
    family: mono("Fira Code"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintFiraCode");
    },
    noLigatures: true,
  },
  {
    id: "source-code-pro",
    name: "Source Code Pro",
    family: mono("Source Code Pro"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintSourceCodePro");
    },
  },
  {
    id: "ibm-plex-mono",
    name: "IBM Plex Mono",
    family: mono("IBM Plex Mono"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintIbmPlexMono");
    },
  },
  {
    id: "inconsolata",
    name: "Inconsolata",
    family: mono("Inconsolata"),
    group: "Mecanografía",
    get hint() {
      return t("fontHintInconsolata");
    },
  },
  {
    id: "atkinson-hyperlegible-mono",
    name: "Atkinson Hyperlegible Mono",
    family: mono("Atkinson Hyperlegible Mono"),
    group: "Legibilidad",
    get hint() {
      return t("fontHintAtkinson");
    },
  },
  {
    id: "intel-one-mono",
    name: "Intel One Mono",
    family: mono("Intel One Mono"),
    group: "Legibilidad",
    get hint() {
      return t("fontHintIntelOneMono");
    },
  },
  {
    id: "lexend-deca",
    name: "Lexend Deca",
    family: '"Lexend Deca", system-ui, sans-serif',
    group: "Legibilidad",
    get hint() {
      return t("fontHintLexendDeca");
    },
  },
  {
    id: "opendyslexic",
    name: "OpenDyslexic",
    family: '"OpenDyslexic", sans-serif',
    group: "Legibilidad",
    get hint() {
      return t("fontHintOpenDyslexic");
    },
  },
];

/** The font with `id`; the default for an unknown id (S1). */
export const fontById = (id: string): PracticeFont =>
  PRACTICE_FONTS.find((f) => f.id === id) ?? PRACTICE_FONTS[0];

const FONT_KEY = "plexustyping.practiceFont";

/** The stored font id; the default when missing or unknown (S1). */
export const loadPracticeFont = (storage: Storage = settingsStorage()) => fontById(storage.getItem(FONT_KEY) ?? "").id;

export const savePracticeFont = (id: string, storage: Storage = settingsStorage()) => storage.setItem(FONT_KEY, id);

/** The CSS variables the prose and code practice text read. */
export const fontStyle = (f: PracticeFont) =>
  `--practice-font: ${f.family}; --practice-features: ${f.noLigatures ? '"liga" 0, "calt" 0' : "normal"}`;
