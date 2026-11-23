<script lang="ts">
  import { heatLevel } from "../keyboard/heat";
  import type { ScoredSession, Weakness } from "./api";
  import { focusTyping } from "./focus";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";
  import { type TempoChange, rhythmSummary, tempoNotice } from "../metronome/beat";
  import { goalMetNotice } from "../goals/goals";
  import { newRecordLine, recordLines } from "./ghost";
  import { t } from "./messages";
  import { unlockNotice } from "../unlock/unlock";

  let {
    scored,
    weak,
    language = null,
    tempo = null,
    goalMet = false,
    record = null,
    onNext,
    onRepeat,
  }: {
    scored: ScoredSession;
    weak: Weakness[];
    /** Name of the code language; `null` for prose. */
    language?: string | null;
    /** What the adaptive metronome decided after this text; `null` when it did not apply. */
    tempo?: TempoChange | null;
    /** This session met the daily goal: "¡Meta diaria cumplida!" is shown (S5). */
    goalMet?: boolean;
    /** Net PPM of the ghost this text raced (ghost S5); `null` when it raced none. */
    record?: number | null;
    onNext: () => void;
    onRepeat: () => void;
  } = $props();
  const stats = $derived(scored.result.stats);
  const records = $derived(record === null ? [] : recordLines(record, scored.result.stats.netWpm, scored.saved));
  const rhythm = $derived(rhythmSummary(scored.offsetsMs ?? []));
  /** "Nueva tecla desbloqueada: e" when this session unlocked a key (S6). */
  const unlocked = $derived(unlockNotice(scored.unlocked));

  const focusAnd = (action: () => void) => () => {
    focusTyping();
    action();
  };
</script>

<section class="results" aria-labelledby="results-title">
  <h2 id="results-title">{t("resultTitle")}{#if language}<span class="lang"> · {t("code")} · {language}</span>{/if}</h2>
  {#if goalMet}<p class="goal-met" role="status">{goalMetNotice()}</p>{/if}
  {#if unlocked}<p class="goal-met" role="status">{unlocked}</p>{/if}
  <dl class="figures">
    <div class="main"><dt>{t("netWpm")}</dt><dd>{formatWpm(stats.netWpm)}</dd></div>
    <div><dt>{t("grossWpm")}</dt><dd>{formatWpm(stats.grossWpm)}</dd></div>
    <div><dt>{t("accuracy")}</dt><dd>{formatPercent(stats.accuracy)}</dd></div>
    <div><dt>{t("time")}</dt><dd>{formatTime(stats.durationMs)}</dd></div>
    <div><dt>{t("errors")}</dt><dd>{stats.errors}</dd></div>
  </dl>
  {#each records as line (line)}<p class="record" class:new-record={line === newRecordLine()} role="status">{line}</p>{/each}
  {#if rhythm}
    <p class="rhythm">
      {t("rhythm", { ms: Math.round(rhythm.meanAbsMs) })}
      <span class="note">{t("rhythmShares", { early: Math.round(rhythm.earlyPct), late: Math.round(rhythm.latePct) })}</span>
    </p>
  {/if}
  {#if tempo}<p class="rhythm">{tempoNotice(tempo)}</p>{/if}
  {#if scored.failedChars.length > 0}
    <p class="failed">
      {t("failedChars")}
      {#each scored.failedChars as c, i (i)}<kbd>{charLabel(c)}</kbd>{/each}
    </p>
  {:else}
    <p class="note">{t("noFailures")}</p>
  {/if}
  {#if weak.length > 0}
    <div class="weak">
      <h3>{language ? t("weakCode") : t("weak")}</h3>
      <ol>
        {#each weak as w (w.unit)}
          <li><kbd>{charLabel(w.unit)}</kbd><span class="bar"><span class="fill fill-{heatLevel(w.score)}"></span></span></li>
        {/each}
      </ol>
      <p class="note">{t("weakNote")}</p>
    </div>
  {/if}
  {#if !scored.saved && scored.saveError}
    <p class="note" role="status">{t("saveFailed", { error: scored.saveError })}</p>
  {/if}
  <div class="actions">
    <button type="button" class="primary" onclick={focusAnd(onNext)}>{t("nextText")} <kbd>{t("enterKey")}</kbd></button>
    <button type="button" onclick={focusAnd(onRepeat)}>{t("repeat")} <kbd>Esc</kbd></button>
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
  .note,
  .goal-met,
  .record {
    margin: 0;
  }
  .new-record {
    font-weight: 700;
    color: var(--accent);
  }
  .goal-met {
    font-weight: 700;
    color: var(--accent);
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
