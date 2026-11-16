<script lang="ts">
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change = Event & { currentTarget: HTMLInputElement };
</script>

<SettingField toggle label="Teclado visual" shortcut="F2" hint="Teclado en pantalla que resalta la siguiente tecla.">
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

<SettingField toggle label="Destellos" hint="Las teclas pulsadas se iluminan en verde o rojo.">
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

<SettingField toggle label="Mapa de calor" hint="Tiñe cada tecla según tus errores y lentitud.">
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
