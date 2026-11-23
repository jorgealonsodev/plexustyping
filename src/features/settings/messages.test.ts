import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { aiToggleTitle, chatgptAccount, densityLabel } from "./ai";
import { t as aiT } from "./aiMessages";
import { CURSOR_STYLES } from "./cursor";
import { PRACTICE_FONTS, fontGroupLabel } from "./fonts";
import { t } from "./messages";
import { SECTIONS } from "./registry";
import { matchCountText } from "./sections";
import { ERROR_MODES, GHOST_SETTING, TYPOGRAPHY_FIELDS, lengthLabel, typographyValue, UNLIMITED_LENGTH } from "./settings";
import { t as textT } from "./textMessages";

afterEach(() => ui.set("es"));

describe("settings messages (ui-i18n S5)", () => {
  it("are complete in every language", () => {
    expectComplete(t);
    expectComplete(textT);
    expectComplete(aiT);
  });
});

describe("settings texts follow the interface language (ui-i18n S1)", () => {
  it("names the sections and the module-level settings in English", () => {
    ui.set("en");
    expect(SECTIONS.map((s) => s.label)).toEqual([
      "Practice",
      "Text",
      "Keyboard",
      "Metronome",
      "Code",
      "Appearance",
      "Accessibility",
      "AI",
      "Profiles",
    ]);
    expect(ERROR_MODES.map((m) => m.label)).toEqual(["Allow continuing", "Stop until corrected", "Erase with Backspace"]);
    expect(GHOST_SETTING.label).toBe("Ghost session");
    expect(TYPOGRAPHY_FIELDS.map((f) => f.label)).toEqual(["Font size", "Letter spacing", "Line height"]);
    expect(lengthLabel(UNLIMITED_LENGTH)).toBe("Unlimited");
    expect(CURSOR_STYLES[0].name).toBe("Bar");
    expect(PRACTICE_FONTS[0].name).toBe("Default");
    expect(fontGroupLabel(PRACTICE_FONTS[0].group)).toBe("Typing");
  });

  it("counts the search matches with the language's plural", () => {
    expect([0, 1, 5].map(matchCountText)).toEqual(["Ningún ajuste coincide", "1 ajuste", "5 ajustes"]);
    ui.set("en");
    expect([0, 1, 5].map(matchCountText)).toEqual(["No setting matches", "1 setting", "5 settings"]);
  });

  it("writes the typography values with the language's decimal separator", () => {
    expect(typographyValue("letterSpacing", 0.12)).toBe("0,12 em");
    expect(typographyValue("fontScale", 150)).toBe("150 %");
    ui.set("en");
    expect(typographyValue("letterSpacing", 0.12)).toBe("0.12 em");
    expect(typographyValue("lineHeight", 1.75)).toBe("1.75");
    expect(typographyValue("fontScale", 150)).toBe("150%");
  });

  it("writes the AI density and texts in English", () => {
    ui.set("en");
    expect(densityLabel(2.5)).toBe("2.5×");
    expect(aiToggleTitle(true)).toBe("AI on: new texts are generated with AI");
    expect(chatgptAccount({ signedIn: true, email: "ana@example.com", planEnabled: true }).account).toBe(
      "Signed in as ana@example.com",
    );
  });
});
