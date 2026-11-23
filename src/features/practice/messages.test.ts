import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import { expectComplete } from "../../i18n/testing";
import { errorMessage, newTextMessage, resultMessage, spokenChar } from "./announce";
import { typingHint } from "./focus";
import { formatPercent } from "./format";
import { formatDelta, ghostToggleTitle, newRecordLine, recordLines } from "./ghost";
import { spoken, t } from "./messages";
import { sourceLabel } from "./prefetch";
import { continuationFailureNotice, withDeadline } from "./unlimited";

afterEach(() => ui.set("es"));

describe("practice messages (ui-i18n S5)", () => {
  it("are complete in every language", () => expectComplete(t));
  it("name every spoken char in every language", () => expectComplete(spoken));
});

describe("practice texts follow the interface language (ui-i18n S1)", () => {
  it("formats percentages in the interface language", () => {
    expect(formatPercent(0.905)).toBe("90,5 %");
    ui.set("en");
    expect([formatPercent(0.905), formatPercent(1)]).toEqual(["90.5%", "100%"]);
    ui.set("de");
    expect(formatPercent(0.905)).toBe("90,5 %");
  });

  it("speaks chars and results in English", () => {
    ui.set("en");
    expect([spokenChar(" "), spokenChar("E"), spokenChar("¿")]).toEqual(["space", "capital e", "inverted question mark"]);
    expect(errorMessage("e", "r")).toBe("error: expected e, typed r");
    expect(newTextMessage(["a"])).toBe("New text of 1 character. next: a");
    expect(newTextMessage(["a", "b"])).toBe("New text of 2 characters. next: a");
    expect(resultMessage({ netWpm: 41.6, accuracy: 0.905, durationMs: 65_000, errors: 1 })).toBe(
      "Result: 42 net WPM, accuracy 90.5%, time 1:05, 1 error.",
    );
  });

  it("formats the ghost delta and record lines in English", () => {
    ui.set("en");
    expect([formatDelta(1234), formatDelta(-800), formatDelta(0)]).toEqual(["+1.2 s", "−0.8 s", "0.0 s"]);
    expect(recordLines(40, 45, true)).toEqual(["Previous record: 40 WPM", newRecordLine()]);
    expect(newRecordLine()).toBe("New record for this text!");
    expect(ghostToggleTitle(false)).toBe("Ghost session off");
  });

  it("labels hints, sources and notices in English", () => {
    ui.set("en");
    expect(typingHint(true, false)).toBe("Esc restarts the text · F4 metronome · F6 menu");
    expect(sourceLabel({ source: "cache", diagnostic: false })).toBe("Cached text");
    expect(sourceLabel({ source: "ai", provider: "claude", model: "m", diagnostic: false })).toBe("AI-generated · Claude · m");
    expect(continuationFailureNotice("x")).toBe("Could not continue the text: x; retrying…");
  });

  it("reports a stuck AI in the interface language", async () => {
    ui.set("en");
    await expect(withDeadline(new Promise(() => {}), 1)).rejects.toThrow("The AI took too long to reply");
  });
});
