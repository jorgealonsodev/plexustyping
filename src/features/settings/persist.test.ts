import { describe, expect, it, vi } from "vitest";
import { MIGRATED_KEY, type SettingsBackend, StoreStorage, initSettingsStorage, settingsStorage } from "./persist";
import { loadLayout, loadShowKeyboard, saveLayout, saveShowKeyboard } from "./settings";

/** An in-memory `Storage` standing in for the webview's localStorage. */
function memoryStorage(initial: Record<string, string> = {}): Storage {
  const items = new Map(Object.entries(initial));
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, String(value)),
  };
}

type Call = ["set", string, string] | ["remove", string];

/** A fake store backend: every write resolves on the next tick (or fails when `fail` says so). */
function fakeBackend(stored: Record<string, string> = {}, opts: { readError?: unknown; fail?: (call: Call) => boolean } = {}) {
  const values = new Map(Object.entries(stored));
  const calls: Call[] = [];
  /** Writes in progress at once; serialized writes never exceed 1. */
  let inFlight = 0;
  let maxInFlight = 0;
  const write = async (call: Call) => {
    calls.push(call);
    inFlight++;
    maxInFlight = Math.max(maxInFlight, inFlight);
    await new Promise((resolve) => setTimeout(resolve, 1));
    inFlight--;
    if (opts.fail?.(call)) throw new Error("disco lleno");
    if (call[0] === "set") values.set(call[1], call[2]);
    else values.delete(call[1]);
  };
  const backend: SettingsBackend = {
    get: async () => {
      if (opts.readError !== undefined) throw opts.readError;
      return Object.fromEntries(values);
    },
    set: (key, value) => write(["set", key, value]),
    remove: (key) => write(["remove", key]),
  };
  return { backend, values, calls, maxInFlight: () => maxInFlight };
}

describe("StoreStorage", () => {
  it("reads the snapshot synchronously and sees its own writes at once (S1, S4)", () => {
    const { backend } = fakeBackend();
    const storage = new StoreStorage({ "plexustyping.layout": "us-qwerty" }, backend);
    expect(storage.getItem("plexustyping.layout")).toBe("us-qwerty");
    expect(storage.getItem("plexustyping.none")).toBeNull();
    expect(storage.length).toBe(1);
    expect(storage.key(0)).toBe("plexustyping.layout");
    expect(storage.key(1)).toBeNull();
    storage.setItem("plexustyping.theme", "dark");
    expect(storage.getItem("plexustyping.theme")).toBe("dark");
    expect(storage.length).toBe(2);
    storage.removeItem("plexustyping.layout");
    expect(storage.getItem("plexustyping.layout")).toBeNull();
    expect(storage.length).toBe(1);
  });

  it("sends every write to the store one at a time, in order (S1)", async () => {
    const fake = fakeBackend();
    const storage = new StoreStorage({}, fake.backend);
    storage.setItem("plexustyping.layout", "us-qwerty");
    storage.setItem("plexustyping.layout", "fr-azerty");
    storage.removeItem("plexustyping.layout");
    storage.setItem("plexustyping.keyboard", "false");
    await storage.flush();
    expect(fake.calls).toEqual([
      ["set", "plexustyping.layout", "us-qwerty"],
      ["set", "plexustyping.layout", "fr-azerty"],
      ["remove", "plexustyping.layout"],
      ["set", "plexustyping.keyboard", "false"],
    ]);
    expect(fake.maxInFlight()).toBe(1);
    expect(Object.fromEntries(fake.values)).toEqual({ "plexustyping.keyboard": "false" });
  });

  it("keeps the value in memory and logs when a write fails, and later writes still go out (S5)", async () => {
    const fake = fakeBackend({}, { fail: (call) => call[1] === "plexustyping.theme" });
    const log = vi.fn();
    const storage = new StoreStorage({}, fake.backend, log);
    storage.setItem("plexustyping.theme", "dark");
    storage.setItem("plexustyping.layout", "us-qwerty");
    await storage.flush();
    expect(storage.getItem("plexustyping.theme")).toBe("dark");
    expect(log).toHaveBeenCalledTimes(1);
    expect(String(log.mock.calls[0][0])).toContain("plexustyping.theme");
    expect(Object.fromEntries(fake.values)).toEqual({ "plexustyping.layout": "us-qwerty" });
  });

  it("works with the existing load/save helpers and their validation (S4)", async () => {
    const fake = fakeBackend();
    const storage = new StoreStorage({ "plexustyping.layout": "nope" }, fake.backend);
    expect(loadLayout(storage)).toBe("es-qwerty");
    saveLayout("us-dvorak", storage);
    saveShowKeyboard(false, storage);
    expect(loadLayout(storage)).toBe("us-dvorak");
    expect(loadShowKeyboard(storage)).toBe(false);
    await storage.flush();
    expect(fake.values.get("plexustyping.layout")).toBe("us-dvorak");
    expect(fake.values.get("plexustyping.keyboard")).toBe("false");
  });
});

