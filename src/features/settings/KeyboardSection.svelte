<script lang="ts">
  import SettingField from "./SettingField.svelte";
  import { LAYOUTS } from "./settings";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change = Event & { currentTarget: HTMLInputElement };
  type SelectChange = Event & { currentTarget: HTMLSelectElement };
</script>

<SettingField
  label="Distribución"
  hint="Debe coincidir con la del sistema; cambiarla carga un texto nuevo."
>
  {#snippet children({ id, hintId })}
    <select {id} aria-describedby={hintId} onchange={(e: SelectChange) => settings.setLayout(e.currentTarget.value)}>
      {#each LAYOUTS as l (l.id)}
        <option value={l.id} selected={l.id === settings.layout}>{l.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

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

<SettingField toggle label="Destellos" hint="Las teclas pulsadas se iluminan en verde o rojo (azul o naranja con el modo daltónico).">
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

<SettingField toggle label="Mapa de calor" hint="Tiñe cada tecla según tus errores y lentitud y la marca con su nivel, de 1 a 5.">
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
