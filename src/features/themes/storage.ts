import { settingsStorage } from "../settings/persist";
import { DEFAULT_THEME_ID, isThemeId } from "./themes";

const KEY = "plexustyping.theme";

/** The saved theme id, or the default one when none (or an unknown one) is saved. */
export function loadThemeId(storage: Storage = settingsStorage()): string {
  const id = storage.getItem(KEY);
  return id !== null && isThemeId(id) ? id : DEFAULT_THEME_ID;
}

/** Remembers the chosen theme id. */
export function saveThemeId(id: string, storage: Storage = settingsStorage()): void {
  storage.setItem(KEY, id);
}
