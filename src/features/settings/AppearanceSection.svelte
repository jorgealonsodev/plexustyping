<script lang="ts">
  import { THEMES } from "../themes";
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  const groups = [
    { label: "Oscuros", themes: THEMES.filter((t) => t.dark) },
    { label: "Claros", themes: THEMES.filter((t) => !t.dark) },
  ];
</script>

<SettingField label="Tema" hint="Colores de toda la aplicación; se aplica al momento.">
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) => settings.setTheme(e.currentTarget.value)}
    >
      {#each groups as g (g.label)}
        <optgroup label={g.label}>
          {#each g.themes as t (t.id)}
            <option value={t.id} selected={t.id === settings.themeId}>{t.name}</option>
          {/each}
        </optgroup>
      {/each}
    </select>
  {/snippet}
</SettingField>
