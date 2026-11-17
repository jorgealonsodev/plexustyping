<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getLayout, keySequences } from "../keyboard/api";
  import { type CharScore, heatScores } from "../keyboard/heat";
  import Keyboard from "../keyboard/Keyboard.svelte";
  import { type Flash, highlightsFor, keyForChar } from "../keyboard/keys";
  import type { KeyPress, Layout } from "../keyboard/types";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";
  import {
    type PracticeText,
    type ScoredSession,
    type Weakness,
    firstText,
    heatmap as fetchHeatmap,
    nextText,
    scoreSession,
    weaknesses,
  } from "./api";
  import {
    type ErrorMode,
    type IndentMode,
    type InputEvent as TypingEvent,
    TypingSession,
    acceptsInput,
    keyAction,
    textToEvents,
  } from "./engine";
  import { type PracticeMode, practiceKey } from "../code/api";
  import CodeText from "../code/CodeText.svelte";
  import { LANGUAGES } from "../code/languages.svelte";
  import { type TokenClass, codeKeyEvent, tokenClasses } from "../code/syntax";
  import { focusTyping, setTypingField } from "./focus";
  import { Prefetcher, generationKey, provisionalFailedChars, sourceLabel } from "./prefetch";
  import { type Phase, indentChangeRestarts, shouldApply } from "./replies";
  import Results from "./Results.svelte";
  import { metronomeApi } from "../metronome/api";
  import {
    type BeatAnchor,
    type MetronomeSettings,
    type TempoChange,
    beatOffsetMs,
    tempoChangeFor,
    toConfig,
  } from "../metronome/beat";
  import BeatDot from "../metronome/BeatDot.svelte";
  import { MetronomeController } from "../metronome/controller";

  let {
    mode,
    liveStats,
    showKeyboard,
    flashes,
    heatmap,
    targetCount,
    textLength,
    aiRevision,
    metronome,
    onTempo,
    practice,
    indent,
    codeLines,
    layoutId,
    languageId,
  }: {
    mode: ErrorMode;
    liveStats: boolean;
    showKeyboard: boolean;
    flashes: boolean;
    heatmap: boolean;
    targetCount: number;
    textLength: number;
    /** Changes when AI settings or keys change. */
    aiRevision: number;
    metronome: MetronomeSettings;
    /** The adaptive metronome chose a new tempo for the next text. */
    onTempo: (bpm: number) => void;
    /** Prose or code of a language (S1); a change loads a new text. */
    practice: PracticeMode;
    /** Code indentation (S4); a change restarts a code text. */
    indent: IndentMode;
    /** Lines per code snippet, from the next text. */
    codeLines: number;
    /** Keyboard layout id (S1–S4); a change loads a new text. */
    layoutId: string;
    /** Prose language id (S1, S4); a change loads a new prose text. */
    languageId: string;
  } = $props();

  // The metronome plays only while a text is ready or being typed and the window is visible (S7).
  let visible = $state(!document.hidden);
  let anchor = $state.raw<BeatAnchor | null>(null);
  let metronomeError = $state<string | null>(null);
  const controller = new MetronomeController(metronomeApi, () => {
    anchor = controller.anchor;
    metronomeError = controller.error;
  });
  /** Metronome offset of each event of the current session (aligned with `session.events`) and the tempos heard. */
  type Track = { offsets: (number | null)[]; bpms: Set<number>; switched: boolean };
  let track: Track = { offsets: [], bpms: new Set(), switched: false };

  let phase = $state<Phase>("loading");
  let text = $state("");
  // Weak units the current text practises, and whether it is the diagnostic text.
  let targets = $state.raw<string[]>([]);
  let diagnostic = $state(false);
  // Origin of the current text, and why the AI was not used (when enabled).
  let source = $state("");
  let notice = $state<string | null>(null);
  let loadingLabel = $state("Cargando texto…");
  // Practice mode of the current text, and the syntax class of each of its chars (code only).
  let textMode = $state.raw<PracticeMode>("prose");
  let classes = $state.raw<TokenClass[]>([]);
  const isCode = $derived(textMode !== "prose");
  const practiceId = $derived(practiceKey(practice));
  // Mode, layout and language of the latest requested text: a different one loads a new text.
  // Code texts do not depend on the prose language (S1).
  const requestKey = (p: PracticeMode, layoutOf: string, languageOf: string) =>
    `${practiceKey(p)}|${layoutOf}|${p === "prose" ? languageOf : ""}`;
  const wantedKey = $derived(requestKey(practice, layoutId, languageId));
  let requestedKey = untrack(() => requestKey(practice, layoutId, languageId));
  // Layout and language the current text was generated for; the scored session records them (S5).
  let textLayout = untrack(() => layoutId);
  let textLanguage = untrack(() => languageId);
  // The next text, requested in the background while this one is typed (S6).
  const prefetch = new Prefetcher<PracticeText>();
  const genKey = $derived(
    generationKey(targetCount, textLength, aiRevision, practiceId, codeLines, layoutId, practice === "prose" ? languageId : "es"),
  );
  let weak = $state.raw<Weakness[]>([]);
  let charScores = $state.raw<CharScore[]>([]);
  let session = $state.raw<TypingSession | null>(null);
  // Bumped after each applied event: the session object itself is not reactive.
  let tick = $state(0);
  let now = $state(0);
  let scored = $state.raw<ScoredSession | null>(null);
  /** What the adaptive metronome decided after the scored text; `null` when it did not apply. */
  let tempo = $state.raw<TempoChange | null>(null);
  let message = $state("");
  let retry: () => void = loadFirst;
  // Invalidates in-flight IPC results when the user restarts or moves on.
  let run = 0;
  // Latest heatmap request: an older reply (e.g. of the previous mode) is stale.
  let heatmapRun = 0;
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
    session = new TypingSession(text, mode, textMode === "prose" ? "manual" : indent);
    track = { offsets: [], bpms: new Set(), switched: false };
    scored = null;
    phase = "typing";
    tick++;
  }

  async function load(
    fetch: () => Promise<PracticeText>,
    again: () => void,
    label = "Cargando texto…",
    textPractice: PracticeMode = practice,
  ) {
    const id = ++run;
    const [forLayout, forLanguage] = [layoutId, languageId];
    requestedKey = requestKey(textPractice, forLayout, forLanguage);
    loadingLabel = label;
    phase = "loading";
    try {
      const next = await fetch();
      if (id !== run) return;
      textMode = textPractice;
      textLayout = forLayout;
      textLanguage = forLanguage;
      classes = tokenClasses([...next.text].length, next.tokens);
      text = next.text;
      targets = next.targets;
      diagnostic = next.diagnostic ?? false;
      source = sourceLabel(next);
      notice = next.notice ?? null;
      restart();
    } catch (err) {
      if (id !== run) return;
      retry = again;
      message = `No se pudo cargar el texto: ${String(err)}`;
      phase = "error";
    }
  }

  function loadFirst() {
    const [chosen, lines, lay, lang] = [practice, codeLines, layoutId, languageId];
    void load(() => firstText(lang, lay, chosen, lines), loadFirst, "Cargando texto…", chosen);
  }

  const randomSeed = () => Math.floor(Math.random() * 2 ** 32);

  // Settings are read here, so a change affects the next text only. A prefetched text made
  // with the same settings is used (awaited if still pending); otherwise one is requested now.
  function loadNext() {
    // Failed chars of a text in another mode say nothing about this one.
    const fallback = practiceKey(textMode) === practiceId ? (scored?.failedChars ?? []) : [];
    const seed = randomSeed();
    const [count, length, chosen, lines, lay, lang] = [targetCount, textLength, practice, codeLines, layoutId, languageId];
    const fresh = () => nextText(lang, lay, seed, count, length, fallback, chosen, lines);
    const again = () => void load(fresh, again, "Generando texto…", chosen);
    const ready = prefetch.take(genKey);
    if (ready) void load(() => ready.catch(fresh), again, "Generando texto…", chosen);
    else again();
  }

  // Not while the diagnostic is typed: the model is empty until it is scored.
  function startPrefetch() {
    if (!session || diagnostic) return;
    const fallback = provisionalFailedChars(session.result().records);
    const [count, length, chosen, lines, lay, lang] = [targetCount, textLength, practice, codeLines, layoutId, languageId];
    prefetch.start(genKey, () => nextText(lang, lay, randomSeed(), count, length, fallback, chosen, lines));
  }

  // Weaknesses and heatmap scores from the stored model; missing ones only hide those views.
  // Of the model of `of`: prose and code statistics are kept apart (S5).
  function refreshModel(id = run, of: PracticeMode = textMode) {
    weaknesses(5, of)
      .then((next) => shouldApply(id, run) && (weak = next))
      .catch((err) => console.warn("weaknesses failed", err));
    // Not tied to `run`: a restart or the text loaded after a switch keeps the mode's heatmap.
    const heatId = ++heatmapRun;
    fetchHeatmap(of)
      .then((next) => shouldApply(heatId, heatmapRun) && (charScores = next))
      .catch((err) => console.warn("heatmap failed", err));
  }

  async function finish(done: TypingSession, measured: Track) {
    const id = ++run;
    phase = "scoring";
    try {
      // Wall-clock start: `performance.now()` stamps are relative to the page's time origin.
      const startedAtMs = Math.round(performance.timeOrigin + (done.firstT ?? performance.now()));
      const bpm = measured.bpms.size === 1 ? [...measured.bpms][0] : null;
      const typed = { mode: textMode, indent: done.indent };
      const result = await scoreSession(textMode === "prose" ? textLanguage : null, textLayout, text, done.mode, done.events, startedAtMs, measured.offsets, bpm, typed);
      if (id !== run) return;
      scored = result;
      tempo = metronome.adaptive ? tempoChangeFor(bpm, result.result.stats.accuracy, result.offsetsMs, measured.switched) : null;
      if (tempo && tempo.to !== tempo.from) onTempo(tempo.to);
      weak = [];
      phase = "results";
      refreshModel(id);
    } catch (err) {
      if (id !== run) return;
      retry = () => void finish(done, measured);
      message = `No se pudo calcular el resultado: ${String(err)}`;
      phase = "error";
    }
  }

  const isFormControl = (target: EventTarget | null) =>
    target !== field && target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);

  function applyEvent(event: TypingEvent) {
    if (phase !== "typing" || !session) return;
    const expected = session.chars[session.cursor];
    const first = session.firstT === null;
    // Chars and Tabs are measured keystrokes (records); backspaces are not.
    const beat = event.kind === "backspace" ? null : anchor;
    // Epoch ms, like beat 0: `performance.now()` stamps are relative to the page's time origin.
    track.offsets.push(beat && beatOffsetMs(performance.timeOrigin + event.tMs, beat.epochMs, beat.intervalMs));
    if (beat) track.bpms.add(beat.bpm);
    if (!session.apply(event)) return;
    now = event.tMs;
    tick++;
    if (first) startPrefetch();
    if (event.kind === "char" && flashes && layout) {
      const code = keyForChar(layout, event.ch);
      if (code) flash = { code, ok: event.ch === expected };
    }
    if (session.completed) void finish(session, track);
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
        // In code, Enter types a measured newline (S4); prose texts have none.
        e.preventDefault();
        const event = phase === "typing" ? codeKeyEvent(action, isCode, performance.now()) : null;
        if (event) applyEvent(event);
      }
      return;
    }
    if (e.target !== field) return;
    if (action === "tab" && phase === "typing") {
      e.preventDefault();
      const event = codeKeyEvent(action, isCode, performance.now());
      if (event) applyEvent(event);
    }
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
    const [current, lay] = [text, layoutId];
    sequences = null;
    if (!current) return;
    let live = true;
    keySequences(lay, current)
      .then((next) => {
        if (live) sequences = next;
      })
      .catch((err) => console.warn("key_sequences failed; no key highlight", err));
    return () => (live = false);
  });

  // The drawn keyboard follows the chosen layout; an older reply is stale.
  $effect(() => {
    const id = layoutId;
    let live = true;
    // Until it loads (or if it fails) the keyboard has no labels, never the previous layout's.
    layout = null;
    getLayout(id)
      .then((next) => live && (layout = next))
      .catch((err) => console.warn("get_layout failed; keyboard without labels", err));
    return () => (live = false);
  });

  // A phase change can remove the focused button (e.g. "Siguiente texto").
  $effect(() => {
    void phase;
    refocusIfLost();
  });

  // New generation settings make a held prefetch stale: request again if a text is under way.
  $effect(() => {
    void genKey;
    untrack(() => {
      // A practice, layout or language switch loads a new text instead: no request for the old one.
      if (wantedKey !== requestedKey) return;
      if (phase === "typing" && session?.firstT != null) startPrefetch();
    });
  });

  // Changing the error mode restarts the current text.
  $effect(() => {
    void mode;
    untrack(restart);
  });

  // Changing the code indentation restarts a code text being typed; otherwise it applies to the next one.
  $effect(() => {
    void indent;
    untrack(() => indentChangeRestarts(phase, isCode) && restart());
  });

  // Switching prose/code, the language or the layout drops the prefetch and loads a new text.
  $effect(() => {
    const key = wantedKey;
    untrack(() => {
      if (key === requestedKey) return;
      prefetch.clear();
      const [count, length, chosen, lines, lay, lang] = [targetCount, textLength, practice, codeLines, layoutId, languageId];
      const fresh = () => nextText(lang, lay, randomSeed(), count, length, [], chosen, lines);
      const again = () => void load(fresh, again, "Generando texto…", chosen);
      again();
      refreshModel(run, chosen);
    });
  });

  // Switching the metronome on or off after the first keystroke rules out a tempo change.
  let wasEnabled = untrack(() => metronome.enabled);
  $effect(() => {
    const enabled = metronome.enabled;
    untrack(() => {
      if (enabled !== wasEnabled && phase === "typing" && session?.firstT != null) track.switched = true;
      wasEnabled = enabled;
    });
  });

  $effect(() => {
    const wish = metronome.enabled && phase === "typing" && visible ? toConfig(metronome) : null;
    untrack(() => void controller.set(wish));
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
    refreshModel(run, practice);
    void LANGUAGES.load();
    return () => {
      setTypingField(null);
      void controller.set(null);
    };
  });

  // WebKit's `autocorrect` is missing from Svelte's textarea attribute types.
  const noAutocorrect: Record<string, string> = { autocorrect: "off" };

  const focusAnd = (action: () => void) => () => {
    focusTyping();
    action();
  };
