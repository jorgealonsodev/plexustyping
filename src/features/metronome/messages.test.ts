import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { SOUNDS, tempoNotice } from "./beat";
import { t } from "./messages";

afterEach(() => ui.set("es"));

describe("metronome messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));

  it("follow the interface language (S1)", () => {
    ui.set("en");
    expect(SOUNDS.map((s) => s.label)).toEqual(["Click", "Wood", "Beep"]);
    expect(tempoNotice({ from: 80, to: 80 })).toBe("Tempo: stays at 80 BPM");
    expect(tempoNotice({ from: 80, to: 85 })).toBe("Tempo: 80 → 85 BPM");
  });
});
