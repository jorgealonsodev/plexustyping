<script lang="ts">
  import type { Sound } from "../metronome/api";
  import { MAX_ACCENT_EVERY, MAX_BPM, MIN_BPM, SOUNDS } from "../metronome/beat";
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  type Change<T extends HTMLElement> = Event & { currentTarget: T };
  const m = $derived(settings.metronome);
  const number = (e: Change<HTMLInputElement>) => e.currentTarget.valueAsNumber;
</script>

<SettingField toggle label="Metrónomo" shortcut="F4" hint="Suena mientras practicas; un clic = una pulsación esperada.">
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={m.enabled}
      onchange={(e: Change<HTMLInputElement>) => settings.setMetronome({ enabled: e.currentTarget.checked })}
    />
  {/snippet}
</SettingField>

<SettingField label="Tempo" hint="Pulsaciones por minuto, de {MIN_BPM} a {MAX_BPM}.">
  {#snippet children({ id, hintId })}
    <input
      type="range"
      min={MIN_BPM}
      max={MAX_BPM}
      value={m.bpm}
      aria-label="Tempo (deslizador)"
      oninput={(e: Change<HTMLInputElement>) => settings.setMetronome({ bpm: number(e) })}
    />
    <input
      {id}
      type="number"
      min={MIN_BPM}
      max={MAX_BPM}
      value={m.bpm}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLInputElement>) => settings.setMetronome({ bpm: number(e) })}
    />
  {/snippet}
</SettingField>

<SettingField
  toggle
  label="Tempo adaptativo"
  hint="Tras cada texto sube el tempo si aciertas y sigues el ritmo, y lo baja si fallas o lo pierdes."
>
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="checkbox"
      aria-describedby={hintId}
      checked={m.adaptive}
      onchange={(e: Change<HTMLInputElement>) => settings.setMetronome({ adaptive: e.currentTarget.checked })}
    />
  {/snippet}
</SettingField>

<SettingField label="Sonido" hint="Sonido de cada pulsación.">
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLSelectElement>) => settings.setMetronome({ sound: e.currentTarget.value as Sound })}
    >
      {#each SOUNDS as s (s.id)}
        <option value={s.id} selected={s.id === m.sound}>{s.label}</option>
      {/each}
    </select>
  {/snippet}
</SettingField>

<SettingField label="Volumen" hint="{m.volume} %">
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="range"
      min="0"
      max="100"
      value={m.volume}
      aria-describedby={hintId}
      oninput={(e: Change<HTMLInputElement>) => settings.setMetronome({ volume: number(e) })}
    />
  {/snippet}
</SettingField>

<SettingField label="Acento" hint="Acentúa una de cada N pulsaciones; 0 = sin acento.">
  {#snippet children({ id, hintId })}
    <input
      {id}
      type="number"
      min="0"
      max={MAX_ACCENT_EVERY}
      value={m.accentEvery}
      aria-describedby={hintId}
      onchange={(e: Change<HTMLInputElement>) => settings.setMetronome({ accentEvery: number(e) })}
    />
  {/snippet}
</SettingField>

<style>
  input[type="number"] {
    width: 4.5rem;
    margin-left: 0.5rem;
    padding: 0.3rem 0.4rem;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  input[type="range"] {
    width: 9rem;
    accent-color: var(--accent);
  }
</style>
