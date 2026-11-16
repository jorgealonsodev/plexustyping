<script lang="ts">
  import type { ScoredSession } from "./api";
  import { focusTyping } from "./focus";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";

  let { scored, onNext, onRepeat }: { scored: ScoredSession; onNext: () => void; onRepeat: () => void } = $props();
  const stats = $derived(scored.result.stats);

  const focusAnd = (action: () => void) => () => {
    focusTyping();
    action();
  };
</script>

<section class="results" aria-labelledby="results-title">
  <h2 id="results-title">Resultado</h2>
  <dl class="figures">
    <div class="main"><dt>PPM netas</dt><dd>{formatWpm(stats.netWpm)}</dd></div>
    <div><dt>PPM brutas</dt><dd>{formatWpm(stats.grossWpm)}</dd></div>
    <div><dt>Precisión</dt><dd>{formatPercent(stats.accuracy)}</dd></div>
    <div><dt>Tiempo</dt><dd>{formatTime(stats.durationMs)}</dd></div>
    <div><dt>Errores</dt><dd>{stats.errors}</dd></div>
  </dl>
  {#if scored.failedChars.length > 0}
    <p class="failed">
      Caracteres fallados:
      {#each scored.failedChars as c, i (i)}<kbd>{charLabel(c)}</kbd>{/each}
    </p>
    <p class="note">El siguiente texto practica estos caracteres.</p>
  {:else}
    <p class="note">Sin fallos. El siguiente texto será de práctica general.</p>
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
  .failed,
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
