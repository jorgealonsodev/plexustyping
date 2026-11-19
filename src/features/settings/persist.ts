import { invoke } from "@tauri-apps/api/core";

// UI settings live in the app's SQLite store (S1, S2): read once before the app mounts,
// then served synchronously from memory; each change is written to the store at once.

/** Prefix of every UI setting key. */
export const SETTINGS_PREFIX = "plexustyping.";
/** Written with the first start's migration, so later starts never import localStorage again (S3). */
export const MIGRATED_KEY = "plexustyping.settingsInStore";

/** The store commands behind the UI settings. */
export interface SettingsBackend {
  get(): Promise<Record<string, string>>;
  set(key: string, value: string): Promise<void>;
  remove(key: string): Promise<void>;
}

export const tauriSettingsBackend: SettingsBackend = {
  get: () => invoke<Record<string, string>>("ui_settings_get"),
  set: (key, value) => invoke<void>("ui_settings_set", { key, value }),
  remove: (key) => invoke<void>("ui_settings_remove", { key }),
};

type Log = (message: string, err: unknown) => void;
const logError: Log = (message, err) => console.error(message, err);

/** A `Storage` over an in-memory snapshot of the store: reads are synchronous; writes update the
 * snapshot at once and go to the store one at a time, in order. A failed write is logged and the
 * in-memory value stays (S5). */
export class StoreStorage implements Storage {
  [name: string]: unknown;
  readonly #values: Map<string, string>;
  readonly #backend: SettingsBackend;
  readonly #log: Log;
  #queue: Promise<void> = Promise.resolve();
  #failures = 0;

  constructor(initial: Record<string, string>, backend: SettingsBackend, log: Log = logError) {
    this.#values = new Map(Object.entries(initial));
    this.#backend = backend;
    this.#log = log;
  }

  get length(): number {
    return this.#values.size;
  }

  key(index: number): string | null {
    return [...this.#values.keys()][index] ?? null;
  }

  getItem(key: string): string | null {
    return this.#values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    const text = String(value);
    this.#values.set(key, text);
    this.#enqueue(`setting ${key} not saved`, () => this.#backend.set(key, text));
  }

  removeItem(key: string): void {
    this.#values.delete(key);
    this.#enqueue(`setting ${key} not removed`, () => this.#backend.remove(key));
  }

  clear(): void {
    for (const key of [...this.#values.keys()]) this.removeItem(key);
  }

  /** Resolves once every write made so far has finished (or failed). */
  flush(): Promise<void> {
    return this.#queue;
  }

  /** Writes that failed so far. */
  get failures(): number {
    return this.#failures;
  }

  #enqueue(failure: string, write: () => Promise<void>) {
    this.#queue = this.#queue.then(write).catch((err) => {
      this.#failures++;
      this.#log(failure, err);
    });
  }
}

let current: Storage | null = null;

/** Where the UI settings are kept: the store once {@link initSettingsStorage} ran, else localStorage. */
export const settingsStorage = (): Storage => current ?? localStorage;

/** Every `plexustyping.*` value in `storage`. */
function uiValues(storage: Storage): Record<string, string> {
  const values: Record<string, string> = {};
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    const value = key === null ? null : storage.getItem(key);
    if (key !== null && key.startsWith(SETTINGS_PREFIX) && value !== null) values[key] = value;
  }
  return values;
}

/**
 * Start-up, before the app mounts (S4): loads the stored UI settings. Until the migration marker is
 * stored, every `plexustyping.*` key of `local` the store lacks is copied into it, leaving `local` as
 * is; the marker follows once every copy landed, so an interrupted or failed copy is retried at the
 * next start (S3). When the store cannot be read, `local` is used as before and the error logged (S5).
 */
export async function initSettingsStorage(
  backend: SettingsBackend = tauriSettingsBackend,
  local: Storage = localStorage,
  log: Log = logError,
): Promise<Storage> {
  let stored: Record<string, string>;
  try {
    stored = await backend.get();
  } catch (err) {
    log("settings store unavailable, using localStorage", err);
    current = local;
    return local;
  }
  const storage = new StoreStorage(stored, backend, log);
  // Until the marker is stored, each start copies the keys the store still lacks (the store's own
  // values win): a first start closed early, or a copy that failed, is completed later. The marker
  // is written only once every copy landed.
  if (!(MIGRATED_KEY in stored)) {
    for (const [key, value] of Object.entries(uiValues(local))) {
      if (!(key in stored)) storage.setItem(key, value);
    }
    const before = storage.failures;
    void storage.flush().then(() => {
      if (storage.failures === before) storage.setItem(MIGRATED_KEY, "true");
    });
  }
  current = storage;
  return storage;
}
