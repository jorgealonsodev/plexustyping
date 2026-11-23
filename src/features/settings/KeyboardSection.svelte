<script lang="ts">
  import SettingField from "./SettingField.svelte";
  import { t } from "./messages";
  import { LAYOUTS } from "./settings";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change = Event & { currentTarget: HTMLInputElement };
  type SelectChange = Event & { currentTarget: HTMLSelectElement };
</script>

<SettingField
  label={t("layoutLabel")}
  hint={t("layoutHint")}
>
  {#snippet children({ id, hintId })}
    <select {id} aria-describedby={hintId} onchange={(e: SelectChange) => settings.setLayout(e.currentTarget.value)}>
      {#each LAYOUTS as l (l.id)}
        <option value={l.id} selected={l.id === settings.layout}>{l.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField toggle label={t("keyboardLabel")} shortcut="F2" hint={t("keyboardHint")}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.showKeyboard}
      onchange={(e: Change) => settings.setShowKeyboard(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<SettingField toggle label={t("flashesLabel")} hint={t("flashesHint")}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.flashes}
      onchange={(e: Change) => settings.setFlashes(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<SettingField toggle label={t("heatmapLabel")} hint={t("heatmapHint")}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.heatmap}
      onchange={(e: Change) => settings.setHeatmap(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>
