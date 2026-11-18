import { describe, expect, it } from "vitest";
import type { Profile, ProfileFields, ProfilesApi, ProfilesList } from "./api";
import {
  MIGRATED_KEY,
  appearanceOf,
  carryOver,
  createProfile,
  deleteConfirmation,
  deleteProfile,
  renameProfile,
  saveAppearance,
  startProfiles,
  switchProfile,
} from "./profiles";

/** An in-memory `Storage` (vitest runs in node, without `localStorage`). */
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

const profile = (id: number, name: string, extra: Partial<Profile> = {}): Profile => ({
  id,
  name,
  language: "es",
  layout: "es-qwerty",
  theme: null,
  createdAtMs: id,
  ...extra,
});

/** An in-memory backend with the S3 semantics; `calls` records every command. */
function fakeApi(initial: Profile[], active = initial[0].id) {
  let profiles = initial.map((p) => ({ ...p }));
  let activeId = active;
  let nextId = Math.max(...profiles.map((p) => p.id)) + 1;
  const calls: [string, ...unknown[]][] = [];
  const find = (id: number) => {
    const p = profiles.find((q) => q.id === id);
    if (!p) throw "Ese perfil no existe";
    return p;
  };
  const api: ProfilesApi = {
    async list(): Promise<ProfilesList> {
      calls.push(["list"]);
      return { active: activeId, profiles: profiles.map((p) => ({ ...p })) };
    },
    async create(name, language, layout) {
      calls.push(["create", name, language, layout]);
      if (!name.trim()) throw "El nombre del perfil no puede estar vacío";
      const p = profile(nextId++, name.trim(), { language, layout });
      profiles.push(p);
      return { ...p };
    },
    async update(id, fields: ProfileFields) {
      calls.push(["update", id, fields]);
      const p = find(id);
      if (fields.name !== undefined) {
        if (profiles.some((q) => q.id !== id && q.name.toLowerCase() === fields.name!.trim().toLowerCase()))
          throw "Ya existe un perfil con ese nombre";
        p.name = fields.name.trim();
      }
      if (fields.language !== undefined) p.language = fields.language;
      if (fields.layout !== undefined) p.layout = fields.layout;
      if (fields.theme !== undefined) p.theme = fields.theme === "" ? null : fields.theme;
      return { ...p };
    },
    async remove(id) {
      calls.push(["remove", id]);
      find(id);
      if (profiles.length === 1) throw "No se puede borrar el único perfil";
      profiles = profiles.filter((p) => p.id !== id);
      if (activeId === id) activeId = profiles[0].id;
      return { active: { ...find(activeId) } };
    },
    async switchTo(id) {
      calls.push(["switch", id]);
      activeId = find(id).id;
      return { ...find(id) };
    },
  };
  return { api, calls, rows: () => profiles };
}

const CURRENT = { layout: "us-qwerty", language: "en", theme: "nord" };

describe("delete confirmation (S4)", () => {
  it("names the profile in the exact Spanish text", () => {
    expect(deleteConfirmation("Ana")).toBe("Se borrarán el historial y las estadísticas de «Ana». ¿Continuar?");
  });
});

describe("carry-over (S6)", () => {
  it("takes the stored valid layout, language and theme while the flag is unset", () => {
    const storage = memoryStorage({
      "plexustyping.layout": "us-dvorak",
      "plexustyping.language": "fr",
      "plexustyping.theme": "nord",
    });
    expect(carryOver(storage)).toEqual({ layout: "us-dvorak", language: "fr", theme: "nord" });
  });

  it("skips missing and unknown ids", () => {
    const storage = memoryStorage({ "plexustyping.layout": "xx-qwerty", "plexustyping.theme": "nope" });
    expect(carryOver(storage)).toEqual({});
    expect(carryOver(memoryStorage({ "plexustyping.language": "de" }))).toEqual({ language: "de" });
  });

  it("is null once the flag is set", () => {
    expect(carryOver(memoryStorage({ [MIGRATED_KEY]: "true", "plexustyping.layout": "us-dvorak" }))).toBeNull();
  });
});

describe("appearance of a profile (S5)", () => {
  it("applies the profile's layout, language and theme", () => {
    const p = profile(1, "Ana", { layout: "de-qwertz", language: "de", theme: "dracula" });
    expect(appearanceOf(p, CURRENT)).toEqual({
      appearance: { layout: "de-qwertz", language: "de", theme: "dracula" },
      saveTheme: false,
    });
  });

  it("keeps the current theme for a null or unknown one and asks to save it", () => {
    expect(appearanceOf(profile(1, "Ana"), CURRENT)).toEqual({
      appearance: { layout: "es-qwerty", language: "es", theme: "nord" },
      saveTheme: true,
    });
    expect(appearanceOf(profile(1, "Ana", { theme: "gone" }), CURRENT).saveTheme).toBe(true);
  });
});

