import { describe, it } from "vitest";
import { t } from "./messages";
import { expectComplete } from "./testing";

describe("interface-language messages", () => {
  it("are complete in every language (S5)", () => expectComplete(t));
});
