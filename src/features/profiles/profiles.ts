import { settingsStorage } from "../settings/persist";
import { LANGUAGES, LAYOUTS } from "../settings/settings";
import { isThemeId } from "../themes";
import type { Profile, ProfileFields, ProfilesApi, ProfilesList } from "./api";

// Profile logic (S4–S6) over the profile commands; the settings store applies the results.

/** Set once the pre-profile settings were carried over to the active profile (S6). */
export const MIGRATED_KEY = "plexustyping.profilesMigrated";

/** The pre-profile settings keys carried over (S6); the values stay in place. */
const LAYOUT_KEY = "plexustyping.layout";
const LANGUAGE_KEY = "plexustyping.language";
const THEME_KEY = "plexustyping.theme";

/** What a profile decides on screen: keyboard layout, prose language and theme id. */
export interface Appearance {
  layout: string;
  language: string;
  theme: string;
}

/** The confirmation shown before a profile and its data are deleted (S4). */
export const deleteConfirmation = (name: string) =>
  `Se borrarán el historial y las estadísticas de «${name}». ¿Continuar?`;

/** The stored valid layout, language and theme to write to the active profile; `null` once carried over. */
export function carryOver(storage: Storage = settingsStorage()): ProfileFields | null {
  if (storage.getItem(MIGRATED_KEY) !== null) return null;
  const fields: ProfileFields = {};
  const layout = storage.getItem(LAYOUT_KEY);
  if (LAYOUTS.some((l) => l.id === layout)) fields.layout = layout!;
  const language = storage.getItem(LANGUAGE_KEY);
  if (LANGUAGES.some((l) => l.id === language)) fields.language = language!;
  const theme = storage.getItem(THEME_KEY);
  if (theme !== null && isThemeId(theme)) fields.theme = theme;
  return fields;
}

/** The appearance to apply for `profile`; a missing or unknown theme keeps `current`'s and is to be saved. */
export function appearanceOf(profile: Profile, current: Appearance): { appearance: Appearance; saveTheme: boolean } {
  const ownTheme = profile.theme !== null && isThemeId(profile.theme);
  return {
    appearance: {
      layout: LAYOUTS.some((l) => l.id === profile.layout) ? profile.layout : current.layout,
      language: LANGUAGES.some((l) => l.id === profile.language) ? profile.language : current.language,
      theme: ownTheme ? profile.theme! : current.theme,
    },
    saveTheme: !ownTheme,
  };
}

const replace = (profiles: Profile[], next: Profile) => profiles.map((p) => (p.id === next.id ? next : p));

/** `profile` and what to apply, with a kept current theme saved to it (S5). */
async function settle(api: ProfilesApi, profile: Profile, current: Appearance) {
  const { appearance, saveTheme } = appearanceOf(profile, current);
  const active = saveTheme ? await api.update(profile.id, { theme: appearance.theme }) : profile;
  return { active, appearance };
}

/** Start: the active profile (after the one-time carry-over of S6), the list and what to apply. */
export async function startProfiles(api: ProfilesApi, storage: Storage, current: Appearance) {
  const listed = await api.list();
  let active = listed.profiles.find((p) => p.id === listed.active) ?? listed.profiles[0];
  const carried = carryOver(storage);
  if (carried) {
    if (Object.keys(carried).length > 0) active = await api.update(active.id, carried);
    storage.setItem(MIGRATED_KEY, "true");
  }
  const settled = await settle(api, active, current);
  return { profiles: replace(listed.profiles, settled.active), ...settled };
}

/** Makes `id` the active profile; its appearance is to be applied. */
export async function switchProfile(api: ProfilesApi, id: number, current: Appearance) {
  return settle(api, await api.switchTo(id), current);
}

/** Creates a profile with the active one's language and layout (S4); the active one stays. */
export async function createProfile(
  api: ProfilesApi,
  name: string,
  from: Pick<Profile, "language" | "layout">,
): Promise<ProfilesList> {
  await api.create(name, from.language, from.layout);
  return api.list();
}

/** `profiles` with `id` renamed. */
export async function renameProfile(api: ProfilesApi, id: number, name: string, profiles: Profile[]) {
  return replace(profiles, await api.update(id, { name }));
}

/** Deletes `id`; when it was the active one, the backend's new active profile is to be applied. */
export async function deleteProfile(api: ProfilesApi, id: number, activeId: number, current: Appearance) {
  const { active } = await api.remove(id);
  const listed = await api.list();
  if (active.id === activeId) return { profiles: listed.profiles, active, appearance: null };
  const settled = await settle(api, active, current);
  return { profiles: replace(listed.profiles, settled.active), ...settled };
}

/** Writes a layout, language or theme change to the active profile (S5). */
export const saveAppearance = (api: ProfilesApi, id: number, fields: ProfileFields) => api.update(id, fields);
