import { beforeEach, describe, expect, it, vi } from "vitest";

// The reply of the mocked `invoke` (a plain function: a `vi.fn` reports a rejection it returns as a failure).
const backend = vi.hoisted(() => ({ reply: (): Promise<unknown> => Promise.resolve(), calls: [] as unknown[][] }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => {
    backend.calls.push(args);
    return backend.reply();
  },
}));

import { goalSet, goalStatus } from "./api";

const status = { goal: { kind: "minutes", amount: 20 }, today: 12, met: false, streak: 3 };

describe("goal command bindings (S4)", () => {
  beforeEach(() => {
    backend.calls = [];
  });
  const resolve = (value: unknown) => (backend.reply = () => Promise.resolve(value));
  const last = () => backend.calls[backend.calls.length - 1];

  it("reads the status with the local offset", async () => {
    resolve(status);
    await expect(goalStatus(120)).resolves.toEqual(status);
    expect(last()).toEqual(["goal_status", { tzOffsetMin: 120 }]);
  });

  it("sets a goal with its amount, and off without one", async () => {
    resolve(status);
    await expect(goalSet("minutes", 20, -300)).resolves.toEqual(status);
    expect(last()).toEqual(["goal_set", { kind: "minutes", amount: 20, tzOffsetMin: -300 }]);
    resolve({ goal: null, today: 0, met: false, streak: 0 });
    await goalSet("off", null, 0);
    expect(last()).toEqual(["goal_set", { kind: "off", amount: null, tzOffsetMin: 0 }]);
  });

  it("passes the backend's Spanish error through", async () => {
    backend.reply = () => Promise.reject("Meta diaria no válida");
    const error = await goalSet("minutes", 7, 0).catch((err: unknown) => err);
    expect(error).toBe("Meta diaria no válida");
  });
});
