import { describe, it } from "vitest";
import { expectComplete } from "../../i18n/testing";
import { t } from "./messages";

describe("code messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));
});
