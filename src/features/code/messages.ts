import { defineMessages } from "../../i18n/i18n.svelte";

// The practice-mode switch in the header (ui-i18n S1).
export const t = defineMessages(
  { prose: "Prosa", words: "Palabras", code: "Código", language: "Lenguaje de programación", mode: "Modo de práctica" },
  {
    en: { prose: "Prose", words: "Words", code: "Code", language: "Programming language", mode: "Practice mode" },
    pt: { prose: "Prosa", words: "Palavras", code: "Código", language: "Linguagem de programação", mode: "Modo de prática" },
    fr: { prose: "Prose", words: "Mots", code: "Code", language: "Langage de programmation", mode: "Mode d’entraînement" },
    de: { prose: "Prosa", words: "Wörter", code: "Code", language: "Programmiersprache", mode: "Übungsmodus" },
  },
);
