<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getLayout, keySequences } from "../keyboard/api";
  import { type CharScore, heatScores } from "../keyboard/heat";
  import Keyboard from "../keyboard/Keyboard.svelte";
  import { type Flash, highlightsFor, keyForChar } from "../keyboard/keys";
  import type { KeyPress, Layout } from "../keyboard/types";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";
  import {
    LAYOUT,
    type PracticeText,
    type ScoredSession,
    type Weakness,
    firstText,
    heatmap as fetchHeatmap,
    nextText,
    scoreSession,
    weaknesses,
  } from "./api";
  import { type ErrorMode, type InputEvent as TypingEvent, TypingSession, acceptsInput, keyAction, textToEvents } from "./engine";
  import { focusTyping, setTypingField } from "./focus";
  import Results from "./Results.svelte";

  let {
    mode,
    liveStats,
    showKeyboard,
    flashes,
    heatmap,
    targetCount,
    textLength,
  }: {
    mode: ErrorMode;
    liveStats: boolean;
    showKeyboard: boolean;
    flashes: boolean;
    heatmap: boolean;
    targetCount: number;
    textLength: number;
  } = $props();

  type Phase = "loading" | "typing" | "scoring" | "results" | "error";
  let phase = $state<Phase>("loading");
  let text = $state("");
  // Weak units the current text practises, and whether it is the diagnostic text.
  let targets = $state.raw<string[]>([]);
  let diagnostic = $state(false);
  let weak = $state.raw<Weakness[]>([]);
  let charScores = $state.raw<CharScore[]>([]);
  let session = $state.raw<TypingSession | null>(null);
  // Bumped after each applied event: the session object itself is not reactive.
  let tick = $state(0);
  let now = $state(0);
  let scored = $state.raw<ScoredSession | null>(null);
  let message = $state("");
  let retry: () => void = loadFirst;
  // Invalidates in-flight IPC results when the user restarts or moves on.
  let run = 0;
  let field = $state<HTMLTextAreaElement>();
  let layout = $state.raw<Layout | null>(null);
  // Key presses for each char of `text`, fetched once per text.
  let sequences = $state.raw<(KeyPress[] | null)[] | null>(null);
  let flash = $state.raw<Flash | null>(null);

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
  const heat = $derived(heatmap && layout ? heatScores(layout, charScores) : null);
  const highlights = $derived(phase === "typing" && view && sequences ? highlightsFor(sequences[view.cursor]) : []);

  function restart() {
    if (!text) return;
    run++;
    session = new TypingSession(text, mode);
    scored = null;
    phase = "typing";
    tick++;
  }

  async function load(fetch: () => Promise<PracticeText>, again: () => void) {
    const id = ++run;
    phase = "loading";
    try {
      const next = await fetch();
      if (id !== run) return;
      text = next.text;
      targets = next.targets;
      diagnostic = next.diagnostic ?? false;
      restart();
    } catch (err) {
      if (id !== run) return;
      retry = again;
      message = `No se pudo cargar el texto: ${String(err)}`;
      phase = "error";
    }
  }

  function loadFirst() {
    void load(firstText, loadFirst);
  }

  // Settings are read here, so a change affects the next text only.
  function loadNext() {
    const fallback = scored?.failedChars ?? [];
    const seed = Math.floor(Math.random() * 2 ** 32);
    const [count, length] = [targetCount, textLength];
    const again = () => void load(() => nextText(seed, count, length, fallback), again);
    again();
  }

  // Weaknesses and heatmap scores from the stored model; missing ones only hide those views.
  function refreshModel(id = run) {
    weaknesses(5)
      .then((next) => id === run && (weak = next))
      .catch((err) => console.warn("weaknesses failed", err));
    fetchHeatmap()
      .then((next) => (charScores = next))
      .catch((err) => console.warn("heatmap failed", err));
  }

  async function finish(done: TypingSession) {
    const id = ++run;
    phase = "scoring";
    try {
      // Wall-clock start: `performance.now()` stamps are relative to the page's time origin.
      const startedAtMs = Math.round(performance.timeOrigin + (done.firstT ?? performance.now()));
      const result = await scoreSession(text, done.mode, done.events, startedAtMs);
      if (id !== run) return;
      scored = result;
      weak = [];
      phase = "results";
      refreshModel(id);
    } catch (err) {
      if (id !== run) return;
      retry = () => void finish(done);
      message = `No se pudo calcular el resultado: ${String(err)}`;
      phase = "error";
    }
  }

  const isFormControl = (target: EventTarget | null) =>
    target !== field && target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);

  function applyEvent(event: TypingEvent) {
    if (phase !== "typing" || !session) return;
    const expected = session.chars[session.cursor];
    if (!session.apply(event)) return;
    now = event.tMs;
    tick++;
    if (event.kind === "char" && flashes && layout) {
      const code = keyForChar(layout, event.ch);
      if (code) flash = { code, ok: event.ch === expected };
    }
    if (session.completed) void finish(session);
  }

  // Typed chars come only from text the field received; reading and clearing it makes the
  // `input` and `compositionend` paths idempotent.
  function takeText() {
    if (!field) return;
    const value = field.value;
    field.value = "";
    for (const event of textToEvents(value, performance.now())) applyEvent(event);
  }

  function onInput(e: Event) {
    const input = e as globalThis.InputEvent;
    if (input.isComposing) return;
    if (acceptsInput(input.inputType)) takeText();
    else if (field) field.value = "";
  }

  function onKeydown(e: KeyboardEvent) {
    if (isFormControl(e.target)) return;
    const action = keyAction(e);
    // Escape restarts only a text being typed or reviewed; while a text loads or is scored,
    // restarting would silently discard the pending result.
    if (action === "escape" && (phase === "typing" || phase === "results")) {
      e.preventDefault();
      restart();
      return;
    }
    if (action === "enter") {
      // Enter on a focused button keeps its native activation (e.g. "Repetir").
      if (phase === "results" && !(e.target instanceof HTMLButtonElement)) {
        e.preventDefault();
        loadNext();
      } else if (e.target === field) {
        // Not typeable yet: no practice text contains a newline.
        e.preventDefault();
      }
      return;
    }
    if (e.target !== field) return;
    if (action === "tab" && phase === "typing") e.preventDefault();
    if (action === "backspace") {
      e.preventDefault();
      applyEvent({ kind: "backspace", tMs: performance.now() });
    }
  }

  // Any click outside a settings control, or focus lost to the page, returns to the typing field.
  function onWindowClick(e: MouseEvent) {
    if (e.target instanceof Element && e.target.closest("select, input, label")) return;
    focusTyping();
  }

  function refocusIfLost() {
    setTimeout(() => {
      if (!document.activeElement || document.activeElement === document.body) focusTyping();
    });
  }

  $effect(() => {
    const current = text;
    sequences = null;
    if (!current) return;
    let live = true;
    keySequences(LAYOUT, current)
      .then((next) => {
        if (live) sequences = next;
      })
      .catch((err) => console.warn("key_sequences failed; no key highlight", err));
    return () => (live = false);
  });

  // A phase change can remove the focused button (e.g. "Siguiente texto").
  $effect(() => {
    void phase;
    refocusIfLost();
  });

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

  onMount(() => {
    setTypingField(field ?? null);
    focusTyping();
    loadFirst();
    refreshModel();
    getLayout(LAYOUT)
      .then((next) => (layout = next))
      .catch((err) => console.warn("get_layout failed; keyboard without labels", err));
    return () => setTypingField(null);
  });

  // WebKit's `autocorrect` is missing from Svelte's textarea attribute types.
  const noAutocorrect: Record<string, string> = { autocorrect: "off" };

  const focusAnd = (action: () => void) => () => {
    focusTyping();
    action();
  };
