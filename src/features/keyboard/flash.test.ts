import { describe, expect, it } from "vitest";
import { contrastRatio } from "../themes";
import { OKABE_ITO_BLUE, OKABE_ITO_ORANGE, flashVars } from "./flash";

describe("flash colours (a11y S5)", () => {
  it("uses the theme's success and danger colours, labelled in --bg, by default", () => {
    expect(flashVars(false)).toEqual({
      "--flash-ok": "var(--success)",
      "--flash-ok-fg": "var(--bg)",
      "--flash-bad": "var(--danger)",
      "--flash-bad-fg": "var(--bg)",
    });
  });

  it("uses the Okabe–Ito blue and orange pair in colour-blind mode (L3)", () => {
    expect([OKABE_ITO_BLUE, OKABE_ITO_ORANGE]).toEqual(["#0072B2", "#E69F00"]);
    const vars = flashVars(true);
    expect(vars["--flash-ok"]).toBe("#0072B2");
    expect(vars["--flash-bad"]).toBe("#E69F00");
  });

  it("labels each colour-blind flash in black or white, whichever reads, at least AA", () => {
    const vars = flashVars(true);
    expect(vars["--flash-ok-fg"]).toBe("#ffffff");
    expect(vars["--flash-bad-fg"]).toBe("#000000");
    expect(contrastRatio(vars["--flash-ok-fg"], vars["--flash-ok"].toLowerCase())).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(vars["--flash-bad-fg"], vars["--flash-bad"].toLowerCase())).toBeGreaterThanOrEqual(4.5);
  });
});
