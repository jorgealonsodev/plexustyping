<script lang="ts">
  import { type FocusTarget, returnFocus } from "../practice/focus";
  import { SECTIONS } from "./registry";
  import { tick } from "svelte";
  import { isSearchShortcut, isSettingsShortcut, matchesSettingQuery, moveSection } from "./sections";
  import type { SettingsStore } from "./store.svelte";

  // Settings modal (S8). A native modal <dialog> traps focus, makes the page inert (typing never
  // reaches the practice text) and gives a backdrop; sections come from `registry.ts`.
  let { settings }: { settings: SettingsStore } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let current = $state(SECTIONS[0].id);
  const ids = SECTIONS.map((s) => s.id);
  const tabs: Record<string, HTMLButtonElement | undefined> = $state({});
  // The control that opened the modal; `null` when a shortcut did (focus goes back to the typing field).
  let opener: FocusTarget | null = null;

  // Quick search ("añade un buscador rapido"): every section shows only the rows whose label, hint or
  // options hold every word of the query, accents and case aside.
  let query = $state("");
  let search: HTMLInputElement | undefined = $state();
  let panels: HTMLDivElement | undefined = $state();
  let matches = $state(0);
  const searching = $derived(query.trim() !== "");
  /** Sections with at least one matching row while searching. */
  let found: Record<string, boolean> = $state({});

  // Marks what the search hid, so clearing it restores exactly that.
  const HIDDEN = "data-search-hidden";
  $effect(() => {
    const q = query;
    const root = panels;
    if (!root) return;
    void tick().then(() => {
      let count = 0;
      const next: Record<string, boolean> = {};
      const setShown = (el: HTMLElement, show: boolean) => {
        if (show && el.hasAttribute(HIDDEN)) {
          el.removeAttribute(HIDDEN);
          el.hidden = false;
        } else if (!show && !el.hidden) {
          el.hidden = true;
          el.setAttribute(HIDDEN, "");
        }
      };
      for (const panel of root.querySelectorAll<HTMLElement>("[data-section]")) {
        let inPanel = 0;
        // Every setting row, at any depth (some sections wrap their rows in one container).
        for (const field of panel.querySelectorAll<HTMLElement>(".field")) {
          const show = q.trim() === "" || matchesSettingQuery(field.textContent ?? "", q);
          setShown(field, show);
          if (show) inPanel++;
        }
        // Other top-level content (notes, headings) stays only beside a matching row.
        for (const child of panel.querySelectorAll<HTMLElement>(":scope > .section-body > *")) {
          if (child.classList.contains("field")) continue;
          const keep = q.trim() === "" || child.querySelector(`.field:not([${HIDDEN}])`) !== null;
          setShown(child, keep);
        }
        next[panel.dataset.section ?? ""] = inPanel > 0;
        count += inPanel;
      }
      found = next;
      matches = count;
    });
  });

  /** Enter in the search box: the first matching setting's control. Escape clears a search first. */
  function onSearchKeydown(ev: KeyboardEvent) {
    const pressed = ev.key;
    if (pressed === "Escape" && query !== "") {
      ev.preventDefault();
      query = "";
      return;
    }
    if (pressed !== "Enter") return;
    ev.preventDefault();
    const first = panels?.querySelector<HTMLElement>(
      "[data-section]:not([hidden]) .field:not([hidden]) :is(input, select, button, textarea)",
    );
    first?.focus();
  }

  function pick(id: string) {
    query = "";
    current = id;
  }

  /**
   * Opens the modal, on `section` when given (e.g. the header's profile name opens "Perfiles");
   * closing it gives the focus back to `from`, or to the typing field without one (a shortcut).
   */
  export function open(section?: string, from: FocusTarget | null = null) {
    if (section && ids.includes(section)) current = section;
    if (!dialog || dialog.open) return;
    opener = from;
    dialog.showModal();
    query = "";
    search?.focus();
  }

  export function close() {
    dialog?.close();
  }

  // Escape (native cancel), ×, backdrop and Ctrl+, all end here: give the keyboard back to the
  // control that opened the modal, or to the typing field.
  function onClose() {
    const to = opener;
    opener = null;
    returnFocus(to);
  }

  // Keys stay in the modal: Escape closes it instead of restarting the text, Enter never starts
  // the next one. F2 still reaches the app so the keyboard toggles from anywhere.
  function onKeydown(e: KeyboardEvent) {
    if (isSettingsShortcut(e)) {
      e.preventDefault();
      close();
    }
    // Ctrl+F: back to the search box from anywhere in the modal.
    if (isSearchShortcut(e)) {
      e.preventDefault();
      search?.focus();
      search?.select();
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
    // Like a click: choosing a section ends the search.
    pick(next);
    tabs[next]?.focus();
  }

  // Escape anywhere in the window (native cancel) clears an active search first; the next one closes.
  function onCancel(ev: Event) {
    if (query === "") return;
    ev.preventDefault();
    query = "";
    search?.focus();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  class="settings"
  aria-labelledby="settings-title"
  onclose={onClose}
  oncancel={onCancel}
  onkeydown={onKeydown}
  onclick={onClick}
>
  <div class="frame">
    <header>
      <h2 id="settings-title">Ajustes</h2>
      <input
        bind:this={search}
        bind:value={query}
        type="search"
        class="search"
        placeholder="Buscar ajuste…  (Ctrl+F)"
        aria-label="Buscar ajuste"
        aria-describedby="settings-search-count"
        spellcheck="false"
        autocomplete="off"
        onkeydown={onSearchKeydown}
      />
      <p id="settings-search-count" class="count" aria-live="polite">
        {#if searching}{matches === 0 ? "Ningún ajuste coincide" : matches === 1 ? "1 ajuste" : `${matches} ajustes`}{/if}
      </p>
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
            onclick={() => pick(s.id)}
            onkeydown={onTabKeydown}
          >
            {s.label}
          </button>
        {/each}
      </div>
      <div class="panels" bind:this={panels}>
        {#each SECTIONS as s (s.id)}
          <div
            class="panel"
            role="tabpanel"
            id="settings-panel-{s.id}"
            data-section={s.id}
            aria-labelledby="settings-tab-{s.id}"
            hidden={searching ? !found[s.id] : s.id !== current}
          >
            {#if searching}<h3 class="result-section">{s.label}</h3>{/if}
            <div class="section-body"><s.component {settings} /></div>
          </div>
        {/each}
      </div>
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
  .search {
    flex: 1;
    min-width: 0;
    margin: 0 0.75rem 0 1rem;
    padding: 0.35rem 0.6rem;
    font: inherit;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  .search::placeholder {
    color: var(--muted);
  }
  .count {
    flex: none;
    margin: 0 0.5rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .result-section {
    margin: 0.75rem 0 0;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted);
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
  /* The panels share one scroll: while searching, every section with a match is listed. */
  .panels {
    min-height: 0;
    overflow-y: auto;
  }
  .panel {
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
