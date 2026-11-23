import { defineMessages } from "../../i18n/i18n.svelte";

// Built-in high-contrast theme names and the errors of importing a VS Code theme (ui-i18n S1).
export const t = defineMessages(
  {
    highContrastDark: "Alto contraste oscuro",
    highContrastLight: "Alto contraste claro",
    importedName: "Tema importado",
    tooLarge: "El archivo es demasiado grande (máximo 1 MB).",
    notJson: "El archivo no es un JSON válido.",
    notATheme: "El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground.",
    tooMany: "Hay demasiados temas importados (máximo {max}); elimina alguno.",
  },
  {
    en: {
      highContrastDark: "High contrast dark",
      highContrastLight: "High contrast light",
      importedName: "Imported theme",
      tooLarge: "The file is too large (1 MB at most).",
      notJson: "The file is not valid JSON.",
      notATheme: "The file does not look like a VS Code theme: editor.background and editor.foreground are missing.",
      tooMany: "There are too many imported themes ({max} at most); remove one.",
    },
    pt: {
      highContrastDark: "Alto contraste escuro",
      highContrastLight: "Alto contraste claro",
      importedName: "Tema importado",
      tooLarge: "O arquivo é grande demais (máximo de 1 MB).",
      notJson: "O arquivo não é um JSON válido.",
      notATheme: "O arquivo não parece um tema do VS Code: faltam editor.background e editor.foreground.",
      tooMany: "Há temas importados demais (máximo de {max}); remova algum.",
    },
    fr: {
      highContrastDark: "Contraste élevé sombre",
      highContrastLight: "Contraste élevé clair",
      importedName: "Thème importé",
      tooLarge: "Le fichier est trop volumineux (1 Mo maximum).",
      notJson: "Le fichier n’est pas un JSON valide.",
      notATheme:
        "Le fichier ne ressemble pas à un thème VS Code : editor.background et editor.foreground manquent.",
      tooMany: "Il y a trop de thèmes importés ({max} maximum) ; supprimez-en un.",
    },
    de: {
      highContrastDark: "Hoher Kontrast dunkel",
      highContrastLight: "Hoher Kontrast hell",
      importedName: "Importiertes Design",
      tooLarge: "Die Datei ist zu groß (höchstens 1 MB).",
      notJson: "Die Datei ist kein gültiges JSON.",
      notATheme: "Die Datei sieht nicht wie ein VS-Code-Design aus: editor.background und editor.foreground fehlen.",
      tooMany: "Es gibt zu viele importierte Designs (höchstens {max}); entferne eines.",
    },
  },
);
