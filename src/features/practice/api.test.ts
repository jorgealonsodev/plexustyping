import { beforeEach, describe, expect, it, vi } from "vitest";

// The reply of the mocked `invoke` (a plain function: a `vi.fn` reports a rejection it returns as a failure).
const backend = vi.hoisted(() => ({ reply: (): Promise<unknown> => Promise.resolve(), calls: [] as unknown[][] }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => {
    backend.calls.push(args);
    return backend.reply();
  },
}));

import { continueText } from "./api";
import { localDay } from "./prefetch";

describe("continue_text binding (unlimited S4)", () => {
  beforeEach(() => {
    backend.calls = [];
  });

  it("sends the camelCase contract and returns the text", async () => {
    const reply = { text: "y sigue.", targets: ["ñ"], source: "corpus", notice: "Sin IA" };
    backend.reply = () => Promise.resolve(reply);
    await expect(continueText("es", "es-qwerty", "words", "cola", 3, 42)).resolves.toEqual(reply);
    expect(backend.calls).toEqual([
      ["continue_text", { language: "es", layout: "es-qwerty", mode: "words", tail: "cola", targetCount: 3, seed: 42, day: localDay() }],
    ]);
  });

  it("passes a backend error through", async () => {
    backend.reply = () => Promise.reject("La cola es demasiado larga");
    await expect(continueText("es", "es-qwerty", "prose", "x", 2, 1)).rejects.toBe("La cola es demasiado larga");
  });
});
