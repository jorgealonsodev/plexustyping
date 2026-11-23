import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { t } from "./messages";
import { TARGET_SETTING, UNLOCK_SETTING, progressLine, unlockNotice } from "./unlock";

afterEach(() => ui.set("es"));

describe("unlock messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("follow the interface language (S1)", () => {
    ui.set("en");
    expect(UNLOCK_SETTING.label).toBe("Progressive unlock");
    expect(TARGET_SETTING.label).toBe("Target speed");
    const status = { enabled: true, active: ["a", "s"], total: 27, next: "e" };
    expect(progressLine(status as never)).toBe("Keys: 2/27 · next: e");
    expect(unlockNotice("e")).toBe("New key unlocked: e");
  });
});
