import { describe, expect, it } from "vitest";
import type { Layout } from "../keyboard/types";
import type { UnlockStatus } from "./api";
import {
  DEFAULT_TARGET_WPM,
  TARGET_SETTING,
  TARGET_WPMS,
  UNLOCK_SETTING,
  lockedKeys,
  progressLine,
  unlockApplies,
  unlockNotice,
} from "./unlock";

const status = (patch: Partial<UnlockStatus> = {}): UnlockStatus => ({
  enabled: true,
  targetWpm: 30,
  active: ["a", "s", "d", "f", "g", "h", "j", "k", "l"],
  mastered: [],
  next: "e",
  total: 27,
  stalledSessions: 0,
  ...patch,
});

const key = (normal: string | null, shift: string | null = null, altgr: string | null = null) => ({ normal, shift, altgr });
const layout: Layout = {
  id: "es-qwerty",
  name: "Español",
  deadKeys: ["´"],
  keys: {
    KeyA: key("a", "A"),
    KeyE: key("e", "E", "€"),
    KeyQ: key("q", "Q"),
    Semicolon: key("ñ", "Ñ"),
    Digit1: key("1", "!", "|"),
    Quote: key("´", "¨", "{"),
    Space: key(" ", " "),
  },
};

describe("unlock settings (S1)", () => {
  it("names the toggle and the target speed exactly", () => {
    expect(UNLOCK_SETTING).toEqual({
      label: "Desbloqueo progresivo",
      hint: "Empieza por la fila base y añade una tecla cada vez que dominas las activas",
    });
    expect(TARGET_SETTING.label).toBe("Velocidad objetivo");
    expect(TARGET_WPMS).toEqual([20, 30, 40, 50]);
    expect(DEFAULT_TARGET_WPM).toBe(30);
  });

  it("applies only to prose with the mode on", () => {
    expect(unlockApplies(status(), "prose")).toBe(true);
    expect(unlockApplies(status(), "words")).toBe(false);
    expect(unlockApplies(status(), { kind: "code", lang: "rust" })).toBe(false);
    expect(unlockApplies(status({ enabled: false }), "prose")).toBe(false);
    expect(unlockApplies(null, "prose")).toBe(false);
  });
});

describe("progress line (S6)", () => {
  it("counts the active letters of the order and names the next one", () => {
    expect(progressLine(status())).toBe("Teclas: 9/27 · siguiente: e");
  });

  it("ignores uppercase, digits and punctuation in the active set", () => {
    expect(progressLine(status({ active: ["a", "A", "s", ";", " "], total: 26, next: "d" }))).toBe("Teclas: 2/26 · siguiente: d");
  });

  it("drops the next key once every letter is unlocked", () => {
    const all = Array.from({ length: 26 }, (_, i) => String.fromCharCode(97 + i));
    expect(progressLine(status({ active: [...all, "1", "."], total: 26, next: null }))).toBe("Teclas: 26/26");
  });

  it("shows nothing with the mode off", () => {
    expect(progressLine(status({ enabled: false }))).toBeNull();
    expect(progressLine(null)).toBeNull();
  });
});

describe("locked keys (S6)", () => {
  it("dims every key whose chars are all locked; space and active letters stay lit", () => {
    expect(lockedKeys(layout, status({ active: ["a", "ñ"] }))).toEqual(new Set(["KeyE", "KeyQ", "Digit1", "Quote"]));
  });

  it("lights a key once its letter is unlocked", () => {
    expect(lockedKeys(layout, status({ active: ["a", "ñ", "e"] }))).toEqual(new Set(["KeyQ", "Digit1", "Quote"]));
  });

  it("lights everything once every letter is unlocked", () => {
    expect(lockedKeys(layout, status({ active: ["a", "e", "q", "ñ"], total: 4, next: null })).size).toBe(0);
  });

  it("dims nothing with the mode off or without a layout", () => {
    expect(lockedKeys(layout, status({ enabled: false })).size).toBe(0);
    expect(lockedKeys(null, status()).size).toBe(0);
    expect(lockedKeys(layout, null).size).toBe(0);
  });
});

describe("unlock notice (S6)", () => {
  it("names the key the session unlocked", () => {
    expect(unlockNotice("e")).toBe("Nueva tecla desbloqueada: e");
    expect(unlockNotice(null)).toBeNull();
    expect(unlockNotice(undefined)).toBeNull();
  });
});
