<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { getLayout, keySequences } from "../keyboard/api";
  import { type CharScore, heatScores } from "../keyboard/heat";
  import Keyboard from "../keyboard/Keyboard.svelte";
  import { type Flash, highlightsFor, keyForChar } from "../keyboard/keys";
  import type { KeyPress, Layout } from "../keyboard/types";
  import { charLabel, formatPercent, formatTime, formatWpm } from "./format";
  import { freshen, keystrokeMessage, liveText, newTextMessage, resultMessage } from "./announce";
  import { DEFAULT_TYPOGRAPHY, typographyStyle } from "../settings/settings";
  import { DEFAULT_FONT_ID, fontById, fontStyle } from "../settings/fonts";
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
  import { type PracticeMode, isCodeMode, practiceKey } from "../code/api";
  import CodeText from "../code/CodeText.svelte";
  import { LANGUAGES } from "../code/languages.svelte";
  import { type TokenClass, codeKeyEvent, tokenClasses } from "../code/syntax";
  import { escapeRestarts, focusTyping, isPointerClick, setTypingField, tabTypes, typingHint } from "./focus";
  import { Prefetcher, generationKey, isProfileSwitch, provisionalFailedChars, sourceLabel } from "./prefetch";
  import { type Phase, indentChangeRestarts, shouldApply } from "./replies";
  import Results from "./Results.svelte";
  import { type Ghost, GhostRace, formatDelta, ghostForText, ghostPosition } from "./ghost";
  import { metronomeApi } from "../metronome/api";
  import {
    type BeatAnchor,
    type MetronomeSettings,
    type TempoChange,
    beatOffsetMs,
    tempoChangeFor,
    toConfig,
  } from "../metronome/beat";
  import MetronomeQuick from "../metronome/MetronomeQuick.svelte";
  import { MetronomeController } from "../metronome/controller";
  import type { UnlockStatus } from "../unlock/api";
  import { lockedKeys, progressLine, unlockApplies } from "../unlock/unlock";

  let {
    mode,
    liveStats,
    ghost: ghostEnabled = true,
    showKeyboard,
    flashes,
    colorBlind = false,
    dimPending = true,
    heatmap,
    targetCount,
    textLength,
    aiRevision,
    metronome,
    onMetronomeChange,
    onTempo,
    practice,
    indent,
    codeLines,
    layoutId,
    languageId,
    profileRevision = 0,
    unlock = null,
    onSaved,
    loadingText = $bindable(null),
    screenReader = false,
    fontScale = DEFAULT_TYPOGRAPHY.fontScale,
    letterSpacing = DEFAULT_TYPOGRAPHY.letterSpacing,
    lineHeight = DEFAULT_TYPOGRAPHY.lineHeight,
    fontId = DEFAULT_FONT_ID,
    cursorStyle = "highlight",
    cursorBlink = false,
  }: {
    mode: ErrorMode;
    liveStats: boolean;
    /** Race the ghost of the best earlier session with the same text (ghost S3). */
    ghost?: boolean;
    showKeyboard: boolean;
    flashes: boolean;
    /** Colour-blind keyboard flashes (a11y S5): Okabe–Ito blue/orange and an underlined wrong key. */
    colorBlind?: boolean;
    /** Pending chars, prose and code, in the dimmed colours of about 2.5:1 (a11y S7). */
    dimPending?: boolean;
    heatmap: boolean;
    targetCount: number;
    textLength: number;
    /** Changes when AI settings or keys change. */
    aiRevision: number;
    metronome: MetronomeSettings;
    /** A change made in the metronome's quick panel (next to the beat dot). */
    onMetronomeChange?: (change: Partial<MetronomeSettings>) => void;
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
    /** Changes when another profile becomes active: a new text, weaknesses and heatmap are loaded. */
    profileRevision?: number;
    /** Progressive unlock of the profile in this language and layout (unlock S1, S6); `null` until read. */
    unlock?: UnlockStatus | null;
    /** A session was saved; resolves `true` when it met the daily goal (its result shows the notice once). */
    onSaved?: () => Promise<boolean>;
    /** Out: the loading label ("Cargando texto…", "Generando texto…") while a text loads, else `null`. */
    loadingText?: string | null;
    /** Screen reader mode (a11y S3): a polite live region announces the next char, errors, new texts and results. */
    screenReader?: boolean;
    /** Practice text size in % (80–200), letter spacing in em and line height, prose and code (a11y S4). */
    fontScale?: number;
    letterSpacing?: number;
    lineHeight?: number;
    /** Practice font id (practice-font S1); an unknown id is the default. */
    fontId?: string;
    /** Cursor look id and blink (cursor-style S1, S2), read by the `data-cursor` rules below and in CodeText. */
    cursorStyle?: string;
    cursorBlink?: boolean;
  } = $props();

  // The practice text region; the typing field is described by it (a11y S3).
  const uid = $props.id();
  const textId = `${uid}-text`;
  const textStyle = $derived(
    `${typographyStyle({ fontScale, letterSpacing, lineHeight })}; ${fontStyle(fontById(fontId))}`,
  );
  // What the live region says; nothing is announced while the setting is off.
  let announcement = $state("");
  function announce(message: string) {
    if (!screenReader || !message) return;
    announcement = freshen(message, announcement);
  }

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
  const isCode = $derived(isCodeMode(textMode));
  const practiceId = $derived(practiceKey(practice));
  // Profile, mode, layout and language of the latest requested text: a different one loads a new text.
  // Code texts do not depend on the prose language (S1); prose and words texts do.
  const requestKey = (p: PracticeMode, layoutOf: string, languageOf: string) =>
    `${profileRevision}|${practiceKey(p)}|${layoutOf}|${isCodeMode(p) ? "" : languageOf}`;
  const wantedKey = $derived(requestKey(practice, layoutId, languageId));
  let requestedKey = untrack(() => requestKey(practice, layoutId, languageId));
  // Layout and language the current text was generated for; the scored session records them (S5).
  let textLayout = untrack(() => layoutId);
  let textLanguage = untrack(() => languageId);
  // The next text, requested in the background while this one is typed (S6).
  const prefetch = new Prefetcher<PracticeText>();
  // The active keys shape prose texts in the unlock mode (S5): a prefetch made before an unlock is stale.
  const unlockKey = $derived(unlockApplies(unlock, practice) ? `unlock:${unlock.active.join("")}` : "");
  const genKey = $derived(
    `${generationKey(targetCount, textLength, aiRevision, practiceId, codeLines, layoutId, isCodeMode(practice) ? "es" : languageId)}|${unlockKey}`,
  );
  let weak = $state.raw<Weakness[]>([]);
  let charScores = $state.raw<CharScore[]>([]);
  let session = $state.raw<TypingSession | null>(null);
  // Ghost session (S1–S5): the record of the current text, the user's side of the race and the
  // record a finished text raced, for its results.
  let ghost = $state.raw<Ghost | null>(null);
  let race: GhostRace | null = null;
  let ghostDelta = $state<number | null>(null);
  let raced = $state.raw<Ghost | null>(null);
  let ghostRun = 0;
  const racing = $derived(ghostEnabled && ghost !== null);
  // Bumped after each applied event: the session object itself is not reactive.
  let tick = $state(0);
  let now = $state(0);
  let scored = $state.raw<ScoredSession | null>(null);
  /** What the adaptive metronome decided after the scored text; `null` when it did not apply. */
  let tempo = $state.raw<TempoChange | null>(null);
  /** The scored session met the daily goal (S5). */
  let goalMet = $state(false);
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
  // The ghost cursor: waits at the start until the first keystroke, then follows the record's pace;
  // at the end it stays after the last char (S4).
  const ghostAt = $derived.by(() => {
    void tick;
    if (!racing || !ghost || !session || phase !== "typing") return null;
    return ghostPosition(ghost.times, session.firstT === null ? null : Math.max(now - session.firstT, 0));
  });
  const ghostIndex = $derived(ghostAt === null || !session ? -1 : Math.min(ghostAt, session.chars.length - 1));
  const ghostEnd = $derived(ghostAt !== null && !!session && ghostAt === session.chars.length);
  const showDelta = $derived(racing && ghostDelta !== null);
  const heat = $derived(heatmap && layout ? heatScores(layout, charScores) : null);
  // Progressive unlock (S6): prose texts only; the keys not reached yet are dimmed.
  const unlockShown = $derived(unlockApplies(unlock, textMode));
  const locked = $derived(unlockShown ? lockedKeys(layout, unlock) : null);
  const unlockLine = $derived(unlockShown ? progressLine(unlock) : null);
  const highlights = $derived(phase === "typing" && view && sequences ? highlightsFor(sequences[view.cursor]) : []);

  function restart() {
    if (!text) return;
    run++;
    session = new TypingSession(text, mode, isCodeMode(textMode) ? indent : "manual");
    track = { offsets: [], bpms: new Set(), switched: false };
    scored = null;
    goalMet = false;
    phase = "typing";
    tick++;
    announce(newTextMessage(session.chars));
    fetchGhost(text, textMode);
  }

  // A record set in this attempt is read by the next restart, so it races the next attempt (S1).
  function fetchGhost(of: string, practiceOf: PracticeMode) {
    const id = ++ghostRun;
    ghost = null;
    race = null;
    ghostDelta = null;
    raced = null;
    ghostForText(of, practiceOf)
      .then((next) => {
        if (id !== ghostRun) return;
        ghost = next;
        race = next ? new GhostRace(next.times) : null;
      })
      .catch((err) => console.warn("ghost_for_text failed; no ghost", err));
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
    const against = ghostEnabled ? ghost : null;
    try {
      // Wall-clock start: `performance.now()` stamps are relative to the page's time origin.
      const startedAtMs = Math.round(performance.timeOrigin + (done.firstT ?? performance.now()));
      const bpm = measured.bpms.size === 1 ? [...measured.bpms][0] : null;
      const typed = { mode: textMode, indent: done.indent };
      const result = await scoreSession(isCodeMode(textMode) ? null : textLanguage, textLayout, text, done.mode, done.events, startedAtMs, measured.offsets, bpm, typed);
      if (id !== run) return;
      scored = result;
      raced = against;
      tempo = metronome.adaptive ? tempoChangeFor(bpm, result.result.stats.accuracy, result.offsetsMs, measured.switched) : null;
      if (tempo && tempo.to !== tempo.from) onTempo(tempo.to);
      weak = [];
      goalMet = false;
      phase = "results";
      announce(resultMessage(result.result.stats));
      refreshModel(id);
      if (result.saved && onSaved)
        onSaved()
          .then((met) => id === run && (goalMet = met))
          .catch((err) => console.warn("goal refresh failed", err));
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
    const before = session.cursor;
    const first = session.firstT === null;
    // Chars and Tabs are measured keystrokes (records); backspaces are not.
    const beat = event.kind === "backspace" ? null : anchor;
    // Epoch ms, like beat 0: `performance.now()` stamps are relative to the page's time origin.
    track.offsets.push(beat && beatOffsetMs(performance.timeOrigin + event.tMs, beat.epochMs, beat.intervalMs));
    if (beat) track.bpms.add(beat.bpm);
    if (!session.apply(event)) return;
    now = event.tMs;
    tick++;
    const record = event.kind === "backspace" ? undefined : session.lastRecord;
    const wrong = record && !record.correct ? record : null;
    announce(keystrokeMessage(wrong, session.chars[session.cursor], session.cursor !== before));
    if (race && session.firstT !== null) {
      race.passed(session.cursor, event.tMs - session.firstT);
      ghostDelta = race.delta;
    }
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
    // Escape restarts only a text being typed or reviewed, and only from the typing field (never
    // from the metronome toggle or another control); while a text loads or is scored, restarting
    // would silently discard the pending result.
    if (action === "escape" && escapeRestarts(e.target === field, phase)) {
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
    // In code, Tab types indentation; in prose and words it moves focus out as usual (F6 leaves in every mode).
    if (action === "tab" && tabTypes(phase, isCode)) {
      e.preventDefault();
      const event = codeKeyEvent(action, isCode, performance.now());
      if (event) applyEvent(event);
    }
    if (action === "backspace") {
      e.preventDefault();
      applyEvent({ kind: "backspace", tMs: performance.now() });
    }
  }

  // A mouse click outside a settings control, or focus lost to the page, returns to the typing
  // field. A control activated by keyboard (Enter or Space) keeps the focus it received.
  function onWindowClick(e: MouseEvent) {
    if (!isPointerClick(e)) return;
    if (e.target instanceof Element && e.target.closest("select, input, label, .metro-quick")) return;
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

  $effect(() => {
    loadingText = phase === "loading" ? loadingLabel : null;
  });

  // Turning the screen reader mode off drops what it would still say.
  $effect(() => {
    if (!screenReader) announcement = "";
  });

  /** Drops the current text (unscored) and loads a new one; ignored while a text loads or is scored. */
  export function newText() {
    if (phase === "loading" || phase === "scoring") return;
    loadNext();
    focusTyping();
  }

  // Keeps the cursor line, and the next one, in view: the text scrolls when typing reaches its hidden part.
  let proseText = $state<HTMLElement>();
  $effect(() => {
    void view?.cursor;
    proseText?.querySelector(".cursor")?.scrollIntoView({ block: "nearest", inline: "nearest" });
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

  // Switching profile, prose/code, the language or the layout drops the prefetch and loads a new text.
  // Another profile starts as at start-up: a new one gets its diagnostic text.
  $effect(() => {
    const key = wantedKey;
    untrack(() => {
      if (key === requestedKey) return;
      prefetch.clear();
      if (isProfileSwitch(requestedKey, key)) {
        loadFirst();
        refreshModel(run, practice);
        return;
      }
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

  // The ghost cursor moves on its own while a raced text is typed.
  $effect(() => {
    if (!racing || phase !== "typing") return;
    const timer = setInterval(() => (now = performance.now()), 50);
    return () => clearInterval(timer);
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

<!-- Back from another window (e.g. the desktop overview the Super key opens): typing resumes. -->
<svelte:window onkeydown={onKeydown} onclick={onWindowClick} onfocus={refocusIfLost} />
<svelte:document onfocusout={refocusIfLost} onvisibilitychange={() => (visible = !document.hidden)} />

<textarea
  bind:this={field}
  class="capture"
  aria-label="Entrada de mecanografía"
  aria-describedby={textId}
  autocomplete="off"
  {...noAutocorrect}
  autocapitalize="off"
  spellcheck="false"
  oninput={onInput}
  oncompositionend={takeText}
></textarea>

{#if screenReader}
  <!-- Screen reader mode (a11y S3): not rendered while the setting is off, so nothing is announced. -->
  <div class="sr-only" aria-live="polite" aria-atomic="true">{liveText(screenReader, announcement)}</div>
{/if}

<main>
  <section class="practice">
    {#if phase === "typing" && ((liveStats && view) || anchor || showDelta)}
      <div class="top">
        {#if anchor}
          <MetronomeQuick
            {anchor}
            {metronome}
            onChange={(change) => onMetronomeChange?.(change)}
          />
        {/if}
        {#if liveStats && view}
          <dl class="live" aria-label="Estadísticas en vivo">
            <div><dt>PPM</dt><dd>{formatWpm(view.stats.netWpm)}</dd></div>
            <div><dt>Pulsaciones/min</dt><dd>{formatWpm(view.stats.kpm)}</dd></div>
            <div><dt>Precisión</dt><dd>{formatPercent(view.stats.accuracy)}</dd></div>
            <div><dt>Tiempo</dt><dd>{formatTime(elapsed)}</dd></div>
          </dl>
        {/if}
        {#if showDelta && ghostDelta !== null}
          {@const delta = formatDelta(ghostDelta)}
          <p class="ghost-delta" class:behind={delta.startsWith("+")} class:ahead={delta.startsWith("−")} aria-label="Diferencia con tu récord">{delta}</p>
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
      <Results {scored} {weak} {tempo} {goalMet} record={raced?.netWpm ?? null} language={isCodeMode(textMode) ? LANGUAGES.name(textMode.lang) : null} onNext={loadNext} onRepeat={restart} />
    {:else if session && view}
      <p class="kind">
        {#if isCodeMode(textMode)}{LANGUAGES.name(textMode.lang)} ·{/if}
        {source}
        {#if targets.length > 0}
          · Practicando:
          {#each targets as t, i (i)}{#if i > 0}<span class="sep">·</span>{/if}<kbd>{charLabel(t)}</kbd>{/each}
        {/if}
      </p>
      {#if unlockLine}<p class="kind" aria-label="Desbloqueo progresivo">{unlockLine}</p>{/if}
      {#if notice}<p class="notice" role="status">{notice}</p>{/if}
      {#if metronomeError && metronome.enabled}<p class="notice" role="status">Metrónomo: {metronomeError}</p>{/if}
      <!-- A named region (a11y S3) carrying the typography variables (a11y S4) for prose and code. -->
      <div
        id={textId}
        role="region"
        aria-label={isCode ? "Código de práctica" : "Texto de práctica"}
        style={textStyle}
        data-cursor={cursorStyle}
        data-cursor-blink={cursorBlink || undefined}
      >
      {#if isCode}
        <CodeText chars={session.chars} states={view.states} cursor={view.cursor} {classes} ghost={ghostIndex} {ghostEnd} dim={dimPending} />
      {:else}
        <!-- No whitespace inside: the text uses white-space: pre-wrap. -->
        <p class="text" class:dim={dimPending} bind:this={proseText}>{#each session.chars as c, i (i)}<span
              class="ch {view.states[i]}"
              class:space={c === " "}
              class:cursor={i === view.cursor}
              class:ghost={i === ghostIndex}
              class:ghost-end={ghostEnd && i === ghostIndex}>{c}</span
            >{/each}</p>
      {/if}
      </div>
      <p class="hint">
        {#if phase === "scoring"}Calculando resultado…{:else}{typingHint(view.cursor !== 0, isCode)}{/if}
      </p>
    {/if}
  </section>
</main>

{#if showKeyboard}
  <footer>
    <Keyboard {layout} {highlights} {flash} {heat} {locked} {colorBlind} />
  </footer>
{/if}

<style>
  /* Read by screen readers, never shown. */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
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
    /* The chosen practice font (practice-font S1), set by the practice text region. */
    font-family: var(--practice-font, var(--mono));
    font-feature-settings: var(--practice-features, normal);
    /* Also scales with the height, so the text and the keyboard fit at 720×480. */
    font-size: calc(clamp(1rem, min(2.4vw, 4vh), 2rem) * var(--text-scale, 1));
    letter-spacing: var(--text-spacing, 0);
    line-height: var(--text-leading, 1.75);
    white-space: pre-wrap;
    overflow-wrap: break-word;
  }
  .ch {
    border-radius: 0.15em;
  }
  .pending {
    color: var(--text-pending);
  }
  /* "Atenuar texto pendiente" (a11y S7): the same rule reads the dimmed colour; the cursor char keeps its own. */
  .text.dim {
    --text-pending: var(--text-pending-dim);
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
    /* Scrolling into view also shows the following line. */
    scroll-margin-block: calc(var(--text-leading, 1.75) * 1em);
    color: var(--text);
    background: color-mix(in srgb, var(--cursor) 18%, transparent);
    box-shadow: inset 0 -0.12em var(--cursor);
    border-radius: 0.1em;
  }
  .cursor.error {
    background: color-mix(in srgb, var(--text-error) 22%, transparent);
    box-shadow: inset 0 -0.12em var(--text-error);
  }
  /* Cursor styles (cursor-style S1, S2): --cur is the cursor colour, or the error colour on a wrong char.
     Shadows and backgrounds only, so no style changes the char's width. */
  .cursor {
    --cur: var(--cursor);
  }
  .cursor.error {
    --cur: var(--text-error);
  }
  :global([data-cursor="bar"]) .cursor {
    background: transparent;
    box-shadow: inset 0.12em 0 var(--cur);
  }
  :global([data-cursor="block"]) .cursor {
    color: var(--bg);
    background: var(--cur);
    box-shadow: none;
  }
  :global([data-cursor="underline"]) .cursor {
    background: transparent;
    box-shadow: inset 0 -0.15em var(--cur);
  }
  :global([data-cursor="outline"]) .cursor {
    background: transparent;
    box-shadow: inset 0 0 0 0.08em var(--cur);
  }
  /* About once a second; the global prefers-reduced-motion rule stops it (a11y S6). */
  :global([data-cursor-blink]) .cursor {
    animation: cursor-blink 1.06s steps(1) infinite;
  }
  @keyframes cursor-blink {
    50% {
      color: var(--text);
      background: transparent;
      box-shadow: none;
    }
  }
  /* The ghost cursor (S4): a translucent frame; after the last char once the record finished. */
  .ghost {
    outline: 0.08em solid color-mix(in srgb, var(--cursor) 45%, transparent);
    outline-offset: -0.04em;
  }
  .ghost-end {
    outline: none;
  }
  .ghost-end::after {
    content: "";
    border-right: 0.12em solid color-mix(in srgb, var(--cursor) 45%, transparent);
  }
  .ghost-delta {
    margin: 0 0 0 auto;
    font-family: var(--mono);
    font-size: 1.25rem;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .ghost-delta.behind {
    color: var(--danger);
  }
  .ghost-delta.ahead {
    color: var(--accent);
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
