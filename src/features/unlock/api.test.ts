import { beforeEach, describe, expect, it, vi } from "vitest";

// The reply of the mocked `invoke` (a plain function: a `vi.fn` reports a rejection it returns as a failure).
const backend = vi.hoisted(() => ({ reply: (): Promise<unknown> => Promise.resolve(), calls: [] as unknown[][] }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => {
    backend.calls.push(args);
    return backend.reply();
  },
}));

import { unlockSet, unlockStatus } from "./api";

const status = {
  enabled: true,
  targetWpm: 30,
  active: ["a", "s", "d", "f", "g", "h", "j", "k", "l"],
  mastered: ["f", "j"],
  next: "e",
  total: 27,
  stalledSessions: 1,
};

describe("unlock command bindings (S7)", () => {
  beforeEach(() => {
    backend.calls = [];
  });
  const resolve = (value: unknown) => (backend.reply = () => Promise.resolve(value));
  const last = () => backend.calls[backend.calls.length - 1];

  it("reads the status for a language and layout", async () => {
    resolve(status);
    await expect(unlockStatus("es", "es-qwerty")).resolves.toEqual(status);
    expect(last()).toEqual(["unlock_status", { language: "es", layout: "es-qwerty" }]);
  });

  it("sets the mode or the target speed, leaving the other unset", async () => {
    resolve(status);
    await expect(unlockSet("es", "es-qwerty", { enabled: true })).resolves.toEqual(status);
    expect(last()).toEqual(["unlock_set", { language: "es", layout: "es-qwerty", enabled: true, targetWpm: null }]);
    await unlockSet("en", "us-qwerty", { targetWpm: 40 });
    expect(last()).toEqual(["unlock_set", { language: "en", layout: "us-qwerty", enabled: null, targetWpm: 40 }]);
  });

  it("passes the backend's Spanish error through", async () => {
    backend.reply = () => Promise.reject("Velocidad objetivo no válida");
    const error = await unlockSet("es", "es-qwerty", { targetWpm: 35 }).catch((err: unknown) => err);
    expect(error).toBe("Velocidad objetivo no válida");
  });
});
