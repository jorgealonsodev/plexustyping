import { describe, it } from "vitest";
import { expectComplete } from "../../i18n/testing";
import { t } from "./messages";

describe("history messages", () => {
  it("are complete in every language (S5)", () => expectComplete(t));
});
