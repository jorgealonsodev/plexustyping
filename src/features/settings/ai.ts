// AI settings types and pure helpers for the "IA" section (S2, S3).

export type ProviderId = "claude" | "openai" | "gemini" | "opencode-go";

export interface ChainEntry {
  provider: ProviderId;
  /** Empty: not chosen yet (the backend skips the row). */
  model: string;
}

/** `plexustyping-app` `AiSettings`: non-secret, stored in the local database. */
export interface AiSettings {
  enabled: boolean;
  chain: ChainEntry[];
  density: number;
  topic: string | null;
}

export interface ProviderInfo {
  id: ProviderId;
  label: string;
  name: string;
  defaultModel: string;
}

export interface UsageRow {
  provider: string;
  callsToday: number;
  callsTotal: number;
  inputTokensToday: number;
  outputTokensToday: number;
  inputTokensTotal: number;
  outputTokensTotal: number;
}

/** A failed AI command: `{ kind, message }`, message in Spanish. */
export interface AiFailure {
  kind: string;
  message: string;
}

export const MAX_CHAIN = 4;
export const DENSITY_MIN = 2;
export const DENSITY_MAX = 5;
export const MAX_TOPIC = 60;

const used = (chain: readonly ChainEntry[], except = -1) => new Set(chain.filter((_, i) => i !== except).map((e) => e.provider));

/** The chain with the first unused provider appended (default model), or `null` when full. */
export function addEntry(chain: readonly ChainEntry[], providers: readonly ProviderInfo[]): ChainEntry[] | null {
  if (chain.length >= MAX_CHAIN) return null;
  const taken = used(chain);
  const next = providers.find((p) => !taken.has(p.id));
  return next ? [...chain, { provider: next.id, model: next.defaultModel }] : null;
}

/** The chain without row `i`; the last row is never removed. */
export function removeEntry(chain: readonly ChainEntry[], i: number): ChainEntry[] {
  if (chain.length <= 1 || i < 0 || i >= chain.length) return [...chain];
  return chain.filter((_, j) => j !== i);
}

/** The chain with row `i` moved by `delta` (−1 up, +1 down), unchanged at the edges. */
export function moveEntry(chain: readonly ChainEntry[], i: number, delta: number): ChainEntry[] {
  const j = i + delta;
  const next = [...chain];
  if (i < 0 || i >= chain.length || j < 0 || j >= chain.length) return next;
  [next[i], next[j]] = [next[j], next[i]];
  return next;
}

/** Row `i` switched to provider `id` with its default model; unchanged if another row has it. */
export function setProvider(
  chain: readonly ChainEntry[],
  i: number,
  id: ProviderId,
  providers: readonly ProviderInfo[],
): ChainEntry[] {
  if (chain[i]?.provider === id || used(chain, i).has(id)) return [...chain];
  const info = providers.find((p) => p.id === id);
  return chain.map((e, j) => (j === i ? { provider: id, model: info?.defaultModel ?? "" } : e));
}

export function setModel(chain: readonly ChainEntry[], i: number, model: string): ChainEntry[] {
  return chain.map((e, j) => (j === i ? { ...e, model: model.trim() } : e));
}

/** Providers row `i` may use: its own plus those no other row has. */
export function providerChoices(chain: readonly ChainEntry[], i: number, providers: readonly ProviderInfo[]): ProviderInfo[] {
  const taken = used(chain, i);
  return providers.filter((p) => !taken.has(p.id));
}

/** Listed models, with the current one first when the list lacks it. */
export function modelChoices(listed: readonly string[], current: string): string[] {
  return current && !listed.includes(current) ? [current, ...listed] : [...listed];
}

export const densityLabel = (d: number) => `${String(d).replace(".", ",")}×`;

/** The message of a typed AI failure, or the error as text. */
export function failureMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err && typeof err.message === "string" && "kind" in err) {
    return err.message;
  }
  return String(err);
}

/** One usage table row: provider name, calls and tokens (prompt + completion). */
export function usageCells(row: UsageRow, providers: readonly ProviderInfo[]) {
  return {
    name: providers.find((p) => p.id === row.provider)?.name ?? row.provider,
    calls: row.callsTotal,
    today: row.inputTokensToday + row.outputTokensToday,
    total: row.inputTokensTotal + row.outputTokensTotal,
  };
}
