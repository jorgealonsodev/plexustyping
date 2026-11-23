import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { lengthChoices } from "./length";

afterEach(() => ui.set("es"));

describe("lengthChoices (header length picker)", () => {
  it("offers the text lengths of the settings for prose and words", () => {
    const prose = lengthChoices("prose", 12);
    expect(prose.map((c) => c.value)).toEqual([100, 200, 300, 450, 600, 800, 1000, 1500, 2000, 0]);
    expect(prose.find((c) => c.value === 300)?.label).toBe("300 caracteres");
    expect(prose.at(-1)?.label).toBe("Ilimitado");
    expect(lengthChoices("words", 12)).toEqual(prose);
  });

  it("offers maximum lines for code, keeping a stored value that is not a preset", () => {
    expect(lengthChoices("code", 12).map((c) => c.value)).toEqual([5, 8, 12, 16, 20]);
    expect(lengthChoices("code", 7).map((c) => c.value)).toEqual([5, 7, 8, 12, 16, 20]);
    expect(lengthChoices("code", 12).find((c) => c.value === 12)?.label).toBe("12 líneas");
  });

  it("follows the interface language", () => {
    ui.set("en");
    expect(lengthChoices("prose", 12).find((c) => c.value === 300)?.label).toBe("300 characters");
    expect(lengthChoices("code", 12).find((c) => c.value === 5)?.label).toBe("5 lines");
  });
});