describe("initSettingsStorage", () => {
  it("starts from the store's values when it holds UI settings, ignoring localStorage (S2, S3)", async () => {
    const fake = fakeBackend({ "plexustyping.layout": "us-qwerty", [MIGRATED_KEY]: "true" });
    const local = memoryStorage({ "plexustyping.layout": "fr-azerty", "plexustyping.theme": "dark" });
    const storage = await initSettingsStorage(fake.backend, local);
    expect(storage.getItem("plexustyping.layout")).toBe("us-qwerty");
    expect(storage.getItem("plexustyping.theme")).toBeNull();
    expect(settingsStorage()).toBe(storage);
    expect(loadLayout()).toBe("us-qwerty");
    await (storage as StoreStorage).flush();
    expect(fake.calls).toEqual([]);
  });

  it("copies only plexustyping.* keys from localStorage once when the store has none, leaving localStorage as is (S3)", async () => {
    const fake = fakeBackend();
    const local = memoryStorage({
      "plexustyping.layout": "fr-azerty",
      "plexustyping.keyboard": "false",
      "other.key": "x",
      plexustyping: "y",
    });
    const before = JSON.stringify([...Array(local.length).keys()].map((i) => [local.key(i), local.getItem(local.key(i)!)]));
    const storage = await initSettingsStorage(fake.backend, local);
    expect(storage.getItem("plexustyping.layout")).toBe("fr-azerty");
    expect(storage.getItem("plexustyping.keyboard")).toBe("false");
    expect(storage.getItem("other.key")).toBeNull();
    // The copies, then the marker once they all landed.
    await (storage as StoreStorage).flush();
    await (storage as StoreStorage).flush();
    expect(Object.fromEntries(fake.values)).toEqual({
      "plexustyping.layout": "fr-azerty",
      "plexustyping.keyboard": "false",
      [MIGRATED_KEY]: "true",
    });
    expect(JSON.stringify([...Array(local.length).keys()].map((i) => [local.key(i), local.getItem(local.key(i)!)]))).toBe(before);

    // The next start reads only the store, even after localStorage changed.
    local.setItem("plexustyping.layout", "de-qwertz");
    const writes = fake.calls.length;
    const again = await initSettingsStorage(fake.backend, local);
    expect(again.getItem("plexustyping.layout")).toBe("fr-azerty");
    await (again as StoreStorage).flush();
    expect(fake.calls.length).toBe(writes);
  });

  it("marks an empty first start as migrated so later starts never import localStorage (S3)", async () => {
    const fake = fakeBackend();
    const storage = await initSettingsStorage(fake.backend, memoryStorage());
    await (storage as StoreStorage).flush();
    await (storage as StoreStorage).flush();
    expect(Object.fromEntries(fake.values)).toEqual({ [MIGRATED_KEY]: "true" });
    const again = await initSettingsStorage(fake.backend, memoryStorage({ "plexustyping.layout": "us-qwerty" }));
    expect(again.getItem("plexustyping.layout")).toBeNull();
  });

  it("resumes an interrupted migration: copies only the keys the store lacks, the store's values win (S3)", async () => {
    // A first start that was closed before every copy landed: some keys, no marker.
    const fake = fakeBackend({ "plexustyping.layout": "us-qwerty" });
    const local = memoryStorage({ "plexustyping.layout": "fr-azerty", "plexustyping.keyboard": "false" });
    const storage = await initSettingsStorage(fake.backend, local);
    expect(storage.getItem("plexustyping.layout")).toBe("us-qwerty");
    expect(storage.getItem("plexustyping.keyboard")).toBe("false");
    await (storage as StoreStorage).flush();
    await (storage as StoreStorage).flush();
    expect(Object.fromEntries(fake.values)).toEqual({
      "plexustyping.layout": "us-qwerty",
      "plexustyping.keyboard": "false",
      [MIGRATED_KEY]: "true",
    });
  });

  it("leaves the marker unwritten when a migrated key fails, so the next start retries it (S3, S5)", async () => {
    let full = true;
    const fake = fakeBackend({}, { fail: (call) => full && call[1] === "plexustyping.keyboard" });
    const local = memoryStorage({ "plexustyping.layout": "fr-azerty", "plexustyping.keyboard": "false" });
    const log = vi.fn();
    const storage = await initSettingsStorage(fake.backend, local, log);
    await (storage as StoreStorage).flush();
    await (storage as StoreStorage).flush();
    expect(Object.fromEntries(fake.values)).toEqual({ "plexustyping.layout": "fr-azerty" });
    expect(log).toHaveBeenCalled();

    full = false;
    const again = await initSettingsStorage(fake.backend, local, log);
    await (again as StoreStorage).flush();
    await (again as StoreStorage).flush();
    expect(Object.fromEntries(fake.values)).toEqual({
      "plexustyping.layout": "fr-azerty",
      "plexustyping.keyboard": "false",
      [MIGRATED_KEY]: "true",
    });
  });

  it("falls back to localStorage and logs when the store cannot be read (S5)", async () => {
    const fake = fakeBackend({}, { readError: "La base de datos no está disponible" });
    const log = vi.fn();
    const local = memoryStorage({ "plexustyping.layout": "us-qwerty" });
    const storage = await initSettingsStorage(fake.backend, local, log);
    expect(storage).toBe(local);
    expect(settingsStorage()).toBe(local);
    expect(loadLayout()).toBe("us-qwerty");
    saveLayout("fr-azerty");
    expect(local.getItem("plexustyping.layout")).toBe("fr-azerty");
    expect(log).toHaveBeenCalledTimes(1);
    expect(fake.calls).toEqual([]);
  });
});
