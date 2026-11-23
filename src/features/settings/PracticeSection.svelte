<script lang="ts">
  import type { ErrorMode } from "../practice/engine";
  import SettingField from "./SettingField.svelte";
  import { t } from "./messages";
  import { ERROR_MODES, GHOST_SETTING, LANGUAGES, TARGET_COUNTS, TEXT_LENGTHS, lengthLabel } from "./settings";
  import type { SettingsStore } from "./store.svelte";
  import type { UnlockChange } from "../unlock/api";
  import { DEFAULT_TARGET_WPM, TARGET_SETTING, TARGET_WPMS, UNLOCK_SETTING } from "../unlock/unlock";

  let { settings }: { settings: SettingsStore } = $props();

  /** The last refused unlock change (from the backend, in the interface language); cleared by the next one that works. */
  let unlockError = $state<string | null>(null);
  const targetWpm = $derived(settings.unlock?.targetWpm ?? DEFAULT_TARGET_WPM);

  async function setUnlock(change: UnlockChange) {
    unlockError = await settings.setUnlock(change);
  }

  type Change<T extends HTMLElement> = Event & { currentTarget: T };
</script>

<SettingField label={t("languageLabel")} hint={t("languageHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setLanguage(e.currentTarget.value)}
    >
      {#each LANGUAGES as l (l.id)}
        <option value={l.id} selected={l.id === settings.language}>{l.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label={t("errorModeLabel")} hint={t("errorModeHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setMode(e.currentTarget.value as ErrorMode)}
    >
      {#each ERROR_MODES as m (m.id)}
        <option value={m.id} selected={m.id === settings.mode}>{m.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label={t("targetsLabel")} hint={t("targetsHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setTargetCount(Number(e.currentTarget.value))}
    >
      {#each TARGET_COUNTS as n (n)}
        <option value={n} selected={n === settings.targetCount}>{n}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label={t("lengthLabel")} hint={t("lengthHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setTextLength(Number(e.currentTarget.value))}
    >
      {#each TEXT_LENGTHS as n (n)}
        <option value={n} selected={n === settings.textLength}>{lengthLabel(n)}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField toggle label={t("liveStatsLabel")} hint={t("liveStatsHint")}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.liveStats}
      onchange={(e: Change<HTMLInputElement>) => settings.setLiveStats(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<SettingField toggle label={GHOST_SETTING.label} hint={GHOST_SETTING.hint}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.ghost}
      onchange={(e: Change<HTMLInputElement>) => settings.setGhost(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<!-- Progressive unlock (S1): per profile, language and layout; prose texts only. -->
<SettingField toggle label={UNLOCK_SETTING.label} hint={UNLOCK_SETTING.hint}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.unlock?.enabled ?? false}
      disabled={settings.unlock === null}
      onchange={(e: Change<HTMLInputElement>) => void setUnlock({ enabled: e.currentTarget.checked })}
    />
  {/snippet}
</SettingField>

<SettingField label={TARGET_SETTING.label} hint={TARGET_SETTING.hint}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      disabled={settings.unlock === null}
      onchange={(e: Change<HTMLSelectElement>) => void setUnlock({ targetWpm: Number(e.currentTarget.value) })}
    >
      {#each TARGET_WPMS as n (n)}
        <option value={n} selected={n === targetWpm}>{t("wpm", { n })}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>
{#if unlockError}<p class="error" role="alert">{unlockError}</p>{/if}

<style>
  .error {
    margin: 0.4rem 0 0;
    font-size: 0.85rem;
    color: var(--danger);
  }
</style>
