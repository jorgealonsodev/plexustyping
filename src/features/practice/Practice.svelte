<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { diagnosticText, nextText, scoreSession, type ScoredSession } from "./api";
  import { type ErrorMode, TypingSession, keyToEvent } from "./engine";
  import { formatPercent, formatTime, formatWpm } from "./format";
  import Results from "./Results.svelte";

  let { mode, liveStats }: { mode: ErrorMode; liveStats: boolean } = $props();

  type Phase = "loading" | "typing" | "scoring" | "results" | "error";
  let phase = $state<Phase>("loading");
  let text = $state("");
  let session = $state.raw<TypingSession | null>(null);
  // Bumped after each applied event: the session object itself is not reactive.
  let tick = $state(0);
  let now = $state(0);
  let scored = $state.raw<ScoredSession | null>(null);
  let message = $state("");
  let retry: () => void = loadDiagnostic;
  // Invalidates in-flight IPC results when the user restarts or moves on.
  let run = 0;

  const view = $derived.by(() => {
    void tick;
    if (!session) return null;
    const s = session;
    return { states: s.chars.map((_, i) => s.state(i)), cursor: s.cursor, stats: s.result().stats };
  });
  const elapsed = $derived.by(() => {
    void tick;
    if (!session || session.firstT === null) return 0;
    return phase === "typing" ? Math.max(now - session.firstT, 0) : (view?.stats.durationMs ?? 0);
  });

  function restart() {
    if (!text) return;
    run++;
    session = new TypingSession(text, mode);
    scored = null;
    phase = "typing";
    tick++;
  }

  async function load(fetch: () => Promise<string>, again: () => void) {
    const id = ++run;
    phase = "loading";
    try {
      const next = await fetch();
      if (id !== run) return;
      text = next;
      restart();
    } catch (err) {
      if (id !== run) return;
      retry = again;
      message = `No se pudo cargar el texto: ${String(err)}`;
      phase = "error";
    }
  }

  function loadDiagnostic() {
    void load(diagnosticText, loadDiagnostic);
  }

  function loadNext() {
    const targets = scored?.failedChars ?? [];
    const seed = Math.floor(Math.random() * 2 ** 32);
    const again = () => void load(() => nextText(targets, seed), again);
    again();
  }

  async function finish(done: TypingSession) {
    const id = ++run;
    phase = "scoring";
    try {
      const result = await scoreSession(text, done.mode, done.events);
      if (id !== run) return;
      scored = result;
      phase = "results";
    } catch (err) {
      if (id !== run) return;
      retry = () => void finish(done);
      message = `No se pudo calcular el resultado: ${String(err)}`;
      phase = "error";
    }
  }

  const isFormControl = (target: EventTarget | null) =>
    target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);

  function onKeydown(e: KeyboardEvent) {
    if (isFormControl(e.target)) return;
    // Escape restarts only a text being typed or reviewed; while a text loads or is scored,
    // restarting would silently discard the pending result.
    if (e.key === "Escape" && (phase === "typing" || phase === "results")) {
      e.preventDefault();
      restart();
      return;
    }
    // Enter on a focused button keeps its native activation (e.g. "Repetir").
    if (phase === "results" && e.key === "Enter" && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      loadNext();
      return;
    }
    if (phase !== "typing" || !session) return;
    const event = keyToEvent(e, performance.now());
    if (!event) return;
    e.preventDefault();
    if (!session.apply(event)) return;
    now = event.tMs;
    tick++;
    if (session.completed) void finish(session);
  }

  // Changing the error mode restarts the current text.
  $effect(() => {
    void mode;
    untrack(restart);
  });

  // Live clock while a text is being typed.
  $effect(() => {
    if (!liveStats || phase !== "typing") return;
    const timer = setInterval(() => (now = performance.now()), 250);
    return () => clearInterval(timer);
  });

  onMount(loadDiagnostic);

  const blurAnd = (action: () => void) => (e: MouseEvent) => {
    (e.currentTarget as HTMLElement).blur();
    action();
  };
</script>

<svelte:window onkeydown={onKeydown} />

<section class="practice">
  {#if liveStats && view && phase === "typing"}
    <dl class="live" aria-label="Estadísticas en vivo">
      <div><dt>PPM</dt><dd>{formatWpm(view.stats.netWpm)}</dd></div>
      <div><dt>Pulsaciones/min</dt><dd>{formatWpm(view.stats.kpm)}</dd></div>
      <div><dt>Precisión</dt><dd>{formatPercent(view.stats.accuracy)}</dd></div>
      <div><dt>Tiempo</dt><dd>{formatTime(elapsed)}</dd></div>
    </dl>
  {/if}

  {#if phase === "loading"}
    <p class="status">Cargando texto…</p>
  {:else if phase === "error"}
    <div class="status" role="alert">
      <p>{message}</p>
      <button type="button" onclick={blurAnd(() => retry())}>Reintentar</button>
    </div>
  {:else if phase === "results" && scored}
    <Results {scored} onNext={loadNext} onRepeat={restart} />
  {:else if session && view}
    <!-- No whitespace inside: the text uses white-space: pre-wrap. -->
    <p class="text" aria-label="Texto de práctica">{#each session.chars as c, i (i)}<span
          class="ch {view.states[i]}"
          class:space={c === " "}
          class:cursor={i === view.cursor}>{c}</span
        >{/each}</p>
    <p class="hint">
      {#if phase === "scoring"}Calculando resultado…{:else if view.cursor === 0}Empieza a escribir cuando quieras · Esc reinicia{:else}Esc reinicia el texto{/if}
    </p>
  {/if}
</section>

<style>
  .practice {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    width: min(100%, 62rem);
    margin: 0 auto;
  }
  .live {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 2rem;
    margin: 0;
    color: var(--muted);
  }
  .live div {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .live dt {
    font-size: 0.85rem;
  }
  .live dd {
    margin: 0;
    color: var(--text);
    font-family: var(--mono);
    font-size: 1.25rem;
    font-variant-numeric: tabular-nums;
  }
  .text {
    margin: 0;
    font-family: var(--mono);
    font-size: clamp(1.4rem, 2.4vw, 2rem);
    line-height: 1.75;
    white-space: pre-wrap;
    overflow-wrap: break-word;
  }
  .ch {
    border-radius: 0.15em;
  }
  .pending {
    color: var(--text-pending);
  }
  .correct {
    color: var(--text-correct);
  }
  .error {
    color: var(--text-error);
    background: var(--error-bg);
  }
  .space.error {
    box-shadow: inset 0 -0.15em var(--text-error);
  }
  .cursor {
    color: var(--cursor-text);
    background: var(--cursor);
  }
  .cursor.error {
    background: var(--text-error);
  }
  .hint,
  .status {
    margin: 0;
    color: var(--muted);
  }
  .status p {
    margin: 0 0 1rem;
    color: var(--danger);
  }
</style>
