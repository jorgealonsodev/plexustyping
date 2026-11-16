<script lang="ts">
  import type { ErrorMode } from "../practice/engine";
  import SettingField from "./SettingField.svelte";
  import { ERROR_MODES, TARGET_COUNTS, TEXT_LENGTHS } from "./settings";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change<T extends HTMLElement> = Event & { currentTarget: T };
</script>

<SettingField label="Modo de error" hint="Qué pasa al fallar una tecla; cambiarlo reinicia el texto.">
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

<SettingField label="Objetivos" hint="Caracteres o bigramas débiles por texto, desde el siguiente.">
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

<SettingField label="Longitud" hint="Caracteres por texto, desde el siguiente.">
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setTextLength(Number(e.currentTarget.value))}
    >
      {#each TEXT_LENGTHS as n (n)}
        <option value={n} selected={n === settings.textLength}>{n}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField toggle label="Estadísticas en vivo" hint="Muestra las métricas mientras escribes.">
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
