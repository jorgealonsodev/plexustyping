import { defineMessages } from "../../i18n/i18n.svelte";

// Progressive unlock: its settings, the progress line and the unlock notice (ui-i18n S1).
export const t = defineMessages(
  {
    label: "Desbloqueo progresivo",
    hint: "Empieza por la fila base y añade una tecla cada vez que dominas las activas",
    targetLabel: "Velocidad objetivo",
    targetHint: "Velocidad a la que una tecla cuenta como dominada.",
    keys: "Teclas: {active}/{total}",
    next: "{keys} · siguiente: {key}",
    unlocked: "Nueva tecla desbloqueada: {key}",
  },
  {
    en: {
      label: "Progressive unlock",
      hint: "Starts with the home row and adds a key each time you master the active ones",
      targetLabel: "Target speed",
      targetHint: "Speed at which a key counts as mastered.",
      keys: "Keys: {active}/{total}",
      next: "{keys} · next: {key}",
      unlocked: "New key unlocked: {key}",
    },
    pt: {
      label: "Desbloqueio progressivo",
      hint: "Começa pela fileira base e adiciona uma tecla cada vez que você domina as ativas",
      targetLabel: "Velocidade-alvo",
      targetHint: "Velocidade a partir da qual uma tecla conta como dominada.",
      keys: "Teclas: {active}/{total}",
      next: "{keys} · próxima: {key}",
      unlocked: "Nova tecla desbloqueada: {key}",
    },
    fr: {
      label: "Déblocage progressif",
      hint: "Commence par la rangée de repos et ajoute une touche chaque fois que vous maîtrisez les actives",
      targetLabel: "Vitesse cible",
      targetHint: "Vitesse à partir de laquelle une touche est considérée comme maîtrisée.",
      keys: "Touches : {active}/{total}",
      next: "{keys} · suivante : {key}",
      unlocked: "Nouvelle touche débloquée : {key}",
    },
    de: {
      label: "Schrittweise Freischaltung",
      hint: "Beginnt mit der Grundreihe und fügt eine Taste hinzu, sobald du die aktiven beherrschst",
      targetLabel: "Zielgeschwindigkeit",
      targetHint: "Geschwindigkeit, ab der eine Taste als beherrscht gilt.",
      keys: "Tasten: {active}/{total}",
      next: "{keys} · nächste: {key}",
      unlocked: "Neue Taste freigeschaltet: {key}",
    },
  },
);
