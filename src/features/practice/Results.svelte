<script lang="ts">
  import { heatLevel } from "../keyboard/heat";
  import type { ScoredSession, Weakness } from "./api";
  import { focusTyping } from "./focus";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";
  import { rhythmSummary } from "../metronome/beat";

  let {
    scored,
    weak,
    language = null,
    onNext,
    onRepeat,
  }: {
    scored: ScoredSession;
    weak: Weakness[];
    /** Name of the code language; `null` for prose. */
    language?: string | null;
    onNext: () => void;
    onRepeat: () => void;
  } = $props();
  const stats = $derived(scored.result.stats);
  const rhythm = $derived(rhythmSummary(scored.offsetsMs ?? []));

  const focusAnd = (action: () => void) => () => {
    focusTyping();
    action();
  };
</script>

<section class="results" aria-labelledby="results-title">
  <h2 id="results-title">Resultado{#if language}<span class="lang"> · Código · {language}</span>{/if}</h2>
  <dl class="figures">
    <div class="main"><dt>PPM netas</dt><dd>{formatWpm(stats.netWpm)}</dd></div>
    <div><dt>PPM brutas</dt><dd>{formatWpm(stats.grossWpm)}</dd></div>
    <div><dt>Precisión</dt><dd>{formatPercent(stats.accuracy)}</dd></div>
    <div><dt>Tiempo</dt><dd>{formatTime(stats.durationMs)}</dd></div>
    <div><dt>Errores</dt><dd>{stats.errors}</dd></div>
  </dl>
  {#if rhythm}
    <p class="rhythm">
      Ritmo: desvío medio ±{Math.round(rhythm.meanAbsMs)} ms
      <span class="note">· adelantadas {Math.round(rhythm.earlyPct)}% / atrasadas {Math.round(rhythm.latePct)}%</span>
    </p>
  {/if}
  {#if scored.failedChars.length > 0}
    <p class="failed">
      Caracteres fallados:
      {#each scored.failedChars as c, i (i)}<kbd>{charLabel(c)}</kbd>{/each}
    </p>
  {:else}
    <p class="note">Sin fallos.</p>
  {/if}
  {#if weak.length > 0}
    <div class="weak">
      <h3>{language ? "Tus puntos débiles en código" : "Tus puntos débiles"}</h3>
      <ol>
        {#each weak as w (w.unit)}
          <li><kbd>{charLabel(w.unit)}</kbd><span class="bar"><span class="fill fill-{heatLevel(w.score)}"></span></span></li>
        {/each}
      </ol>
      <p class="note">El siguiente texto los practica.</p>
    </div>
  {/if}
  {#if !scored.saved && scored.saveError}
    <p class="note" role="status">No se pudo guardar la sesión: {scored.saveError}</p>
  {/if}
  <div class="actions">
    <button type="button" class="primary" onclick={focusAnd(onNext)}>Siguiente texto <kbd>Intro</kbd></button>
    <button type="button" onclick={focusAnd(onRepeat)}>Repetir <kbd>Esc</kbd></button>
  </div>
</section>

<style>
  .results {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--muted);
    font-weight: 600;
  }
  .figures {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 1rem 2.5rem;
    margin: 0;
  }
  dt {
    color: var(--muted);
    font-size: 0.9rem;
  }
  dd {
    margin: 0;
    font-family: var(--mono);
    font-size: 1.75rem;
    font-variant-numeric: tabular-nums;
  }
  .main dd {
    font-size: 3.5rem;
    line-height: 1;
    color: var(--accent);
  }
  .lang {
    font-weight: 400;
  }
  .failed,
  .rhythm,
  .note {
    margin: 0;
  }
  .note {
    color: var(--muted);
  }
  .failed kbd {
    margin-left: 0.4rem;
  }
  kbd {
    display: inline-block;
    min-width: 1.4em;
    padding: 0.1em 0.4em;
    font-family: var(--mono);
    text-align: center;
    color: var(--key-fg);
    background: var(--key-bg);
    border: 1px solid var(--key-border);
    border-radius: 0.3em;
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.95rem;
    font-weight: 600;
  }
  ol {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    margin: 0 0 0.5rem;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  /* Score bar: classes, not style attributes (CSP), one per tenth. */
  .bar {
    width: 4rem;
    height: 0.4rem;
    background: var(--surface-2);
    border-radius: 0.2rem;
  }
  .fill {
    display: block;
    width: 0;
    height: 100%;
    background: var(--danger);
    border-radius: inherit;
  }
  .fill-1 { width: 10%; }
  .fill-2 { width: 20%; }
  .fill-3 { width: 30%; }
  .fill-4 { width: 40%; }
  .fill-5 { width: 50%; }
  .fill-6 { width: 60%; }
  .fill-7 { width: 70%; }
  .fill-8 { width: 80%; }
  .fill-9 { width: 90%; }
  .fill-10 { width: 100%; }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }
  .primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  .primary kbd {
    color: inherit;
    background: transparent;
    border-color: currentColor;
  }
</style>
