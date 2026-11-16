<script lang="ts">
  import { tick } from "svelte";
  import type { ErrorMode } from "../practice/engine";
  import { focusTyping } from "../practice/focus";
  import { THEMES } from "../themes";
  import { ERROR_MODES, TARGET_COUNTS, TEXT_LENGTHS } from "./settings";

  let {
    mode,
    themeId,
    liveStats,
    showKeyboard,
    flashes,
    heatmap,
    targetCount,
    textLength,
    onMode,
    onTheme,
    onLiveStats,
    onShowKeyboard,
    onFlashes,
    onHeatmap,
    onTargetCount,
    onTextLength,
  }: {
    mode: ErrorMode;
    themeId: string;
    liveStats: boolean;
    showKeyboard: boolean;
    flashes: boolean;
    heatmap: boolean;
    targetCount: number;
    textLength: number;
    onMode: (mode: ErrorMode) => void;
    onTheme: (id: string) => void;
    onLiveStats: (on: boolean) => void;
    onShowKeyboard: (on: boolean) => void;
    onFlashes: (on: boolean) => void;
    onHeatmap: (on: boolean) => void;
    onTargetCount: (count: number) => void;
    onTextLength: (length: number) => void;
  } = $props();

  const groups = [
    { label: "Oscuros", themes: THEMES.filter((t) => t.dark) },
    { label: "Claros", themes: THEMES.filter((t) => !t.dark) },
  ];

  // Each handler gives the keyboard back to the typing field after the change.
  function pickMode(e: Event & { currentTarget: HTMLSelectElement }) {
    onMode(e.currentTarget.value as ErrorMode);
    focusTyping();
  }

  function pickTheme(e: Event & { currentTarget: HTMLSelectElement }) {
    onTheme(e.currentTarget.value);
    focusTyping();
  }

  const pickNumber = (set: (n: number) => void) => (e: Event & { currentTarget: HTMLSelectElement }) => {
    set(Number(e.currentTarget.value));
    focusTyping();
  };

  const toggle = (set: (on: boolean) => void) => (e: Event & { currentTarget: HTMLInputElement }) => {
    set(e.currentTarget.checked);
    focusTyping();
  };

  // "Ajustes" disclosure: the less used settings live in a popover so the header stays one row.
  let open = $state(false);
  let disclosure: HTMLElement | undefined = $state();
  let panel: HTMLElement | undefined = $state();

  /** Closes the panel; `refocus` hands the keyboard back to the typing field. */
  function close(refocus: boolean) {
    open = false;
    if (refocus) focusTyping();
  }

  async function toggleOpen() {
    if (open) return close(true);
    open = true;
    await tick();
    panel?.querySelector<HTMLElement>("select, input")?.focus();
  }

  // Escape closes the panel without reaching the practice screen (which would restart the text).
  function onDisclosureKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape" || !open) return;
    e.preventDefault();
    e.stopPropagation();
    close(true);
  }

  // Clicks inside the disclosure must not trigger the practice screen's refocus.
  function onDisclosureClick(e: MouseEvent) {
    e.stopPropagation();
  }

  // Focus leaving the disclosure closes it; focus taken by another control stays there.
  function onDisclosureFocusout(e: FocusEvent) {
    if (!open) return;
    const next = e.relatedTarget;
    if (next instanceof Node && disclosure?.contains(next)) return;
    close(!(next instanceof HTMLElement));
  }

  function onWindowPointerdown(e: PointerEvent) {
    if (open && e.target instanceof Node && !disclosure?.contains(e.target)) close(false);
  }
</script>

<svelte:window onpointerdown={onWindowPointerdown} />

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
  <div
    class="disclosure"
    bind:this={disclosure}
    role="presentation"
    onkeydown={onDisclosureKeydown}
    onclick={onDisclosureClick}
    onfocusout={onDisclosureFocusout}
  >
    <button
      type="button"
      class="trigger"
      aria-expanded={open}
      aria-controls="settings-panel"
      onclick={toggleOpen}
    >
      Ajustes <span aria-hidden="true">▾</span>
    </button>
    <div id="settings-panel" class="panel" role="group" aria-label="Ajustes" hidden={!open} bind:this={panel}>
      <label title="Caracteres o bigramas débiles por texto">
        Objetivos
        <select onchange={pickNumber(onTargetCount)}>
          {#each TARGET_COUNTS as n (n)}
            <option value={n} selected={n === targetCount}>{n}</option>
          {/each}
        </select>
      </label>
      <label title="Caracteres por texto">
        Longitud
        <select onchange={pickNumber(onTextLength)}>
          {#each TEXT_LENGTHS as n (n)}
            <option value={n} selected={n === textLength}>{n}</option>
          {/each}
        </select>
      </label>
      <label class="toggle">
        <input type="checkbox" checked={liveStats} onchange={toggle(onLiveStats)} />
        Estadísticas en vivo
      </label>
      <label class="toggle" title="Mostrar u ocultar con F2">
        <input type="checkbox" checked={showKeyboard} onchange={toggle(onShowKeyboard)} />
        Teclado <kbd>F2</kbd>
      </label>
      <label class="toggle">
        <input type="checkbox" checked={flashes} onchange={toggle(onFlashes)} />
        Destellos
      </label>
      <label class="toggle">
        <input type="checkbox" checked={heatmap} onchange={toggle(onHeatmap)} />
        Mapa de calor
      </label>
    </div>
  </div>
</div>

<style>
  /* One row: the selects shrink before anything wraps (minimum window 720×480). */
  .settings {
    flex: 0 1 auto;
    min-width: 0;
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.9rem;
    color: var(--muted);
  }
  label {
    flex: 0 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    white-space: nowrap;
  }
  .disclosure {
    flex: none;
    position: relative;
  }
  .trigger {
    padding: 0.3rem 0.6rem;
    white-space: nowrap;
  }
  .trigger[aria-expanded="true"] {
    border-color: var(--accent);
  }
  /* Overlays the practice area instead of pushing it down. */
  .panel {
    position: absolute;
    top: calc(100% + 0.4rem);
    right: 0;
    z-index: 10;
    display: grid;
    gap: 0.6rem;
    min-width: 14rem;
    max-height: calc(100vh - 5rem);
    overflow-y: auto;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
  }
  .panel[hidden] {
    display: none;
  }
  .panel label {
    justify-content: space-between;
  }
  .panel .toggle {
    justify-content: flex-start;
  }
  select {
    min-width: 6rem;
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
  kbd {
    padding: 0 0.3em;
    font-family: var(--mono);
    font-size: 0.85em;
    border: 1px solid var(--key-border);
    border-radius: 0.25em;
  }
</style>
