<script lang="ts">
  import { onMount } from "svelte";
  import { LANGUAGES } from "../code/languages.svelte";
  import type { IndentMode } from "../practice/engine";
  import SettingField from "./SettingField.svelte";
  import { t } from "./messages";
  import { CODE_INDENTS, MAX_CODE_LINES, MIN_CODE_LINES } from "./settings";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change<T extends HTMLElement> = Event & { currentTarget: T };

  onMount(() => void LANGUAGES.load());
</script>

<SettingField label={t("codeLanguageLabel")} hint={t("codeLanguageHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setCodeLanguage(e.currentTarget.value)}
    >
      {#each LANGUAGES.list as l (l.id)}
        <option value={l.id} selected={l.id === settings.codeLanguage}>{l.name}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label={t("indentLabel")} hint={t("indentHint")}>
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setCodeIndent(e.currentTarget.value as IndentMode)}
    >
      {#each CODE_INDENTS as m (m.id)}
        <option value={m.id} selected={m.id === settings.codeIndent}>{m.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label={t("codeLinesLabel")} hint={t("codeLinesHint", { min: MIN_CODE_LINES, max: MAX_CODE_LINES })}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="number"
      min={MIN_CODE_LINES}
      max={MAX_CODE_LINES}
      value={settings.codeLines}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLInputElement>) => {
        settings.setCodeLines(e.currentTarget.valueAsNumber);
        e.currentTarget.value = String(settings.codeLines);
      }}
    />
  {/snippet}
</SettingField>

<style>
  input[type="number"] {
    width: 4.5rem;
    padding: 0.3rem 0.4rem;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
</style>
