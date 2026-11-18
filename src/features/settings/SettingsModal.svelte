<script lang="ts">
  import { focusTyping } from "../practice/focus";
  import { SECTIONS } from "./registry";
  import { isSettingsShortcut, moveSection } from "./sections";
  import type { SettingsStore } from "./store.svelte";

  // Settings modal (S8). A native modal <dialog> traps focus, makes the page inert (typing never
  // reaches the practice text) and gives a backdrop; sections come from `registry.ts`.
  let { settings }: { settings: SettingsStore } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let current = $state(SECTIONS[0].id);
  const ids = SECTIONS.map((s) => s.id);
  const tabs: Record<string, HTMLButtonElement | undefined> = $state({});

  /** Opens the modal, on `section` when given (e.g. the header's profile name opens "Perfiles"). */
  export function open(section?: string) {
    if (section && ids.includes(section)) current = section;
    if (!dialog || dialog.open) return;
    dialog.showModal();
    tabs[current]?.focus();
  }

  export function close() {
    dialog?.close();
  }

  // Escape (native cancel), ×, backdrop and Ctrl+, all end here: give the keyboard back.
  function onClose() {
    focusTyping();
  }

  // Keys stay in the modal: Escape closes it instead of restarting the text, Enter never starts
  // the next one. F2 still reaches the app so the keyboard toggles from anywhere.
  function onKeydown(e: KeyboardEvent) {
    if (isSettingsShortcut(e)) {
      e.preventDefault();
      close();
    }
    if (e.key !== "F2") e.stopPropagation();
  }

  // The dialog box has no padding, so a click whose target is the dialog itself hit the backdrop.
  function onClick(e: MouseEvent) {
    e.stopPropagation();
    if (e.target === dialog) close();
  }

  function onTabKeydown(e: KeyboardEvent) {
    const next = moveSection(ids, current, e.key);
    if (next === null) return;
    e.preventDefault();
    current = next;
    tabs[next]?.focus();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  class="settings"
  aria-labelledby="settings-title"
  onclose={onClose}
  onkeydown={onKeydown}
  onclick={onClick}
>
  <div class="frame">
    <header>
      <h2 id="settings-title">Ajustes</h2>
      <button type="button" class="close" aria-label="Cerrar" title="Cerrar" onclick={close}>×</button>
    </header>
    <div class="body">
      <div class="tabs" role="tablist" aria-orientation="vertical" aria-label="Secciones de ajustes">
        {#each SECTIONS as s (s.id)}
          <button
            type="button"
            role="tab"
            id="settings-tab-{s.id}"
            aria-selected={s.id === current}
            aria-controls="settings-panel-{s.id}"
            tabindex={s.id === current ? 0 : -1}
            bind:this={tabs[s.id]}
            onclick={() => (current = s.id)}
            onkeydown={onTabKeydown}
          >
            {s.label}
          </button>
        {/each}
      </div>
      {#each SECTIONS as s (s.id)}
        <div
          class="panel"
          role="tabpanel"
          id="settings-panel-{s.id}"
          aria-labelledby="settings-tab-{s.id}"
          hidden={s.id !== current}
        >
          <s.component {settings} />
        </div>
      {/each}
    </div>
  </div>
</dialog>

<style>
  /* No padding: the frame fills the box, so only backdrop clicks target the dialog itself. */
  dialog {
    width: min(46rem, calc(100vw - 2rem));
    max-width: none;
    max-height: calc(100vh - 2rem);
    padding: 0;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 0.75rem;
  }
  dialog[open] {
    display: flex;
  }
  dialog::backdrop {
    background: color-mix(in srgb, var(--bg, #000) 70%, transparent);
  }
  .frame {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 0.75rem 0.6rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .close {
    padding: 0.1rem 0.55rem;
    font-size: 1.3rem;
    line-height: 1.2;
    background: transparent;
    border-color: transparent;
  }
  .close:hover {
    border-color: var(--border);
  }
  /* Section list on the left; only the active panel scrolls. */
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 11rem minmax(0, 1fr);
  }
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem;
    overflow-y: auto;
    background: var(--surface-2);
    border-right: 1px solid var(--border);
  }
  [role="tab"] {
    padding: 0.45rem 0.75rem;
    text-align: left;
    color: var(--muted);
    background: transparent;
    border-color: transparent;
  }
  [role="tab"]:hover {
    color: var(--text);
  }
  [role="tab"][aria-selected="true"] {
    color: var(--text);
    font-weight: 600;
    background: var(--surface);
    border-color: var(--border);
    border-left: 3px solid var(--accent);
  }
  .panel {
    min-height: 0;
    overflow-y: auto;
    padding: 0.5rem 1.25rem 1rem;
  }
  /* Narrow windows: the section list becomes a row above the panel. */
  @media (max-width: 600px) {
    .body {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto minmax(0, 1fr);
    }
    .tabs {
      flex-direction: row;
      flex-wrap: wrap;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    [role="tab"][aria-selected="true"] {
      border-left: 1px solid var(--border);
      border-bottom: 3px solid var(--accent);
    }
  }
</style>
