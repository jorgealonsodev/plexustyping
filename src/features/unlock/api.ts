import { invoke } from "@tauri-apps/api/core";

// Progressive unlock commands (S7), all for the active profile. Failures reject with the backend's Spanish message.

/** The beginner mode of one language and layout: whether it is on, its target speed and the unlock progress. */
export interface UnlockStatus {
  enabled: boolean;
  /** PPM a key must be typed at to count as mastered (20, 30, 40 or 50). */
  targetWpm: number;
  /** The chars that may appear in texts (the home row first, then one key at a time). */
  active: string[];
  mastered: string[];
  /** The key the next unlock adds; `null` once every letter is unlocked. */
  next: string | null;
  /** Letters of the full unlock order. */
  total: number;
  /** Sessions in a row without a newly mastered key (3 unlock the next one). */
  stalledSessions: number;
}

/** What `unlock_set` changes; a field left out keeps its stored value. */
export interface UnlockChange {
  enabled?: boolean;
  targetWpm?: number;
}

export const unlockStatus = (language: string, layout: string) =>
  invoke<UnlockStatus>("unlock_status", { language, layout });

/** Turns the mode on or off and/or sets the target speed; refuses other targets with "Velocidad objetivo no válida". */
export const unlockSet = (language: string, layout: string, change: UnlockChange) =>
  invoke<UnlockStatus>("unlock_set", {
    language,
    layout,
    enabled: change.enabled ?? null,
    targetWpm: change.targetWpm ?? null,
  });

/** The unlock commands, as an object so the settings store can run against a fake backend. */
export interface UnlockApi {
  status(language: string, layout: string): Promise<UnlockStatus>;
  set(language: string, layout: string, change: UnlockChange): Promise<UnlockStatus>;
}

export const unlockApi: UnlockApi = { status: unlockStatus, set: unlockSet };
