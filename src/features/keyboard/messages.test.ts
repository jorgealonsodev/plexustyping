import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { NAMED_LABELS } from "./geometry";
import { HEAT_LEGEND, heatLegendText, heatLegendTitle } from "./heat";
import { t } from "./messages";

afterEach(() => ui.set("es"));

describe("keyboard messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("follow the interface language (S1)", () => {
    ui.set("en");
    expect(heatLegendTitle()).toBe("Errors per key");
    expect(HEAT_LEGEND[0].name).toBe("very few");
    expect(heatLegendText()).toBe("Errors per key: 1 very few, 2 few, 3 some, 4 quite a few, 5 many");
    expect([NAMED_LABELS.Enter, NAMED_LABELS.CapsLock, NAMED_LABELS.ShiftLeft]).toEqual(["Enter", "Caps Lock", "Shift"]);
  });
});
