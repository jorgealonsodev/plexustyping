import { defineMessages, plural } from "../../i18n/i18n.svelte";

const n = (p: Record<string, string | number>) => Number(p.n);

// The practice-mode switch and the length picker in the header (ui-i18n S1).
export const t = defineMessages(
  {
    prose: "Prosa",
    words: "Palabras",
    code: "Código",
    language: "Lenguaje de programación",
    mode: "Modo de práctica",
    length: "Longitud del texto",
    codeLength: "Líneas máximas del código",
    chars: "{n} caracteres",
    lines: (p) => plural(n(p), { one: `${p.shown} línea`, other: `${p.shown} líneas` }),
  },
  {
    en: {
      prose: "Prose",
      words: "Words",
      code: "Code",
      language: "Programming language",
      mode: "Practice mode",
      length: "Text length",
      codeLength: "Maximum code lines",
      chars: "{n} characters",
      lines: (p) => plural(n(p), { one: `${p.shown} line`, other: `${p.shown} lines` }),
    },
    pt: {
      prose: "Prosa",
      words: "Palavras",
      code: "Código",
      language: "Linguagem de programação",
      mode: "Modo de prática",
      length: "Tamanho do texto",
      codeLength: "Máximo de linhas de código",
      chars: "{n} caracteres",
      lines: (p) => plural(n(p), { one: `${p.shown} linha`, other: `${p.shown} linhas` }),
    },
    fr: {
      prose: "Prose",
      words: "Mots",
      code: "Code",
      language: "Langage de programmation",
      mode: "Mode d’entraînement",
      length: "Longueur du texte",
      codeLength: "Lignes de code maximum",
      chars: "{n} caractères",
      lines: (p) => plural(n(p), { one: `${p.shown} ligne`, other: `${p.shown} lignes` }),
    },
    de: {
      prose: "Prosa",
      words: "Wörter",
      code: "Code",
      language: "Programmiersprache",
      mode: "Übungsmodus",
      length: "Textlänge",
      codeLength: "Maximale Codezeilen",
      chars: "{n} Zeichen",
      lines: (p) => plural(n(p), { one: `${p.shown} Zeile`, other: `${p.shown} Zeilen` }),
    },
  },
);
