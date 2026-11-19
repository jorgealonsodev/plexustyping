// AI settings types and pure helpers for the "IA" section (S2, S3).

export type ProviderId = "claude" | "openai" | "gemini" | "opencode-go" | "chatgpt";

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

/** One listed model: `id` is stored and sent, `name` is shown (ChatGPT's display name). */
export interface ModelOption {
  id: string;
  name: string;
}

/** The Sign in with ChatGPT account (`ai_chatgpt_status`); never a token. */
export interface ChatgptStatus {
  signedIn: boolean;
  email: string | null;
  planEnabled: boolean;
}

/** `ai_chatgpt_sign_out`: `revoked` is false when OpenAI could not confirm it. */
export interface ChatgptSignOut {
  revoked: boolean;
}

/** Every provider once (backend `MAX_CHAIN`). */
export const MAX_CHAIN = 5;
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

/**
 * Rows of provider `id` without a model get its first listed model (providers such as OpenCode Go
 * and ChatGPT have no default); `null` when nothing changes.
 */
export function fillMissingModel(
  chain: readonly ChainEntry[],
  id: ProviderId,
  list: readonly ModelOption[],
): ChainEntry[] | null {
  const first = list[0]?.id;
  if (!first || !chain.some((e) => e.provider === id && e.model.trim() === "")) return null;
  return chain.map((e) => (e.provider === id && e.model.trim() === "" ? { ...e, model: first } : e));
}

export function setModel(chain: readonly ChainEntry[], i: number, model: string): ChainEntry[] {
  return chain.map((e, j) => (j === i ? { ...e, model: model.trim() } : e));
}

/** Providers row `i` may use: its own plus those no other row has. */
export function providerChoices(chain: readonly ChainEntry[], i: number, providers: readonly ProviderInfo[]): ProviderInfo[] {
  const taken = used(chain, i);
  return providers.filter((p) => !taken.has(p.id));
}

/** Listed models (shown by name, stored by id), with the current one first when the list lacks it. */
export function modelOptions(listed: readonly ModelOption[], current: string): ModelOption[] {
  return current && !listed.some((m) => m.id === current) ? [{ id: current, name: current }, ...listed] : [...listed];
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

/** Title of the header "IA" toggle (ai-toggle S1). */
export const aiToggleTitle = (enabled: boolean) =>
  enabled ? "IA activada: los textos nuevos se generan con IA" : "IA desactivada: textos guardados";

/** `s` with the AI turned on or off, everything else kept. */
export const toggleEnabled = (s: AiSettings): AiSettings => ({ ...s, enabled: !s.enabled });

/** ChatGPT uses the signed-in account instead of an API key. */
export const needsKey = (id: ProviderId) => id !== "chatgpt";

/** The ChatGPT row is usable: signed in with plan use enabled (as a saved key for the others). */
export const chatgptReady = (s: ChatgptStatus | null | undefined) => !!s && s.signedIn && s.planEnabled;

/** The account lines of the ChatGPT block; `ok` when the row is usable. */
export function chatgptAccount(s: ChatgptStatus): { account: string; plan: string; ok: boolean } {
  if (!s.signedIn) return { account: "Sin sesión iniciada", plan: "", ok: false };
  return {
    account: s.email ? `Sesión iniciada como ${s.email}` : "Sesión iniciada",
    plan: s.planEnabled
      ? "El uso del plan está activado"
      : "El uso del plan no está activado: inicia sesión de nuevo y permítelo",
    ok: s.planEnabled,
  };
}

/** What to tell after signing out; empty when the revocation was confirmed. */
export const signOutNotice = (r: ChatgptSignOut) => (r.revoked ? "" : "No se pudo confirmar la revocación");

/** OpenAI's page with the ChatGPT plan usage ("Gestionar uso"). */
export const CHATGPT_USAGE_URL = "https://chatgpt.com/settings/usage";
