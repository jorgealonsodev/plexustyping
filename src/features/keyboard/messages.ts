import { defineMessages } from "../../i18n/i18n.svelte";

// Key caps and the heatmap legend of the keyboard (a11y S5, ui-i18n S1).
export const t = defineMessages(
  {
    heatTitle: "Errores por tecla",
    heat1: "muy pocos",
    heat2: "pocos",
    heat3: "algunos",
    heat4: "bastantes",
    heat5: "muchos",
    // Key caps that carry a word.
    enter: "Intro",
    capsLock: "Bloq Mayús",
    shift: "Mayús",
  },
  {
    en: { heatTitle: "Errors per key", heat1: "very few", heat2: "few", heat3: "some", heat4: "quite a few", heat5: "many", enter: "Enter", capsLock: "Caps Lock", shift: "Shift" },
    pt: { heatTitle: "Erros por tecla", heat1: "muito poucos", heat2: "poucos", heat3: "alguns", heat4: "bastantes", heat5: "muitos", enter: "Enter", capsLock: "Caps Lock", shift: "Shift" },
    fr: { heatTitle: "Erreurs par touche", heat1: "très peu", heat2: "peu", heat3: "quelques-unes", heat4: "pas mal", heat5: "beaucoup", enter: "Entrée", capsLock: "Verr Maj", shift: "Maj" },
    de: { heatTitle: "Fehler pro Taste", heat1: "sehr wenige", heat2: "wenige", heat3: "einige", heat4: "ziemlich viele", heat5: "viele", enter: "Enter", capsLock: "Feststell", shift: "Umschalt" },
  },
);
