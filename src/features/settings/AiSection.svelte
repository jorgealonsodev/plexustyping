<script lang="ts">
  import { onMount } from "svelte";
  import { localDay } from "../practice/prefetch";
  import {
    type AiSettings,
    type ChainEntry,
    type ChatgptStatus,
    type ModelOption,
    CHATGPT_USAGE_URL,
    type ProviderId,
    type ProviderInfo,
    type UsageRow,
    DENSITY_MAX,
    DENSITY_MIN,
    MAX_CHAIN,
    MAX_TOPIC,
    addEntry,
    chatgptAccount,
    chatgptReady,
    densityLabel,
    failureMessage,
    fillMissingModel,
    modelOptions,
    moveEntry,
    needsKey,
    providerChoices,
    removeEntry,
    setModel,
    setProvider,
    signOutNotice,
    usageCells,
  } from "./ai";
  import {
    aiKeyDelete,
    aiKeySave,
    aiChatgptCancel,
    aiChatgptOpenUsage,
    aiChatgptSignIn,
    aiChatgptSignOut,
    aiChatgptStatus,
    aiKeyStatus,
    aiModelOptions,
    aiProviders,
    aiSettingsGet,
    aiSettingsSet,
    aiTest,
    aiUsage,
  } from "./aiApi";
  import { formatNumber } from "../../i18n/i18n.svelte";
  import { t } from "./aiMessages";
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  // "IA" settings (S2, S3); texts in `aiMessages.ts`. Keys typed here go once to `ai_key_save`, read straight from the
  // form field, which is then cleared; they are never kept in component state.
  let { settings }: { settings: SettingsStore } = $props();

  type KeyState = "saved" | "none" | "unavailable" | "unknown";
  type TestState = { ok: boolean; message: string } | "pending";

  let ai = $state.raw<AiSettings | null>(null);
  let providers = $state.raw<ProviderInfo[]>([]);
  let error = $state("");
  let keyState: Partial<Record<ProviderId, KeyState>> = $state({});
  let keyError: Partial<Record<ProviderId, string>> = $state({});
  // Listed models per provider (shown by name, stored by id); `null` when listing failed (free text instead).
  let models: Partial<Record<ProviderId, ModelOption[] | null>> = $state({});
  // Rows whose model is typed by hand.
  let custom: Partial<Record<ProviderId, boolean>> = $state({});
  let tests: Partial<Record<ProviderId, TestState>> = $state({});
  let usage = $state.raw<UsageRow[]>([]);
  // Sign in with ChatGPT: the account (never a token), a sign-in waiting for the browser,
  // and the last error or notice.
  let chatgpt = $state.raw<ChatgptStatus | null>(null);
  let signingIn = $state(false);
  let chatgptError = $state("");
  let chatgptNotice = $state("");
  let density = $state(3);

  const keyLabel = (state: KeyState) =>
    state === "saved" ? t("keySaved") : state === "none" ? t("keyNone") : state === "unavailable" ? t("keyUnavailable") : "…";
  const OTHER = "__other__";

  const nameOf = (id: ProviderId) => providers.find((p) => p.id === id)?.name ?? id;

  let root = $state<HTMLElement>();

  // The modal mounts every section at startup; the keychain (which may ask to be unlocked) and
  // the providers are only asked once this section is actually shown.
  onMount(() => {
    const loaded = Promise.all([aiSettingsGet(), aiProviders()]).then(([s, list]) => {
      providers = list;
      apply(s);
    });
    loaded.catch((err) => (error = failureMessage(err)));
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      void loaded.then(() => {
        for (const e of ai?.chain ?? []) refreshProvider(e.provider, models[e.provider] === undefined);
      });
      refreshUsage();
    });
    if (root) observer.observe(root);
    return () => observer.disconnect();
  });

  function apply(s: AiSettings) {
    ai = s;
    density = s.density;
    settings.aiEnabled = s.enabled;
  }

  // The header "IA" toggle saves `enabled` itself: show it here too.
  $effect(() => {
    const enabled = settings.aiEnabled;
    if (ai && enabled !== null && ai.enabled !== enabled) ai = { ...ai, enabled };
  });

  /** Validated by the backend; on failure the previous settings stay on screen. */
  async function commit(next: AiSettings) {
    try {
      apply(await aiSettingsSet(next));
      error = "";
      settings.bumpAi();
    } catch (err) {
      error = failureMessage(err);
      if (ai) apply({ ...ai });
    }
  }

  const commitChain = (chain: ChainEntry[]) => ai && void commit({ ...ai, chain });

  function refreshProvider(id: ProviderId, listModels = true) {
    if (id === "chatgpt") {
      refreshChatgpt(listModels);
      return;
    }
    aiKeyStatus(id)
      .then((saved) => {
        keyState[id] = saved ? "saved" : "none";
      })
      .catch(() => {
        keyState[id] = "unavailable";
      });
    if (listModels) listModelsOf(id);
  }

  function listModelsOf(id: ProviderId) {
    aiModelOptions(id)
      .then((list) => {
        models[id] = list.length > 0 ? list : null;
        // A provider without a default model gets its first listed one.
        const filled = ai ? fillMissingModel(ai.chain, id, list) : null;
        if (filled) commitChain(filled);
      })
      .catch(() => {
        models[id] = null;
      });
  }

  /** The account; its models are listed when it is usable (signed in with plan use). */
  function refreshChatgpt(listModels = true) {
    aiChatgptStatus()
      .then((s) => {
        chatgpt = s;
        keyState.chatgpt = chatgptReady(s) ? "saved" : "none";
        if (!listModels) return;
        if (chatgptReady(s)) listModelsOf("chatgpt");
        else models.chatgpt = null;
      })
      .catch((err) => {
        chatgptError = failureMessage(err);
      });
  }

  async function signIn() {
    signingIn = true;
    chatgptError = "";
    chatgptNotice = "";
    try {
      chatgpt = await aiChatgptSignIn();
      settings.bumpAi();
    } catch (err) {
      chatgptError = failureMessage(err);
    } finally {
      signingIn = false;
    }
    refreshChatgpt();
  }

  async function signOut() {
    chatgptError = "";
    chatgptNotice = "";
    try {
      chatgptNotice = signOutNotice(await aiChatgptSignOut());
      settings.bumpAi();
    } catch (err) {
      chatgptError = failureMessage(err);
    }
    refreshChatgpt();
  }

  function openUsage(e: MouseEvent) {
    e.preventDefault();
    aiChatgptOpenUsage().catch((err) => {
      chatgptError = failureMessage(err);
    });
  }

  function refreshUsage() {
    aiUsage(localDay())
      .then((rows) => (usage = rows))
      .catch((err) => console.warn("ai_usage failed", err));
  }

  async function saveKey(e: SubmitEvent, id: ProviderId) {
    e.preventDefault();
    const form = e.currentTarget as HTMLFormElement;
    const field = form.elements.namedItem("key") as HTMLInputElement;
    const key = field.value;
    field.value = "";
    try {
      await aiKeySave(id, key);
      keyError[id] = "";
      settings.bumpAi();
    } catch (err) {
      keyError[id] = failureMessage(err);
    }
    refreshProvider(id);
  }

  async function deleteKey(id: ProviderId) {
    try {
      await aiKeyDelete(id);
      keyError[id] = "";
      settings.bumpAi();
    } catch (err) {
      keyError[id] = failureMessage(err);
    }
    refreshProvider(id);
  }

  async function test(entry: ChainEntry) {
    const id = entry.provider;
    tests[id] = "pending";
    try {
      await aiTest(id, entry.model);
      tests[id] = { ok: true, message: t("connectionOk") };
    } catch (err) {
      tests[id] = { ok: false, message: failureMessage(err) };
    }
  }

  function onProvider(i: number, id: ProviderId) {
    if (!ai) return;
    commitChain(setProvider(ai.chain, i, id, providers));
    refreshProvider(id);
  }

  function onModelSelect(i: number, id: ProviderId, value: string) {
    if (!ai) return;
    if (value === OTHER) {
      custom[id] = true;
      return;
    }
    commitChain(setModel(ai.chain, i, value));
  }

  function add() {
    if (!ai) return;
    const next = addEntry(ai.chain, providers);
    if (!next) return;
    commitChain(next);
    refreshProvider(next[next.length - 1].provider);
  }

  function onTopic(value: string) {
    if (!ai) return;
    const topic = value.trim();
    void commit({ ...ai, topic: topic === "" ? null : topic });
  }

  type Change<T extends HTMLElement> = Event & { currentTarget: T };
