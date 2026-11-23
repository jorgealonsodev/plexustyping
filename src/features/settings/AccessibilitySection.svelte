<script lang="ts">
  import SettingField from "./SettingField.svelte";
  import { t } from "./messages";
  import { COLOR_BLIND_SETTING, SCREEN_READER_SETTING } from "./settings";
  import type { SettingsStore } from "./store.svelte";

  // The accessibility aids in one place; the high-contrast themes are in Apariencia and the text
  // size and spacing in Texto.
  let { settings }: { settings: SettingsStore } = $props();
</script>

<!-- Screen reader mode (a11y S3): a polite aria-live region in the practice; silent while off. -->
<SettingField toggle label={SCREEN_READER_SETTING.label} hint={SCREEN_READER_SETTING.hint}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.screenReader}
      onchange={(e: Event & { currentTarget: HTMLInputElement }) => settings.setScreenReader(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<!-- Colour-blind keyboard (a11y S5): Okabe–Ito blue and orange flashes, a bold underline on a wrong key. -->
<SettingField toggle label={COLOR_BLIND_SETTING.label} hint={COLOR_BLIND_SETTING.hint}>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={settings.colorBlind}
      onchange={(e: Event & { currentTarget: HTMLInputElement }) => settings.setColorBlind(e.currentTarget.checked)}
    />
  {/snippet}
</SettingField>

<p class="note">{t("accessibilityNote")}</p>

<style>
  .note {
    margin: 0.75rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
  }
</style>
