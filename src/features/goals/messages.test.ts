import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { GOAL_KINDS, goalHeaderText, goalMetNotice } from "./goals";
import { t } from "./messages";

afterEach(() => ui.set("es"));

describe("goal messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("follow the interface language (S1)", () => {
    ui.set("en");
    expect(GOAL_KINDS.map((k) => k.label)).toEqual(["Off", "Minutes a day", "Texts a day"]);
    expect(goalMetNotice()).toBe("Daily goal met!");
    expect(goalHeaderText({ goal: { kind: "minutes", amount: 20 }, today: 12, met: false, streak: 3 })).toBe(
      "Today 12/20 min · streak 3",
    );
    expect(goalHeaderText({ goal: { kind: "texts", amount: 5 }, today: 2, met: true, streak: 0 })).toBe("Goal met");
  });
});
