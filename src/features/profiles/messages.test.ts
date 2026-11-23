import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { t } from "./messages";
import { deleteConfirmation } from "./profiles";

afterEach(() => ui.set("es"));

describe("profile messages (ui-i18n S1, S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("asks before deleting a profile in the interface language", () => {
    ui.set("en");
    expect(deleteConfirmation("Ana")).toBe("The history and statistics of “Ana” will be deleted. Continue?");
  });
});
