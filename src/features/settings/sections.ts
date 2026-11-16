// Pure helpers for the settings modal (S8); the section list itself lives in `registry.ts`.

/** The next section id for a tab-list navigation key, or `null` when the key does not move. */
export function moveSection(ids: readonly string[], current: string, key: string): string | null {
  const at = ids.indexOf(current);
  if (at < 0) return null;
  switch (key) {
    case "ArrowDown":
      return ids[(at + 1) % ids.length];
    case "ArrowUp":
      return ids[(at - 1 + ids.length) % ids.length];
    case "Home":
      return ids[0];
    case "End":
      return ids[ids.length - 1];
    default:
      return null;
  }
}

type ShortcutKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat">;

/** Ctrl+, opens (or closes) the settings modal; only plain Ctrl counts, and never a held key. */
export const isSettingsShortcut = (e: ShortcutKey) =>
  e.key === "," && e.ctrlKey && !e.altKey && !e.metaKey && !e.shiftKey && !e.repeat;
