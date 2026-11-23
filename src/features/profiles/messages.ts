import { defineMessages } from "../../i18n/i18n.svelte";

/** Profile texts (ui-i18n S1). */
export const t = defineMessages(
  {
    deleteConfirmation: "Se borrarán el historial y las estadísticas de «{name}». ¿Continuar?",
  },
  {
    en: { deleteConfirmation: "The history and statistics of “{name}” will be deleted. Continue?" },
    pt: { deleteConfirmation: "O histórico e as estatísticas de “{name}” serão apagados. Continuar?" },
    fr: { deleteConfirmation: "L’historique et les statistiques de « {name} » seront supprimés. Continuer ?" },
    de: { deleteConfirmation: "Verlauf und Statistiken von „{name}“ werden gelöscht. Fortfahren?" },
  },
);