</script>

<div class="ai" bind:this={root}>
{#if ai}
  <SettingField toggle label={t("enabledLabel")} hint={t("enabledHint")}>
    {#snippet children({ id, hintId })}
      <input
        {id}
        type="checkbox"
        aria-describedby={hintId}
        checked={ai?.enabled}
        onchange={(e: Change<HTMLInputElement>) => ai && void commit({ ...ai, enabled: e.currentTarget.checked })}
      />
    {/snippet}
  </SettingField>

  <p class="privacy">{t("privacy")}</p>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <section class="chain" aria-label={t("chain")}>
    <h3>{t("chain")}</h3>
    <p class="hint">{t("chainHint")}</p>
    <ol>
      {#each ai.chain as entry, i (entry.provider)}
        {@const listed = models[entry.provider]}
        {@const typed = custom[entry.provider] || listed === null || listed === undefined}
        {@const test_ = tests[entry.provider]}
        <li>
          <div class="row">
            <span class="order">{i + 1}.</span>
            <select
              aria-label={t("providerN", { n: i + 1 })}
              onchange={(e: Change<HTMLSelectElement>) => onProvider(i, e.currentTarget.value as ProviderId)}
            >
              {#each providerChoices(ai.chain, i, providers) as p (p.id)}
                <option value={p.id} selected={p.id === entry.provider}>{p.label}</option>
              {/each}
            </select>
            {#if typed}
              <input
                class="model"
                type="text"
                aria-label={t("modelOf", { name: nameOf(entry.provider) })}
                placeholder={t("modelPlaceholder")}
                value={entry.model}
                onchange={(e: Change<HTMLInputElement>) => ai && commitChain(setModel(ai.chain, i, e.currentTarget.value))}
              />
            {:else}
              <select
                class="model"
                aria-label={t("modelOf", { name: nameOf(entry.provider) })}
                onchange={(e: Change<HTMLSelectElement>) => onModelSelect(i, entry.provider, e.currentTarget.value)}
              >
                {#if !entry.model}<option value="" selected>{t("chooseModel")}</option>{/if}
                {#each modelOptions(listed ?? [], entry.model) as m (m.id)}
                  <option value={m.id} selected={m.id === entry.model}>{m.name}</option>
                {/each}
                <option value={OTHER}>{t("otherModel")}</option>
              </select>
            {/if}
            <span class="moves">
              <button type="button" aria-label={t("moveUp")} title={t("moveUp")} disabled={i === 0} onclick={() => ai && commitChain(moveEntry(ai.chain, i, -1))}>↑</button>
              <button
                type="button"
                aria-label={t("moveDown")}
                title={t("moveDown")}
                disabled={i === ai.chain.length - 1}
                onclick={() => ai && commitChain(moveEntry(ai.chain, i, 1))}>↓</button
              >
              <button
                type="button"
                aria-label={t("remove")}
                title={t("remove")}
                disabled={ai.chain.length === 1}
                onclick={() => ai && commitChain(removeEntry(ai.chain, i))}>×</button
              >
            </span>
          </div>
          {#if needsKey(entry.provider)}
            <form class="row" onsubmit={(e) => void saveKey(e, entry.provider)}>
              <input name="key" type="password" autocomplete="off" placeholder={t("apiKeyPlaceholder")} aria-label={t("keyOf", { name: nameOf(entry.provider) })} />
              <button type="submit">{t("saveKey")}</button>
              <button type="button" onclick={() => void deleteKey(entry.provider)}>{t("deleteKey")}</button>
              <span class="status" class:ok={keyState[entry.provider] === "saved"}>{keyLabel(keyState[entry.provider] ?? "unknown")}</span>
            </form>
            {#if keyError[entry.provider]}<p class="error" role="alert">{keyError[entry.provider]}</p>{/if}
          {:else}
            <div class="account" aria-label={t("chatgptAccount")}>
              {#if chatgpt}
                {@const acc = chatgptAccount(chatgpt)}
                <div class="row">
                  <span class="status" class:ok={acc.ok}>{acc.account}</span>
                  {#if acc.plan}<span class="status" class:ok={acc.ok} class:bad={!acc.ok}>{acc.plan}</span>{/if}
                </div>
              {/if}
              <div class="row">
                {#if signingIn}
                  <span class="status" role="status">{t("waitingBrowser")}</span>
                  <button type="button" onclick={() => void aiChatgptCancel()}>{t("cancel")}</button>
                {:else if chatgpt?.signedIn}
                  <button type="button" onclick={() => void signOut()}>{t("signOut")}</button>
                {:else}
                  <button type="button" onclick={() => void signIn()}>{t("signIn")}</button>
                {/if}
                <a href={CHATGPT_USAGE_URL} onclick={openUsage}>{t("manageUsage")}</a>
              </div>
              <p class="hint">{t("chatgptPlan")}</p>
              {#if chatgptNotice}<p class="error" role="status">{chatgptNotice}</p>{/if}
              {#if chatgptError}<p class="error" role="alert">{chatgptError}</p>{/if}
            </div>
          {/if}
          <div class="row">
            <button type="button" disabled={test_ === "pending"} onclick={() => void test(entry)}>{t("testConnection")}</button>
            {#if test_ === "pending"}
              <span class="status">{t("testing")}</span>
            {:else if test_}
              <span class="status" class:ok={test_.ok} class:bad={!test_.ok} role="status">{test_.message}</span>
            {/if}
          </div>
        </li>
      {/each}
    </ol>
    <button type="button" disabled={ai.chain.length >= MAX_CHAIN || ai.chain.length >= providers.length} onclick={add}>
      {t("addProvider")}
    </button>
  </section>

  <SettingField label={t("densityLabel", { value: densityLabel(density) })} hint={t("densityHint")}>
    {#snippet children({ id, hintId })}
      <input
        {id}
        type="range"
        min={DENSITY_MIN}
        max={DENSITY_MAX}
        step="0.5"
        aria-describedby={hintId}
        value={density}
        oninput={(e: Change<HTMLInputElement>) => (density = Number(e.currentTarget.value))}
        onchange={(e: Change<HTMLInputElement>) => ai && void commit({ ...ai, density: Number(e.currentTarget.value) })}
      />
    {/snippet}
  </SettingField>

  <SettingField label={t("topicLabel")} hint={t("topicHint")}>
    {#snippet children({ id, hintId })}
      <input
        {id}
        type="text"
        maxlength={MAX_TOPIC}
        placeholder={t("topicPlaceholder")}
        aria-describedby={hintId}
        value={ai?.topic ?? ""}
        onchange={(e: Change<HTMLInputElement>) => onTopic(e.currentTarget.value)}
      />
    {/snippet}
  </SettingField>

  <section class="usage" aria-label={t("usageLabel")}>
    <div class="usage-head">
      <h3>{t("usage")}</h3>
      <button type="button" onclick={refreshUsage}>{t("refresh")}</button>
    </div>
    {#if usage.length === 0}
      <p class="hint">{t("noCalls")}</p>
    {:else}
      <table>
        <thead><tr><th>{t("provider")}</th><th>{t("calls")}</th><th>{t("tokensToday")}</th><th>{t("tokensTotal")}</th></tr></thead>
        <tbody>
          {#each usage as row (row.provider)}
            {@const c = usageCells(row, providers)}
            <tr><td>{c.name}</td><td>{formatNumber(c.calls)}</td><td>{formatNumber(c.today)}</td><td>{formatNumber(c.total)}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </section>
{:else if error}
  <p class="error" role="alert">{error}</p>
{:else}
  <p class="hint">{t("loading")}</p>
{/if}
</div>

<style>
  h3 {
    margin: 0.6rem 0 0.15rem;
    font-size: 0.95rem;
  }
  .hint,
  .privacy {
    margin: 0 0 0.4rem;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .privacy {
    margin-top: 0.4rem;
  }
  .error {
    margin: 0.25rem 0;
    font-size: 0.85rem;
    color: var(--danger);
  }
  .chain {
    padding-bottom: 0.6rem;
    border-bottom: 1px solid var(--border);
  }
  ol {
    margin: 0 0 0.5rem;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.5rem 0;
    border-bottom: 1px dashed var(--border);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
  }
  .order {
    min-width: 1.2rem;
    font-weight: 600;
  }
  .moves {
    display: flex;
    gap: 0.2rem;
    margin-left: auto;
  }
  button {
    padding: 0.25rem 0.6rem;
    font-size: 0.85rem;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  select,
  input[type="text"],
  input[type="password"] {
    min-width: 0;
    padding: 0.25rem 0.45rem;
    font: inherit;
    font-size: 0.9rem;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  .model {
    flex: 1 1 9rem;
  }
  input[type="password"] {
    flex: 1 1 8rem;
  }
  input[type="range"] {
    width: 10rem;
    accent-color: var(--accent);
  }
  .status {
    font-size: 0.85rem;
    color: var(--muted);
  }
  .status.ok {
    color: var(--success);
  }
  .status.bad {
    color: var(--danger);
  }
  .account {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .account a {
    font-size: 0.85rem;
    color: var(--accent);
  }
  .account .hint {
    margin: 0;
  }
  .usage-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  th,
  td {
    padding: 0.25rem 0.4rem;
    text-align: right;
    border-bottom: 1px solid var(--border);
  }
  th:first-child,
  td:first-child {
    text-align: left;
  }
  th {
    color: var(--muted);
    font-weight: 600;
  }
</style>
