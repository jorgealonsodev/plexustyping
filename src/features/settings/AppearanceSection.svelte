<script lang="ts">
  import { MAX_THEME_FILE_CHARS, THEMES, TOO_LARGE } from "../themes";
  import SettingField from "./SettingField.svelte";
  import type { SettingsStore } from "./store.svelte";

  let { settings }: { settings: SettingsStore } = $props();

  const groups = $derived(
    [
      { label: "Oscuros", themes: THEMES.filter((t) => t.dark) },
      { label: "Claros", themes: THEMES.filter((t) => !t.dark) },
      { label: "Importados", themes: settings.customThemes },
    ].filter((g) => g.themes.length > 0),
  );
  const imported = $derived(settings.customThemes.find((t) => t.id === settings.themeId));

  let picker: HTMLInputElement;
  let notice = $state<{ text: string; error: boolean } | null>(null);

  async function importFile(e: Event & { currentTarget: HTMLInputElement }) {
    const input = e.currentTarget;
    const file = input.files?.[0];
    // Cleared so choosing the same file again fires `change`.
    input.value = "";
    if (!file) return;
    try {
      // Bytes are at least chars: a file over the limit is never read.
      const result = file.size > MAX_THEME_FILE_CHARS ? null : settings.importTheme(await file.text(), file.name);
      if (!result) notice = { text: TOO_LARGE, error: true };
      else if (result.ok) notice = { text: `Tema importado: ${result.name}`, error: false };
      else notice = { text: result.error, error: true };
    } catch (err) {
      notice = { text: `No se pudo leer el archivo: ${String(err)}`, error: true };
    }
  }

  function remove(id: string) {
    settings.removeTheme(id);
    notice = null;
  }
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

<SettingField
  label="Temas de VS Code"
  hint="Importa un archivo de tema (.json) de VS Code: usa sus colores del editor y del terminal."
>
  {#snippet children({ id, hintId })}
    <input
      bind:this={picker}
      class="picker"
      type="file"
      accept=".json,.jsonc,application/json"
      tabindex="-1"
      aria-hidden="true"
      onchange={importFile}
    />
    <button {id} type="button" aria-describedby={hintId} onclick={() => picker.click()}>Importar tema de VS Code…</button>
    {#if imported}
      <button type="button" onclick={() => remove(imported.id)}>Eliminar tema importado</button>
    {/if}
  {/snippet}
</SettingField>
{#if notice}<p class="notice" class:error={notice.error} role={notice.error ? "alert" : "status"}>{notice.text}</p>{/if}

<style>
  .picker {
    display: none;
  }
  button {
    padding: 0.25rem 0.6rem;
    font-size: 0.85rem;
  }
  button + button {
    margin-left: 0.4rem;
  }
  .notice {
    margin: 0.4rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .notice.error {
    color: var(--danger);
  }
</style>
