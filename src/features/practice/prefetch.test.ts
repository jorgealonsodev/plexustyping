import { describe, expect, it } from "vitest";
import { Prefetcher, generationKey, isProfileSwitch, localDay, provisionalFailedChars, sourceLabel } from "./prefetch";

const deferred = <T>() => {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

describe("sourceLabel", () => {
  it("names the AI provider and model, the cache, the corpus and the diagnostic", () => {
    expect(sourceLabel({ source: "ai", provider: "claude", model: "claude-haiku-5-5" })).toBe(
      "Generado con IA · Claude · claude-haiku-5-5",
    );
    expect(sourceLabel({ source: "ai", provider: "opencode-go", model: "kimi-k2" })).toBe(
      "Generado con IA · OpenCode Go · kimi-k2",
    );
    expect(sourceLabel({ source: "ai" })).toBe("Generado con IA");
    expect(sourceLabel({ source: "cache", provider: "gemini", model: "g" })).toBe("Texto del caché");
    expect(sourceLabel({ source: "corpus" })).toBe("Corpus local");
    expect(sourceLabel({ source: "corpus", diagnostic: true })).toBe("Texto de diagnóstico");
  });
});

describe("generationKey", () => {
  it("changes with every setting that affects the next text", () => {
    const base = generationKey(3, 300, 0);
    expect(generationKey(3, 300, 0)).toBe(base);
    expect(generationKey(4, 300, 0)).not.toBe(base);
    expect(generationKey(3, 450, 0)).not.toBe(base);
    expect(generationKey(3, 300, 1)).not.toBe(base);
    expect(generationKey(3, 300, 0, "prose", 12)).toBe(base);
    const rust = generationKey(3, 300, 0, "code:rust", 12);
    expect(rust).not.toBe(base);
    expect(generationKey(3, 300, 0, "code:python", 12)).not.toBe(rust);
    expect(generationKey(3, 300, 0, "code:rust", 20)).not.toBe(rust);
    expect(generationKey(3, 300, 0, "prose", 12, "es-qwerty")).toBe(base);
    expect(generationKey(3, 300, 0, "prose", 12, "us-qwerty")).not.toBe(base);
    expect(generationKey(3, 300, 0, "prose", 12, "es-qwerty", "es")).toBe(base);
    expect(generationKey(3, 300, 0, "prose", 12, "es-qwerty", "fr")).not.toBe(base);
    expect(generationKey(3, 300, 0, "words", 12, "es-qwerty", "es")).not.toBe(base);
    expect(generationKey(3, 300, 0, "words", 12, "es-qwerty", "fr")).not.toBe(generationKey(3, 300, 0, "words", 12, "es-qwerty", "es"));
  });
});

describe("Prefetcher", () => {
  it("hands over the request started for the same key once", async () => {
    const p = new Prefetcher<string>();
    let calls = 0;
    p.start("k", async () => `t${++calls}`);
    p.start("k", async () => `t${++calls}`);
    expect(calls).toBe(1);
    expect(p.has("k")).toBe(true);
    await expect(p.take("k")).resolves.toBe("t1");
    expect(p.take("k")).toBeNull();
    expect(p.has("k")).toBe(false);
  });

  it("drops a request whose key is stale", () => {
    const p = new Prefetcher<string>();
    p.start("old", async () => "x");
    expect(p.has("new")).toBe(false);
    expect(p.take("new")).toBeNull();
    expect(p.take("old")).toBeNull();
  });

  it("replaces a stale request when started for a new key", async () => {
    const p = new Prefetcher<string>();
    const first = deferred<string>();
    p.start("a", () => first.promise);
    p.start("b", async () => "b");
    await expect(p.take("b")).resolves.toBe("b");
    first.resolve("a");
  });

  it("returns a pending request so the caller can await it, and keeps rejections for the taker", async () => {
    const p = new Prefetcher<string>();
    const d = deferred<string>();
    p.start("k", () => d.promise);
    const taken = p.take("k");
    expect(taken).not.toBeNull();
    d.reject(new Error("sin red"));
    await expect(taken).rejects.toThrow("sin red");
  });

  it("clear forgets everything", () => {
    const p = new Prefetcher<string>();
    p.start("k", async () => "x");
    p.clear();
    expect(p.take("k")).toBeNull();
  });
});

describe("provisionalFailedChars", () => {
  it("lists expected chars of wrong keystrokes, most failed first, first seen on ties", () => {
    const r = (expected: string, correct: boolean) => ({ expected, correct });
    expect(provisionalFailedChars([])).toEqual([]);
    expect(
      provisionalFailedChars([r("a", false), r("b", true), r("c", false), r("c", false), r("a", true), r("d", false)]),
    ).toEqual(["c", "a", "d"]);
  });
});

describe("localDay", () => {
  it("formats the local calendar day as YYYY-MM-DD", () => {
    expect(localDay(new Date(2026, 10, 16, 23, 59))).toBe("2026-11-16");
    expect(localDay(new Date(2027, 0, 5, 0, 1))).toBe("2027-01-05");
  });
});

describe("isProfileSwitch", () => {
  it("is true only when the profile part of the request key changes", () => {
    // A new profile gets its diagnostic text, as at start-up.
    expect(isProfileSwitch("0|prose|es-qwerty|es", "1|prose|es-qwerty|es")).toBe(true);
    expect(isProfileSwitch("1|prose|es-qwerty|es", "1|prose|us-qwerty|en")).toBe(false);
    expect(isProfileSwitch("1|prose|es-qwerty|es", "1|code:rust|es-qwerty|")).toBe(false);
    expect(isProfileSwitch("2|prose|es-qwerty|es", "2|prose|es-qwerty|es")).toBe(false);
  });
});
