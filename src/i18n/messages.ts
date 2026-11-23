import { defineMessages } from "./i18n.svelte";

/** The interface-language setting itself (ui-i18n S4). */
export const t = defineMessages(
  {
    label: "Idioma de la interfaz",
    hint: "Idioma de menús, avisos y ajustes; el del texto de práctica se elige aparte.",
    system: "Automático (idioma del sistema: {name})",
  },
  {
    en: {
      label: "Interface language",
      hint: "Language of menus, notices and settings; the practice-text language is chosen separately.",
      system: "Automatic (system language: {name})",
    },
    pt: {
      label: "Idioma da interface",
      hint: "Idioma dos menus, avisos e configurações; o do texto de prática é escolhido à parte.",
      system: "Automático (idioma do sistema: {name})",
    },
    fr: {
      label: "Langue de l’interface",
      hint: "Langue des menus, avis et réglages ; celle du texte d’entraînement se choisit à part.",
      system: "Automatique (langue du système : {name})",
    },
    de: {
      label: "Sprache der Oberfläche",
      hint: "Sprache von Menüs, Hinweisen und Einstellungen; die des Übungstexts wird getrennt gewählt.",
      system: "Automatisch (Systemsprache: {name})",
    },
  },
);
