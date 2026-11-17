import { describe, expect, it } from "vitest";
import { indentChangeRestarts, isNewTextShortcut, shouldApply } from "./replies";

describe("isNewTextShortcut", () => {
  const key = (k: string, mods: Partial<Record<"ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat", boolean>> = {}) => ({
    key: k,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    repeat: false,
    ...mods,
  });

  it("is a bare F5, not held down nor with modifiers", () => {
    expect(isNewTextShortcut(key("F5"))).toBe(true);
    expect(isNewTextShortcut(key("F5", { repeat: true }))).toBe(false);
    for (const mod of ["ctrlKey", "altKey", "metaKey", "shiftKey"] as const) {
      expect(isNewTextShortcut(key("F5", { [mod]: true }))).toBe(false);
    }
    expect(isNewTextShortcut(key("F4"))).toBe(false);
    expect(isNewTextShortcut(key("r", { ctrlKey: true }))).toBe(false);
  });
});

describe("shouldApply", () => {
  it("applies a reply only to the request it answers", () => {
    expect(shouldApply(3, 3)).toBe(true);
    // A heatmap or weaknesses reply of an earlier request (e.g. the previous mode) is stale.
    expect(shouldApply(2, 3)).toBe(false);
    expect(shouldApply(4, 3)).toBe(false);
  });
});

describe("indentChangeRestarts", () => {
  it("restarts a code text only while it is being typed", () => {
    expect(indentChangeRestarts("typing", true)).toBe(true);
    expect(indentChangeRestarts("typing", false)).toBe(false);
  });

  it("keeps a text loading, being scored, its results or an error", () => {
    for (const phase of ["loading", "scoring", "results", "error"] as const) {
      expect(indentChangeRestarts(phase, true)).toBe(false);
      expect(indentChangeRestarts(phase, false)).toBe(false);
    }
  });
});
