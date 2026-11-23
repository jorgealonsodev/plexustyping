import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { parseVsCodeTheme, tooLargeError, tooManyError } from "./custom";
import { t } from "./messages";
import { THEMES } from "./themes";

afterEach(() => ui.set("es"));

describe("theme messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("follow the interface language (S1)", () => {
    ui.set("en");
    expect(THEMES.find((entry) => entry.id === "alto-contraste-oscuro")?.name).toBe("High contrast dark");
    expect(tooLargeError()).toBe("The file is too large (1 MB at most).");
    expect(tooManyError()).toBe("There are too many imported themes (30 at most); remove one.");
    expect(parseVsCodeTheme("{", "x.json")).toEqual({ ok: false, error: "The file is not valid JSON." });
  });
});
