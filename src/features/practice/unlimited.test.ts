import { describe, expect, it } from "vitest";
import { UNLIMITED_LENGTH } from "../settings/settings";
import {
  RETRY_AFTER_CHARS,
  TAIL_CHARS,
  UNLIMITED_START,
  continuationPiece,
  continuationTail,
  endAction,
  isUnlimited,
  requestLength,
  shouldContinue,
  typedPrefix,
} from "./unlimited";

describe("unlimited text: settings → request (S1, S2)", () => {
  it("requests 600 characters for the unlimited choice and the chosen length otherwise", () => {
    expect(UNLIMITED_START).toBe(600);
    expect(requestLength(UNLIMITED_LENGTH)).toBe(600);
    for (const n of [100, 300, 600, 800, 2000]) expect(requestLength(n)).toBe(n);
  });

  it("applies to prose and words, never to code or the diagnostic text", () => {
    expect(isUnlimited(UNLIMITED_LENGTH, "prose", false)).toBe(true);
    expect(isUnlimited(UNLIMITED_LENGTH, "words", false)).toBe(true);
    expect(isUnlimited(UNLIMITED_LENGTH, { kind: "code", lang: "rust" }, false)).toBe(false);
    expect(isUnlimited(UNLIMITED_LENGTH, "prose", true)).toBe(false);
    for (const n of [300, 2000]) expect(isUnlimited(n, "prose", false)).toBe(false);
  });
});

describe("shouldContinue (S2)", () => {
  const idle = { pending: false, retryFrom: 0 };

  it("triggers when the cursor reaches half of the current text", () => {
    expect(shouldContinue({ cursor: 299, length: 600, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 300, length: 600, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 301, length: 601, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 300, length: 601, ...idle })).toBe(false);
  });

  it("re-triggers at half of the new total after an append", () => {
    expect(shouldContinue({ cursor: 300, length: 1201, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 601, length: 1201, ...idle })).toBe(true);
  });

  it("asks at most once at a time", () => {
    expect(shouldContinue({ cursor: 500, length: 600, pending: true, retryFrom: 0 })).toBe(false);
  });

  it("after a failure waits until the cursor reaches the retry point", () => {
    expect(RETRY_AFTER_CHARS).toBe(50);
    expect(shouldContinue({ cursor: 349, length: 600, pending: false, retryFrom: 350 })).toBe(false);
    expect(shouldContinue({ cursor: 350, length: 600, pending: false, retryFrom: 350 })).toBe(true);
  });

  it("never triggers on an empty text", () => {
    expect(shouldContinue({ cursor: 0, length: 0, ...idle })).toBe(false);
  });
});

describe("continuationTail (S4)", () => {
  it("is the last 400 chars, counted as Unicode scalars", () => {
    expect(TAIL_CHARS).toBe(400);
    const chars = [..."ñ".repeat(100) + "a".repeat(400)];
    expect(continuationTail(chars)).toBe("a".repeat(400));
    const mixed = [..."x".repeat(10) + "é".repeat(400)];
    expect(continuationTail(mixed)).toBe("é".repeat(400));
  });

  it("is the whole text when it is shorter", () => {
    expect(continuationTail([..."hola"])).toBe("hola");
    expect(continuationTail([])).toBe("");
  });
});

describe("continuationPiece (S2)", () => {
  it("joins the continuation after a single space", () => {
    expect(continuationPiece([..."hola."], "Y adios.")).toBe(" Y adios.");
  });

  it("never doubles the space and trims what the continuation brings", () => {
    expect(continuationPiece([..."hola "], "  Y adios. ")).toBe("Y adios.");
    expect(continuationPiece([..."hola"], "\n Y adios.\n")).toBe(" Y adios.");
  });

  it("is empty for an empty continuation", () => {
    expect(continuationPiece([..."hola"], "  ")).toBe("");
  });
});

describe("ending an unlimited session (S3)", () => {
  it("scores the typed prefix, chars 0..cursor", () => {
    expect(typedPrefix([..."hola mundo"], 4)).toBe("hola");
    expect(typedPrefix([..."ñandú y"], 5)).toBe("ñandú");
  });

  it("discards a session with no typed chars and scores any other", () => {
    expect(endAction(0)).toBe("discard");
    expect(endAction(1)).toBe("score");
  });
});
