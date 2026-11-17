<script lang="ts">
  import { LANGUAGES } from "../code/languages.svelte";
  import { focusTyping } from "../practice/focus";
  import { exportSessionsCsv, sessionHistory, type SessionSummary } from "./api";
  import {
    chartGeometry,
    exportNotice,
    FILTERS,
    formatSummary,
    isHistoryShortcut,
    modeArg,
    summarize,
    tableRow,
    type ChartBox,
    type HistoryFilter,
  } from "./history";

  // History window (S1–S5, S7). Built like the settings modal: a native modal <dialog> traps
  // focus and makes the page inert, so typing never reaches the practice text.
  let dialog: HTMLDialogElement | undefined = $state();
  let filter = $state<HistoryFilter>("all");
  let sessions = $state.raw<SessionSummary[]>([]);
  let loaded = $state(false);
  let loadError = $state<string | null>(null);
  let notice = $state<{ text: string; error: boolean } | null>(null);
  let exporting = $state(false);
  // Only the newest request may fill the window (filters can change faster than replies).
  let request = 0;

  const BOX: ChartBox = { width: 600, height: 180, left: 36, right: 44, top: 10, bottom: 14 };
  const summary = $derived(summarize(sessions));
  const chart = $derived(chartGeometry(sessions, BOX));
  const rows = $derived(sessions.map((s) => tableRow(s, (id) => LANGUAGES.name(id))));

  export function open() {
    if (!dialog || dialog.open) return;
    void LANGUAGES.load();
    notice = null;
    dialog.showModal();
    void load();
  }

  export function close() {
    dialog?.close();
  }

  async function load() {
    const id = ++request;
    loadError = null;
    try {
      const list = await sessionHistory(modeArg(filter));
      if (id !== request) return;
      sessions = list;
    } catch (err) {
      if (id !== request) return;
      sessions = [];
      loadError = String(err);
    }
    loaded = true;
  }

  function choose(f: HistoryFilter) {
    if (f === filter) return;
    filter = f;
    notice = null;
    void load();
  }

  async function exportCsv() {
    exporting = true;
    try {
      notice = { text: exportNotice(await exportSessionsCsv(modeArg(filter))), error: false };
    } catch (err) {
      notice = { text: String(err), error: true };
    } finally {
      exporting = false;
    }
  }

  // Escape (native cancel), ×, backdrop and Ctrl+H all end here: give the keyboard back.
  function onClose() {
    focusTyping();
  }

  // Keys stay in the modal: Escape closes it instead of restarting the text. F2 still reaches the
  // app so the keyboard toggles from anywhere.
  function onKeydown(e: KeyboardEvent) {
    if (isHistoryShortcut(e)) {
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
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  class="history"
  aria-labelledby="history-title"
  onclose={onClose}
  onkeydown={onKeydown}
  onclick={onClick}
>
  <div class="frame">
    <header>
      <h2 id="history-title">Historial</h2>
      <div class="filters" role="group" aria-label="Filtro de sesiones">
        {#each FILTERS as f (f.id)}
          <button type="button" aria-pressed={filter === f.id} class:active={filter === f.id} onclick={() => choose(f.id)}
            >{f.label}</button
          >
        {/each}
      </div>
      <button type="button" class="close" aria-label="Cerrar" title="Cerrar" onclick={close}>×</button>
    </header>
    <div class="body">
      {#if loadError !== null}
        <p class="error" role="alert">{loadError}</p>
      {:else if loaded && summary === null}
        <p class="empty">Aún no hay sesiones guardadas.</p>
      {:else if summary !== null}
        <dl class="summary">
          {#each formatSummary(summary) as item (item.label)}
            <div>
              <dt>{item.label}</dt>
              <dd>{item.value}</dd>
            </div>
          {/each}
        </dl>

        <figure class="chart">
          <figcaption>
            Últimas {chart.wpm.length} sesiones:
            <span class="key wpm">PPM netas</span>
            <span class="key accuracy">Precisión</span>
          </figcaption>
          <svg
            viewBox="0 0 {BOX.width} {BOX.height}"
            role="img"
            aria-label="Evolución de las PPM netas y la precisión, de la sesión más antigua a la más reciente"
          >
            {#each chart.ticks as t (t.y)}
              <line class="grid" x1={BOX.left} x2={BOX.width - BOX.right} y1={t.y} y2={t.y} />
              <text class="tick wpm" x={BOX.left - 6} y={t.y} text-anchor="end" dominant-baseline="middle">{t.wpm}</text>
              <text class="tick accuracy" x={BOX.width - BOX.right + 6} y={t.y} dominant-baseline="middle">{t.accuracy}</text>
            {/each}
            <polyline class="line accuracy" points={chart.accuracyLine} fill="none" stroke-width="2" />
            <polyline class="line wpm" points={chart.wpmLine} fill="none" stroke-width="2" />
            {#each chart.accuracy as p (p.id)}
              <circle class="dot accuracy" cx={p.x} cy={p.y} r="2.5" />
            {/each}
            {#each chart.wpm as p (p.id)}
              <circle class="dot wpm" cx={p.x} cy={p.y} r="2.5" />
            {/each}
          </svg>
        </figure>

        <div class="export">
          <button type="button" onclick={exportCsv} disabled={exporting}>Exportar CSV</button>
          {#if notice !== null}
            <p class:error={notice.error} role="status">{notice.text}</p>
          {/if}
        </div>

        <table>
          <thead>
            <tr>
              <th scope="col">Fecha</th>
              <th scope="col">Modo</th>
              <th scope="col" class="num">PPM netas</th>
              <th scope="col" class="num">Precisión</th>
              <th scope="col" class="num">Tiempo</th>
              <th scope="col" class="num">BPM</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as r (r.id)}
              <tr>
                <td>{r.date}</td>
                <td>{r.mode}</td>
                <td class="num">{r.netWpm}</td>
                <td class="num">{r.accuracy}</td>
                <td class="num">{r.time}</td>
                <td class="num">{r.bpm}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </div>
</dialog>

<style>
  /* No padding: the frame fills the box, so only backdrop clicks target the dialog itself. */
  dialog {
    width: min(50rem, calc(100vw - 2rem));
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
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.6rem 0.75rem 0.6rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .filters {
    display: flex;
    margin-right: auto;
  }
  .filters button {
    padding: 0.3rem 0.7rem;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .filters button:not(:first-child) {
    margin-left: -1px;
  }
  .filters button:first-child {
    border-radius: 0.375rem 0 0 0.375rem;
  }
  .filters button:last-child {
    border-radius: 0 0.375rem 0.375rem 0;
  }
  .filters .active {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
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
  /* The whole body scrolls, so summary, chart and table fit even at 720×480. */
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.75rem 1.25rem 1rem;
  }
  .empty,
  .error {
    margin: 1.5rem 0;
    text-align: center;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
  }
  .summary {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.5rem;
    margin: 0 0 0.75rem;
  }
  .summary div {
    padding: 0.4rem 0.6rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
  }
  dt {
    font-size: 0.75rem;
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }
  .chart {
    margin: 0 0 0.75rem;
  }
  figcaption {
    display: flex;
    gap: 0.75rem;
    font-size: 0.8rem;
    color: var(--muted);
  }
  .key::before {
    content: "";
    display: inline-block;
    width: 0.8rem;
    height: 0.2rem;
    margin-right: 0.3rem;
    vertical-align: middle;
    border-radius: 0.1rem;
  }
  .key.wpm::before {
    background: var(--accent);
  }
  .key.accuracy::before {
    background: var(--success);
  }
  svg {
    display: block;
    width: 100%;
    height: auto;
    max-height: 11rem;
  }
  .grid {
    stroke: var(--border);
  }
  .tick {
    font-size: 11px;
    fill: var(--muted);
  }
  .line.wpm {
    stroke: var(--accent);
  }
  .line.accuracy {
    stroke: var(--success);
  }
  .dot.wpm {
    fill: var(--accent);
  }
  .dot.accuracy {
    fill: var(--success);
  }
  .export {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .export button {
    flex: none;
  }
  .export p {
    margin: 0;
    min-width: 0;
    font-size: 0.85rem;
    overflow-wrap: anywhere;
    color: var(--success);
  }
  .export p.error {
    color: var(--danger);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  th,
  td {
    padding: 0.3rem 0.5rem;
    text-align: left;
    white-space: nowrap;
    border-bottom: 1px solid var(--border);
  }
  th {
    position: sticky;
    top: -0.75rem;
    font-weight: 600;
    color: var(--muted);
    background: var(--surface);
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  @media (max-width: 600px) {
    .summary {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
