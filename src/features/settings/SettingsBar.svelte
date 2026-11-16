<script lang="ts">
  import type { ErrorMode } from "../practice/engine";
  import { THEMES } from "../themes";
  import { ERROR_MODES } from "./settings";

  let {
    mode,
    themeId,
    liveStats,
    onMode,
    onTheme,
    onLiveStats,
  }: {
    mode: ErrorMode;
    themeId: string;
    liveStats: boolean;
    onMode: (mode: ErrorMode) => void;
    onTheme: (id: string) => void;
    onLiveStats: (on: boolean) => void;
  } = $props();

  const groups = [
    { label: "Oscuros", themes: THEMES.filter((t) => t.dark) },
    { label: "Claros", themes: THEMES.filter((t) => !t.dark) },
  ];

  // Each handler gives the keyboard back to the practice text after the change.
  function pickMode(e: Event & { currentTarget: HTMLSelectElement }) {
    onMode(e.currentTarget.value as ErrorMode);
    e.currentTarget.blur();
  }

  function pickTheme(e: Event & { currentTarget: HTMLSelectElement }) {
    onTheme(e.currentTarget.value);
    e.currentTarget.blur();
  }

  function toggleLive(e: Event & { currentTarget: HTMLInputElement }) {
    onLiveStats(e.currentTarget.checked);
    e.currentTarget.blur();
  }
</script>

<div class="settings">
  <label>
    Modo de error
    <select onchange={pickMode}>
      {#each ERROR_MODES as m (m.id)}
        <option value={m.id} selected={m.id === mode}>{m.label}</option>
      {/each}
    </select>
  </label>
  <label>
    Tema
    <select onchange={pickTheme}>
      {#each groups as g (g.label)}
        <optgroup label={g.label}>
          {#each g.themes as t (t.id)}
            <option value={t.id} selected={t.id === themeId}>{t.name}</option>
          {/each}
        </optgroup>
      {/each}
    </select>
  </label>
  <label class="toggle">
    <input type="checkbox" checked={liveStats} onchange={toggleLive} />
    Estadísticas en vivo
  </label>
</div>

<style>
  .settings {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem 1.25rem;
    font-size: 0.9rem;
    color: var(--muted);
  }
  label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  select {
    max-width: 14rem;
    padding: 0.3rem 0.5rem;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  input {
    accent-color: var(--accent);
  }
</style>
