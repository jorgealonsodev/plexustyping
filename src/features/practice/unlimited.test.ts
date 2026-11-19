import { describe, expect, it } from "vitest";
import { UNLIMITED_LENGTH } from "../settings/settings";
import { type ErrorMode, type InputEvent, TypingSession, replay } from "./engine";
import {
  CONTINUE_DEADLINE_MS,
  withDeadline,
  CONTINUE_AHEAD,
  RETRY_AFTER_CHARS,
  RETRY_DELAY_MS,
  TAIL_CHARS,
  UNLIMITED_START,
  continuationPiece,
  continuationTail,
  endAction,
  continuationFailureNotice,
  isUnlimited,
  noticeAfterContinuation,
  prefixEvents,
  requestLength,
  retryPlan,
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
    expect(shouldContinue({ cursor: 899, length: 1800, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 900, length: 1800, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 901, length: 1801, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 900, length: 1801, ...idle })).toBe(false);
  });

  it("also triggers when at most 400 chars remain ahead of the cursor (S5)", () => {
    expect(CONTINUE_AHEAD).toBe(400);
    expect(shouldContinue({ cursor: 199, length: 600, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 200, length: 600, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 600, length: 600, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 50, length: 300, ...idle })).toBe(true);
  });

  it("near the end still asks one at a time and keeps the retry point (S5)", () => {
    expect(shouldContinue({ cursor: 600, length: 600, pending: true, retryFrom: 0 })).toBe(false);
    expect(shouldContinue({ cursor: 300, length: 600, pending: false, retryFrom: 350 })).toBe(false);
    expect(shouldContinue({ cursor: 600, length: 600, pending: false, retryFrom: Number.POSITIVE_INFINITY })).toBe(false);
  });

  it("re-triggers at half of the new total after an append", () => {
    expect(shouldContinue({ cursor: 300, length: 1201, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 600, length: 1201, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 601, length: 1201, ...idle })).toBe(true);
    expect(shouldContinue({ cursor: 600, length: 2401, ...idle })).toBe(false);
    expect(shouldContinue({ cursor: 1201, length: 2401, ...idle })).toBe(true);
  });

  it("asks at most once at a time", () => {
    expect(shouldContinue({ cursor: 500, length: 600, pending: true, retryFrom: 0 })).toBe(false);
  });

  it("after a failure waits until the cursor reaches the retry point", () => {
    expect(RETRY_AFTER_CHARS).toBe(50);
    expect(shouldContinue({ cursor: 949, length: 1800, pending: false, retryFrom: 950 })).toBe(false);
    expect(shouldContinue({ cursor: 950, length: 1800, pending: false, retryFrom: 950 })).toBe(true);
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

describe("retrying a failed or empty continuation (S5)", () => {
  it("with more than 400 chars ahead, asks again 50 chars later, without a timer", () => {
    expect(retryPlan(900, 1800)).toEqual({ retryFrom: 950, delayMs: null });
    expect(retryPlan(199, 600)).toEqual({ retryFrom: 249, delayMs: null });
  });

  it("at the end or within 400 chars of it, asks again after a short delay, not by cursor", () => {
    expect(RETRY_DELAY_MS).toBe(2500);
    for (const [cursor, length] of [
      [600, 600],
      [200, 600],
      [1401, 1801],
    ])
      expect(retryPlan(cursor, length)).toEqual({ retryFrom: Number.POSITIVE_INFINITY, delayMs: 2500 });
  });

  it("while the timer runs no keystroke asks; once it fires the end asks again", () => {
    const { retryFrom } = retryPlan(600, 600);
    expect(shouldContinue({ cursor: 600, length: 600, pending: false, retryFrom })).toBe(false);
    expect(shouldContinue({ cursor: 600, length: 600, pending: false, retryFrom: 0 })).toBe(true);
  });

  it("names the failure in a non-blocking notice", () => {
    expect(continuationFailureNotice("sin conexión")).toBe("No se pudo continuar el texto: sin conexión; reintentando…");
  });
});

describe("noticeAfterContinuation (S5)", () => {
  const initial = { text: "IA no disponible", continuation: false };
  const fromContinuation = { text: "Proveedor lento", continuation: true };

  it("shows the notice a continuation brings", () => {
    expect(noticeAfterContinuation(initial, "Sin IA: corpus")).toEqual({ text: "Sin IA: corpus", continuation: true });
    expect(noticeAfterContinuation(fromContinuation, "Otro")).toEqual({ text: "Otro", continuation: true });
  });

  it("a continuation without a notice clears the previous continuation notice", () => {
    expect(noticeAfterContinuation(fromContinuation, null)).toEqual({ text: null, continuation: false });
    expect(noticeAfterContinuation(fromContinuation, undefined)).toEqual({ text: null, continuation: false });
    expect(noticeAfterContinuation(fromContinuation, "")).toEqual({ text: null, continuation: false });
  });

  it("keeps a notice that did not come from a continuation", () => {
    expect(noticeAfterContinuation(initial, null)).toEqual(initial);
    expect(noticeAfterContinuation({ text: null, continuation: false }, null)).toEqual({ text: null, continuation: false });
  });
});

describe("prefixEvents: the events an ended session is scored with (S5)", () => {
  const TEXT = "hola mundo";
  let t = 0;
  const ch = (s: string): InputEvent[] => [...s].map((c) => ({ kind: "char", ch: c, tMs: (t += 100) }));
  const bs = (n = 1): InputEvent[] => Array.from({ length: n }, () => ({ kind: "backspace", tMs: (t += 100) }));

  /** Types `events` on the whole text, ends at the cursor and checks the prefix replay. */
  function end(mode: ErrorMode, events: InputEvent[]) {
    const session = new TypingSession(TEXT, mode);
    for (const e of events) session.apply(e);
    const prefix = typedPrefix(session.chars, session.cursor);
    const offsets = events.map((_, i) => i * 10);
    const kept = prefixEvents(prefix, mode, events, offsets);
    const scored = replay(prefix, mode, kept.events);
    expect(scored.completed, `${mode}: the prefix replay completes`).toBe(true);
    expect(scored.cursor).toBe([...prefix].length);
    // The same records the session made with the kept events.
    expect(scored.records).toEqual(replay(TEXT, mode, kept.events).records);
    expect(kept.events).toEqual(events.slice(0, kept.events.length));
    expect(kept.offsets).toEqual(offsets.slice(0, kept.events.length));
    return { prefix, kept: kept.events.length, scored };
  }

  it("backspace-correct: drops chars typed past the cursor and erased", () => {
    const r = end("backspace-correct", [...ch("hola mx"), ...bs(2)]);
    expect([r.prefix, r.kept]).toEqual(["hola ", 5]);
    expect(r.scored.stats.uncorrectedErrors).toBe(0);
  });

  it("backspace-correct: keeps a corrected error before the cursor", () => {
    const r = end("backspace-correct", [...ch("hx"), ...bs(), ...ch("ola")]);
    expect([r.prefix, r.kept]).toEqual(["hola", 6]);
    expect([r.scored.stats.errors, r.scored.stats.uncorrectedErrors]).toEqual([1, 0]);
  });

  it("stop-on-error: drops a wrong key at the cursor that did not advance", () => {
    const r = end("stop-on-error", [...ch("hola m"), ...ch("x")]);
    expect([r.prefix, r.kept]).toEqual(["hola m", 6]);
    expect(r.scored.stats.errors).toBe(0);
  });

  it("stop-on-error: keeps the wrong keys before the cursor", () => {
    const r = end("stop-on-error", [...ch("hx"), ...ch("ola")]);
    expect([r.prefix, r.kept]).toEqual(["hola", 5]);
    expect(r.scored.stats.errors).toBe(1);
  });

  it("allow-continue: keeps every event, the wrong chars count as uncorrected", () => {
    const r = end("allow-continue", ch("hola mx"));
    expect([r.prefix, r.kept]).toEqual(["hola mu", 7]);
    expect(r.scored.stats.uncorrectedErrors).toBe(1);
  });

  it("keeps nothing for an empty prefix", () => {
    expect(prefixEvents("", "backspace-correct", ch("x"), [1])).toEqual({ events: [], offsets: [] });
  });
});

describe("withDeadline (S5)", () => {
  it("passes a reply that arrives in time and turns a silent provider into a failure", async () => {
    await expect(withDeadline(Promise.resolve("ok"), 50)).resolves.toBe("ok");
    await expect(withDeadline(new Promise(() => {}), 10)).rejects.toThrow("La IA tardó demasiado en responder");
    await expect(withDeadline(Promise.reject(new Error("x")), 50)).rejects.toThrow("x");
    expect(CONTINUE_DEADLINE_MS).toBe(25_000);
  });
});
