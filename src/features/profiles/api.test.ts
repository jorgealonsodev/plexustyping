import { beforeEach, describe, expect, it, vi } from "vitest";

// The reply of the mocked `invoke` (a plain function: a `vi.fn` reports a rejection it returns as a failure).
const backend = vi.hoisted(() => ({ reply: (): Promise<unknown> => Promise.resolve(), calls: [] as unknown[][] }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => {
    backend.calls.push(args);
    return backend.reply();
  },
}));

import { profileCreate, profileDelete, profileSwitch, profileUpdate, profilesList } from "./api";

const ana = { id: 2, name: "Ana", language: "es", layout: "es-qwerty", theme: null, createdAtMs: 1 };

describe("profile command bindings (S3)", () => {
  beforeEach(() => {
    backend.calls = [];
  });
  const resolve = (value: unknown) => (backend.reply = () => Promise.resolve(value));
  const last = () => backend.calls[backend.calls.length - 1];

  it("lists profiles with the active id", async () => {
    resolve({ active: 2, profiles: [ana] });
    await expect(profilesList()).resolves.toEqual({ active: 2, profiles: [ana] });
    expect(last()).toEqual(["profiles_list"]);
  });

  it("creates a profile with its name, language and layout", async () => {
    resolve(ana);
    await expect(profileCreate("Ana", "es", "es-qwerty")).resolves.toEqual(ana);
    expect(last()).toEqual(["profile_create", { name: "Ana", language: "es", layout: "es-qwerty" }]);
  });

  it("updates only the given fields; the rest travel as null (left unchanged)", async () => {
    resolve(ana);
    await profileUpdate(2, { layout: "us-qwerty" });
    expect(last()).toEqual(["profile_update", { id: 2, name: null, language: null, layout: "us-qwerty", theme: null }]);
    await profileUpdate(2, { name: "Bea", language: "en", theme: "nord" });
    expect(last()).toEqual(["profile_update", { id: 2, name: "Bea", language: "en", layout: null, theme: "nord" }]);
  });

  it("deletes and switches by id", async () => {
    resolve({ active: ana });
    await expect(profileDelete(3)).resolves.toEqual({ active: ana });
    expect(last()).toEqual(["profile_delete", { id: 3 }]);
    resolve(ana);
    await expect(profileSwitch(2)).resolves.toEqual(ana);
    expect(last()).toEqual(["profile_switch", { id: 2 }]);
  });

  it("passes the backend's Spanish error through", async () => {
    backend.reply = () => Promise.reject("Ya existe un perfil con ese nombre");
    const error = await profileCreate("ana", "es", "es-qwerty").catch((err: unknown) => err);
    expect(error).toBe("Ya existe un perfil con ese nombre");
  });
});