describe("start (S5, S6)", () => {
  it("writes the carry-over once to the active profile, sets the flag and keeps localStorage", async () => {
    const { api, calls, rows } = fakeApi([profile(1, "Perfil"), profile(2, "Bea")], 2);
    const storage = memoryStorage({
      "plexustyping.layout": "us-dvorak",
      "plexustyping.language": "fr",
      "plexustyping.theme": "dracula",
    });
    const started = await startProfiles(api, storage, CURRENT);
    expect(calls).toEqual([["list"], ["update", 2, { layout: "us-dvorak", language: "fr", theme: "dracula" }]]);
    expect(started.active).toEqual(profile(2, "Bea", { layout: "us-dvorak", language: "fr", theme: "dracula" }));
    expect(started.profiles).toEqual(rows());
    expect(started.appearance).toEqual({ layout: "us-dvorak", language: "fr", theme: "dracula" });
    expect(storage.getItem(MIGRATED_KEY)).toBe("true");
    expect(storage.getItem("plexustyping.layout")).toBe("us-dvorak");
    expect(rows()[0]).toEqual(profile(1, "Perfil"));

    // A later start takes the three from the profile, without writing it again.
    storage.setItem("plexustyping.layout", "us-qwerty");
    calls.length = 0;
    const again = await startProfiles(api, storage, CURRENT);
    expect(calls).toEqual([["list"]]);
    expect(again.appearance.layout).toBe("us-dvorak");
  });

  it("saves the current theme to a profile without one", async () => {
    const { api, calls } = fakeApi([profile(1, "Perfil")]);
    const storage = memoryStorage({ [MIGRATED_KEY]: "true" });
    const started = await startProfiles(api, storage, CURRENT);
    expect(calls).toEqual([["list"], ["update", 1, { theme: "nord" }]]);
    expect(started.active.theme).toBe("nord");
    expect(started.appearance).toEqual({ layout: "es-qwerty", language: "es", theme: "nord" });
  });

  it("leaves the flag unset when the carry-over cannot be written", async () => {
    const { api } = fakeApi([profile(1, "Perfil")]);
    api.update = async () => {
      throw "Ese perfil no existe";
    };
    const storage = memoryStorage({ "plexustyping.layout": "us-dvorak" });
    await expect(startProfiles(api, storage, CURRENT)).rejects.toBe("Ese perfil no existe");
    expect(storage.getItem(MIGRATED_KEY)).toBeNull();
  });
});

describe("switch, create, rename, delete (S4, S5)", () => {
  it("switches and applies the profile; a null theme keeps the current one and is saved", async () => {
    const { api, calls } = fakeApi([profile(1, "Ana", { theme: "dracula" }), profile(2, "Bea", { layout: "pt-abnt2" })]);
    const switched = await switchProfile(api, 2, CURRENT);
    expect(calls).toEqual([["switch", 2], ["update", 2, { theme: "nord" }]]);
    expect(switched.active).toEqual(profile(2, "Bea", { layout: "pt-abnt2", theme: "nord" }));
    expect(switched.appearance).toEqual({ layout: "pt-abnt2", language: "es", theme: "nord" });
  });

  it("creates with the active profile's language and layout, without switching", async () => {
    const { api, calls } = fakeApi([profile(1, "Ana", { language: "de", layout: "de-qwertz" })]);
    const list = await createProfile(api, "Bea", profile(1, "Ana", { language: "de", layout: "de-qwertz" }));
    expect(calls).toEqual([["create", "Bea", "de", "de-qwertz"], ["list"]]);
    expect(list.active).toBe(1);
    expect(list.profiles.map((p) => p.name)).toEqual(["Ana", "Bea"]);
  });

  it("renames and returns the list with the renamed profile", async () => {
    const { api } = fakeApi([profile(1, "Ana"), profile(2, "Bea")]);
    const list = await renameProfile(api, 2, "Carla", [profile(1, "Ana"), profile(2, "Bea")]);
    expect(list.map((p) => p.name)).toEqual(["Ana", "Carla"]);
    await expect(renameProfile(api, 2, "ana", list)).rejects.toBe("Ya existe un perfil con ese nombre");
  });

  it("deleting another profile keeps the active one and applies nothing", async () => {
    const { api } = fakeApi([profile(1, "Ana"), profile(2, "Bea")]);
    const result = await deleteProfile(api, 2, 1, CURRENT);
    expect(result.profiles.map((p) => p.id)).toEqual([1]);
    expect(result.active.id).toBe(1);
    expect(result.appearance).toBeNull();
  });

  it("deleting the active profile applies the remaining one", async () => {
    const { api } = fakeApi([profile(1, "Ana", { theme: "dracula", layout: "fr-azerty" }), profile(2, "Bea")], 2);
    const result = await deleteProfile(api, 2, 2, CURRENT);
    expect(result.active.id).toBe(1);
    expect(result.appearance).toEqual({ layout: "fr-azerty", language: "es", theme: "dracula" });
  });

  it("passes refusals through and changes nothing", async () => {
    const { api, rows } = fakeApi([profile(1, "Ana")]);
    await expect(deleteProfile(api, 1, 1, CURRENT)).rejects.toBe("No se puede borrar el único perfil");
    expect(rows()).toEqual([profile(1, "Ana")]);
  });
});

describe("per-profile settings (S5)", () => {
  it("writes a layout, language or theme change to the active profile", async () => {
    const { api, calls } = fakeApi([profile(1, "Ana")]);
    const saved = await saveAppearance(api, 1, { layout: "us-colemak" });
    expect(calls).toEqual([["update", 1, { layout: "us-colemak" }]]);
    expect(saved.layout).toBe("us-colemak");
  });
});
