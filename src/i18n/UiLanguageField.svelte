<script lang="ts">
  import SettingField from "../features/settings/SettingField.svelte";
  import {
    LOCALES,
    LOCALE_NAMES,
    type UiLanguage,
    detectLocale,
    loadUiLanguage,
    resolveLocale,
    saveUiLanguage,
    systemLanguages,
    ui,
  } from "./i18n.svelte";
  import { t } from "./messages";

  // Interface language (ui-i18n S3, S4): apart from the practice-text language, applied live.
  let choice = $state<UiLanguage>(loadUiLanguage());
  const detected = LOCALE_NAMES[detectLocale(systemLanguages())];

  function choose(e: Event & { currentTarget: HTMLSelectElement }) {
    choice = e.currentTarget.value as UiLanguage;
    saveUiLanguage(choice);
    ui.set(resolveLocale(choice));
  }
</script>

<SettingField label={t("label")} hint={t("hint")}>
  {#snippet children({ id, hintId })}
    <select {id} aria-describedby={hintId} onchange={choose}>
      <option value="system" selected={choice === "system"}>{t("system", { name: detected })}</option>
      {#each LOCALES as l (l)}
        <option value={l} lang={l} selected={choice === l}>{LOCALE_NAMES[l]}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>
