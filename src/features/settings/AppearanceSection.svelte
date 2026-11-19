<script lang="ts">
  import { MAX_THEME_FILE_CHARS, THEMES, TOO_LARGE } from "../themes";
  import SettingField from "./SettingField.svelte";
  import { TYPOGRAPHY_FIELDS, type Typography } from "./settings";
  import { FONT_GROUPS, FONT_SETTING, PRACTICE_FONTS, fontById } from "./fonts";
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

  const UNITS: Record<keyof Typography, string> = { fontScale: " %", letterSpacing: " em", lineHeight: "" };
  /** A value as the hint shows it: Spanish decimal comma and its unit, e.g. `0,12 em`. */
  const shown = (key: keyof Typography, value: number) => `${String(value).replace(".", ",")}${UNITS[key]}`;
  const setTypography = (key: keyof Typography) => (e: Event & { currentTarget: HTMLInputElement }) => {
    const value = e.currentTarget.valueAsNumber;
    if (Number.isFinite(value)) settings.setTypography({ [key]: value });
  };

  const font = $derived(fontById(settings.practiceFont));

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

<!-- Typeface of the practice text, prose and code (practice-font S1): each option and the preview in its own font. -->
<SettingField label={FONT_SETTING.label} hint="{FONT_SETTING.hint} {font.hint}">
  {#snippet children({ id, hintId })}
    <select
      {id}
      aria-describedby={hintId}
      style="font-family: {font.family}"
      onchange={(e: Event & { currentTarget: HTMLSelectElement }) => settings.setPracticeFont(e.currentTarget.value)}
    >
      {#each FONT_GROUPS as g (g)}
        <optgroup label={g}>
          {#each PRACTICE_FONTS.filter((f) => f.group === g) as f (f.id)}
            <option value={f.id} selected={f.id === font.id} style="font-family: {f.family}">{f.name}</option>
          {/each}
        </optgroup>
      {/each}
    </select>
    <p class="preview" aria-hidden="true" style="font-family: {font.family}">Ñandú veloz: 0O 1lI ¿{"{}"}?</p>
  {/snippet}
</SettingField>

<!-- Typography of the practice text, prose and code (a11y S4): applies live, out-of-range values are clamped. -->
{#each TYPOGRAPHY_FIELDS as f (f.key)}
  <SettingField
    label={f.label}
    hint="Texto de práctica: {shown(f.key, settings.typography[f.key])} (de {shown(f.key, f.min)} a {shown(f.key, f.max)})."
  >
    {#snippet children({ id, hintId })}
      <input
        type="range"
        min={f.min}
        max={f.max}
        step={f.step}
        value={settings.typography[f.key]}
        aria-label="{f.label} (deslizador)"
        oninput={setTypography(f.key)}
      />
      <input
        {id}
        type="number"
        min={f.min}
        max={f.max}
        step={f.step}
        value={settings.typography[f.key]}
        aria-describedby={hintId}
        onchange={setTypography(f.key)}
      />
    {/snippet}
  </SettingField>
{/each}

<style>
  .picker {
    display: none;
  }
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
  button {
    padding: 0.25rem 0.6rem;
    font-size: 0.85rem;
  }
  button + button {
    margin-left: 0.4rem;
  }
  .preview {
    margin: 0.4rem 0 0;
    font-size: 1.05rem;
    font-feature-settings: "liga" 0, "calt" 0;
    color: var(--text);
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
