<script lang="ts">
  import { LANGUAGES } from "../code/languages.svelte";
  import { type FocusTarget, returnFocus } from "../practice/focus";
  import {
    exportSessionsCsv,
    historyByChar,
    historyByDay,
    sessionHistory,
    type CharRow,
    type DayRow,
    type SessionSummary,
  } from "./api";
  import {
    charChartGeometry,
    charTableRow,
    dayChartGeometry,
    charCaption,
    dayChartCaption,
    dayTableRow,
    dayTooltip,
    VIEWS,
    type HistoryView,
  } from "./charts";
  import {
    chartCaption,
    chartGeometry,
    exportNotice,
    FILTERS,
    formatSummary,
    isHistoryShortcut,
    modeArg,
    requestGuard,
    summarize,
    tableRow,
    type ChartBox,
    type HistoryFilter,
  } from "./history";

  // History window (S1–S5, S7), with views by day and by character (history charts S1–S4). Built
  // like the settings modal: a native modal <dialog> traps focus and makes the page inert, so typing
  // never reaches the practice text.
  let dialog: HTMLDialogElement | undefined = $state();
  let filter = $state<HistoryFilter>("all");
  let view = $state<HistoryView>("sessions");
  let sessions = $state.raw<SessionSummary[]>([]);
  let days = $state.raw<DayRow[]>([]);
  let chars = $state.raw<CharRow[]>([]);
  let loaded = $state(false);
  let loadError = $state<string | null>(null);
  let notice = $state<{ text: string; error: boolean } | null>(null);
  let exporting = $state(false);
  // Only the newest request may fill the window (filters can change faster than replies), and an
  // export reply only applies to the filter and opening it was started from.
  const loads = requestGuard();
  const exports = requestGuard();

  const BOX: ChartBox = { width: 600, height: 180, left: 36, right: 44, top: 10, bottom: 14 };
  const summary = $derived(summarize(sessions));
  const chart = $derived(chartGeometry(sessions, BOX));
  const rows = $derived(sessions.map((s) => tableRow(s, (id) => LANGUAGES.name(id))));
  const DAY_BOX: ChartBox = { width: 600, height: 190, left: 36, right: 44, top: 10, bottom: 22 };
  const CHAR_BOX: ChartBox = { width: 600, height: 200, left: 44, right: 12, top: 10, bottom: 46 };
  const dayChart = $derived(dayChartGeometry(days, DAY_BOX));
  const dayRows = $derived(days.map(dayTableRow));
  const charChart = $derived(charChartGeometry(chars, CHAR_BOX));
  const charRows = $derived(chars.map(charTableRow));
  const dayLabels = $derived(new Map(dayRows.map((r) => [r.day, dayTooltip(r)])));
  const dayCaption = $derived(dayChartCaption(days.length));
  // Labels of the character chart sit just under its plot area.
  const CHAR_LABEL_Y = CHAR_BOX.height - CHAR_BOX.bottom + 14;

  // The control that opened the window; `null` when a shortcut did (focus goes back to the typing field).
  let opener: FocusTarget | null = null;

  /** Opens the window; closing it gives the focus back to `from`, or to the typing field without one (a shortcut). */
  export function open(from: FocusTarget | null = null) {
    if (!dialog || dialog.open) return;
    opener = from;
    void LANGUAGES.load();
    notice = null;
    exports.invalidate();
    exporting = false;
    dialog.showModal();
    void load();
  }

  export function close() {
    dialog?.close();
  }

  async function load() {
    const id = loads.next();
    loadError = null;
    try {
      const mode = modeArg(filter);
      const [list, byDay, byChar] = await Promise.all([sessionHistory(mode), historyByDay(mode), historyByChar(mode)]);
      if (!loads.isCurrent(id)) return;
      sessions = list;
      days = byDay;
      chars = byChar;
    } catch (err) {
      if (!loads.isCurrent(id)) return;
      sessions = [];
      days = [];
      chars = [];
      loadError = String(err);
    }
    loaded = true;
  }

  function choose(f: HistoryFilter) {
    if (f === filter) return;
    filter = f;
    notice = null;
    exports.invalidate();
    exporting = false;
    void load();
  }

  async function exportCsv() {
    const id = exports.next();
    exporting = true;
    try {
      const result = await exportSessionsCsv(modeArg(filter));
      if (exports.isCurrent(id)) notice = { text: exportNotice(result), error: false };
    } catch (err) {
      if (exports.isCurrent(id)) notice = { text: String(err), error: true };
    } finally {
      if (exports.isCurrent(id)) exporting = false;
    }
  }

  // Escape (native cancel), ×, backdrop and Ctrl+H all end here: give the keyboard back to the
  // control that opened the window, or to the typing field.
  function onClose() {
    const to = opener;
    opener = null;
    returnFocus(to);
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
    <div class="filters views" role="group" aria-label="Vista del historial">
      {#each VIEWS as v (v.id)}
        <button type="button" aria-pressed={view === v.id} class:active={view === v.id} onclick={() => (view = v.id)}
          >{v.label}</button
        >
      {/each}
    </div>
    <div class="body">
      {#if loadError !== null}
        <p class="error" role="alert">{loadError}</p>
      {:else if loaded && summary === null}
        <p class="empty">Aún no hay sesiones guardadas.</p>
      {:else if summary !== null}
        {#if view === "sessions"}
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
              {chartCaption(chart.wpm.length)}:
              <span class="key wpm">PPM netas</span>
              <span class="key accuracy">Precisión</span>
            </figcaption>
            <svg
              viewBox="0 0 {BOX.width} {BOX.height}"
              role="img"
              aria-labelledby="history-chart-title"
            >
              <title id="history-chart-title"
                >Evolución de las PPM netas y la precisión, de la sesión más antigua a la más reciente</title
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
        {:else if view === "days"}
          <figure class="chart">
            <figcaption>
              {dayCaption}:
              <span class="key wpm">PPM netas</span>
              <span class="key minutes">Minutos</span>
            </figcaption>
            <svg viewBox="0 0 {DAY_BOX.width} {DAY_BOX.height}" role="img" aria-labelledby="history-day-title">
              <title id="history-day-title"
                >PPM netas medias y minutos practicados por día, del día más antiguo al más reciente</title
              >
              {#each dayChart.ticks as t (t.y)}
                <line class="grid" x1={DAY_BOX.left} x2={DAY_BOX.width - DAY_BOX.right} y1={t.y} y2={t.y} />
                <text class="tick" x={DAY_BOX.left - 6} y={t.y} text-anchor="end" dominant-baseline="middle">{t.wpm}</text>
                <text class="tick" x={DAY_BOX.width - DAY_BOX.right + 6} y={t.y} dominant-baseline="middle"
                  >{t.minutes} min</text
                >
              {/each}
              {#each dayChart.bars as b (b.day)}
                <rect class="bar minutes" x={b.x} y={b.y} width={b.width} height={b.height}>
                  <title>{dayLabels.get(b.day)}</title>
                </rect>
              {/each}
              <polyline class="line wpm" points={dayChart.line} fill="none" stroke-width="2" />
              {#each dayChart.points as p (p.day)}
                <circle class="dot wpm" cx={p.x} cy={p.y} r="2.5">
                  <title>{dayLabels.get(p.day)}</title>
                </circle>
              {/each}
              {#if dayRows.length > 0}
                <text class="tick" x={DAY_BOX.left} y={DAY_BOX.height - 4}>{dayRows[dayRows.length - 1].date}</text>
                <text class="tick" x={DAY_BOX.width - DAY_BOX.right} y={DAY_BOX.height - 4} text-anchor="end"
                  >{dayRows[0].date}</text
                >
              {/if}
            </svg>
          </figure>

          <table>
            <thead>
              <tr>
                <th scope="col">Día</th>
                <th scope="col" class="num">Sesiones</th>
                <th scope="col" class="num">Minutos</th>
                <th scope="col" class="num">PPM netas</th>
                <th scope="col" class="num">Precisión</th>
              </tr>
            </thead>
            <tbody>
              {#each dayRows as r (r.day)}
                <tr>
                  <td>{r.date}</td>
                  <td class="num">{r.sessions}</td>
                  <td class="num">{r.minutes}</td>
                  <td class="num">{r.netWpm}</td>
                  <td class="num">{r.accuracy}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else if charRows.length === 0}
          <p class="empty">Aún no hay bastantes pulsaciones por carácter.</p>
        {:else}
          <figure class="chart">
            <figcaption>
              {charCaption(charChart.bars.length)}:
              <span class="key error">Error</span>
            </figcaption>
            <svg viewBox="0 0 {CHAR_BOX.width} {CHAR_BOX.height}" role="img" aria-labelledby="history-char-title">
              <title id="history-char-title">Error de los caracteres con más errores, de peor a mejor</title>
              {#each charChart.ticks as t (t.y)}
                <line class="grid" x1={CHAR_BOX.left} x2={CHAR_BOX.width - CHAR_BOX.right} y1={t.y} y2={t.y} />
                <text class="tick" x={CHAR_BOX.left - 6} y={t.y} text-anchor="end" dominant-baseline="middle"
                  >{t.error}</text
                >
              {/each}
              {#each charChart.bars as b, i (b.unit)}
                <rect class="bar error" x={b.x} y={b.y} width={b.width} height={b.height}>
                  <title>{b.label}: {charRows[i].error}</title>
                </rect>
                <text
                  class="tick unit"
                  x={b.labelX}
                  y={CHAR_LABEL_Y}
                  text-anchor={b.label.length > 1 ? "end" : "middle"}
                  transform={b.label.length > 1 ? `rotate(-45 ${b.labelX} ${CHAR_LABEL_Y})` : undefined}
                  >{b.label}</text
                >
              {/each}
            </svg>
          </figure>

          <table>
            <thead>
              <tr>
                <th scope="col">Carácter</th>
                <th scope="col" class="num">Muestras</th>
                <th scope="col" class="num">Error</th>
                <th scope="col" class="num">Lentitud</th>
              </tr>
            </thead>
            <tbody>
              {#each charRows as r (r.unit)}
                <tr>
                  <td class="unit">{r.label}</td>
                  <td class="num">{r.samples}</td>
                  <td class="num">{r.error}</td>
                  <td class="num">{r.slowness}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
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
  /* Inside the button: an outside ring would overlap the joined neighbours. */
  .filters button:focus-visible {
    position: relative;
    z-index: 1;
    outline-offset: -3px;
  }
  .filters button.active:focus-visible {
    outline-color: var(--on-accent);
  }
  .filters .active {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  /* The view switch sits under the header, built like the filter. */
  .views {
    flex: none;
    padding: 0.5rem 1.25rem 0;
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
  .key.minutes::before {
    height: 0.6rem;
    background: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .key.error::before {
    height: 0.6rem;
    background: var(--danger);
  }
  .bar.minutes {
    fill: color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .bar.error {
    fill: var(--danger);
  }
  .tick.unit {
    font-family: var(--mono, monospace);
  }
  td.unit {
    font-family: var(--mono, monospace);
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