</script>

<svelte:window onkeydown={onKeydown} onclick={onWindowClick} />
<svelte:document onfocusout={refocusIfLost} />

<textarea
  bind:this={field}
  class="capture"
  aria-label="Entrada de mecanografía"
  autocomplete="off"
  {...noAutocorrect}
  autocapitalize="off"
  spellcheck="false"
  oninput={onInput}
  oncompositionend={takeText}
></textarea>

<main>
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
        <button type="button" onclick={focusAnd(() => retry())}>Reintentar</button>
      </div>
    {:else if phase === "results" && scored}
      <Results {scored} {weak} onNext={loadNext} onRepeat={restart} />
    {:else if session && view}
      <p class="kind">
        {diagnostic ? "Texto de diagnóstico" : "Texto adaptativo"}
        {#if targets.length > 0}
          · Practicando:
          {#each targets as t, i (i)}{#if i > 0}<span class="sep">·</span>{/if}<kbd>{charLabel(t)}</kbd>{/each}
        {/if}
      </p>
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
</main>

{#if showKeyboard}
  <footer>
    <Keyboard {layout} {highlights} {flash} {heat} />
  </footer>
{/if}

<style>
  /* Focusable but invisible: it only collects typed and composed text. */
  .capture {
    position: fixed;
    left: 0;
    bottom: 0;
    width: 1px;
    height: 1px;
    padding: 0;
    border: 0;
    opacity: 0;
    resize: none;
    overflow: hidden;
    pointer-events: none;
  }
  main {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: clamp(1rem, 5vh, 4rem) 2rem;
  }
  footer {
    flex: none;
    padding: clamp(0.25rem, 1.5vh, 1rem) 1.5rem;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }
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
    /* Also scales with the height, so the text and the keyboard fit at 720×480. */
    font-size: clamp(1rem, min(2.4vw, 4vh), 2rem);
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
  /* A soft tint with an underline bar: marks the next char without pulling the eye. */
  .cursor {
    color: var(--text);
    background: color-mix(in srgb, var(--cursor) 18%, transparent);
    box-shadow: inset 0 -0.12em var(--cursor);
    border-radius: 0.1em;
  }
  .cursor.error {
    background: color-mix(in srgb, var(--text-error) 22%, transparent);
    box-shadow: inset 0 -0.12em var(--text-error);
  }
  .kind {
    margin: 0 0 -0.75rem;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .kind kbd {
    display: inline-block;
    margin-left: 0.3rem;
    padding: 0 0.35em;
    font-family: var(--mono);
    color: var(--key-fg);
    background: var(--key-bg);
    border: 1px solid var(--key-border);
    border-radius: 0.3em;
  }
  .kind .sep {
    margin-left: 0.3rem;
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