</script>

<svelte:window onkeydown={onKeydown} onclick={onWindowClick} />
<svelte:document onfocusout={refocusIfLost} onvisibilitychange={() => (visible = !document.hidden)} />

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
    {#if phase === "typing" && ((liveStats && view) || anchor)}
      <div class="top">
        {#if anchor}<BeatDot {anchor} />{/if}
        {#if liveStats && view}
          <dl class="live" aria-label="Estadísticas en vivo">
            <div><dt>PPM</dt><dd>{formatWpm(view.stats.netWpm)}</dd></div>
            <div><dt>Pulsaciones/min</dt><dd>{formatWpm(view.stats.kpm)}</dd></div>
            <div><dt>Precisión</dt><dd>{formatPercent(view.stats.accuracy)}</dd></div>
            <div><dt>Tiempo</dt><dd>{formatTime(elapsed)}</dd></div>
          </dl>
        {/if}
      </div>
    {/if}

    {#if phase === "loading"}
      <p class="status">{loadingLabel}</p>
    {:else if phase === "error"}
      <div class="status" role="alert">
        <p>{message}</p>
        <button type="button" onclick={focusAnd(() => retry())}>Reintentar</button>
      </div>
    {:else if phase === "results" && scored}
      <Results {scored} {weak} {tempo} language={textMode === "prose" ? null : LANGUAGES.name(textMode.lang)} onNext={loadNext} onRepeat={restart} />
    {:else if session && view}
      <p class="kind">
        {#if textMode !== "prose"}{LANGUAGES.name(textMode.lang)} ·{/if}
        {source}
        {#if targets.length > 0}
          · Practicando:
          {#each targets as t, i (i)}{#if i > 0}<span class="sep">·</span>{/if}<kbd>{charLabel(t)}</kbd>{/each}
        {/if}
      </p>
      {#if notice}<p class="notice" role="status">{notice}</p>{/if}
      {#if metronomeError && metronome.enabled}<p class="notice" role="status">Metrónomo: {metronomeError}</p>{/if}
      {#if isCode}
        <CodeText chars={session.chars} states={view.states} cursor={view.cursor} {classes} />
      {:else}
        <!-- No whitespace inside: the text uses white-space: pre-wrap. -->
        <p class="text" aria-label="Texto de práctica">{#each session.chars as c, i (i)}<span
              class="ch {view.states[i]}"
              class:space={c === " "}
              class:cursor={i === view.cursor}>{c}</span
            >{/each}</p>
      {/if}
      <p class="hint">
        {#if phase === "scoring"}Calculando resultado…{:else if view.cursor === 0}Empieza a escribir cuando quieras · {#if isCode}Intro = salto de línea · Tab = sangría · {/if}Esc reinicia · F4 metrónomo{:else}Esc reinicia el texto · F4 metrónomo{/if}
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
  .top {
    display: flex;
    align-items: center;
    gap: 1rem;
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
  .notice {
    margin: 0 0 -0.75rem;
    font-size: 0.8rem;
    color: var(--warning);
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
