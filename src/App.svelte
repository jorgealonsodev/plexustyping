<script lang="ts">
  import { onMount } from "svelte";
  import { storageStatus } from "./features/practice/api";
  import Practice from "./features/practice/Practice.svelte";
  import { practiceMode } from "./features/code/api";
  import ModeSwitch from "./features/code/ModeSwitch.svelte";
  import HistoryModal from "./features/history/HistoryModal.svelte";
  import { isHistoryShortcut } from "./features/history/history";
  import { isMetronomeShortcut } from "./features/metronome/beat";
  import { isNewTextShortcut } from "./features/practice/replies";
  import { isSettingsShortcut } from "./features/settings/sections";
  import SettingsModal from "./features/settings/SettingsModal.svelte";
  import { SettingsStore } from "./features/settings/store.svelte";
  import { focusTyping, isHeaderShortcut, isPointerClick, isTypingFocused } from "./features/practice/focus";
  import { ghostToggleTitle } from "./features/practice/ghost";
  import { aiSettingsGet, aiSettingsSet } from "./features/settings/aiApi";
  import { aiToggleTitle, toggleEnabled } from "./features/settings/ai";
  import { goalHeaderText } from "./features/goals/goals";

  const settings = new SettingsStore();
  const practice = $derived(practiceMode(settings.practiceKind, settings.codeLanguage));
  /** Today's daily goal progress next to the profile name (S5); `null` without a goal. */
  const goalText = $derived(goalHeaderText(settings.goal?.status ?? null));
  let modal: SettingsModal | undefined = $state();
  let history: HistoryModal | undefined = $state();
  let practiceScreen: Practice | undefined = $state();
  /** The first header control: F6 moves the focus here from the typing field. */
  let logo: HTMLButtonElement | undefined = $state();
  /** A mouse click on a header toggle gives the keyboard back to the text; a keyboard activation keeps the focus on it. */
  const backToText = (e: MouseEvent) => isPointerClick(e) && focusTyping();
  /** "Cargando texto…" or "Generando texto…" while a text loads; `null` otherwise. */
  let loadingText = $state<string | null>(null);
  // Why sessions are not kept on disk; `null` while persistent or unknown.
  let storageError = $state<string | null>(null);

  let aiSaving = $state(false);

  /** Flips the stored AI setting (the same as Ajustes → IA) and drops the prefetched text. */
  async function toggleAi() {
    aiSaving = true;
    try {
      const saved = await aiSettingsSet(toggleEnabled(await aiSettingsGet()));
      settings.aiEnabled = saved.enabled;
      settings.bumpAi();
    } catch (err) {
      console.warn("ai_settings_set failed", err);
    } finally {
      aiSaving = false;
    }
  }

  onMount(() => {
    void settings.loadProfiles();
    aiSettingsGet()
      .then((s) => (settings.aiEnabled ??= s.enabled))
      .catch((err) => console.warn("ai_settings_get failed", err));
    storageStatus()
      .then((s) => (storageError = s.persistent ? null : (s.error ?? "")))
      .catch((err) => console.warn("storage_status failed", err));
  });

  // F2 shows or hides the keyboard from anywhere (S11), F4 turns the metronome on or off (S7) and
  // Ctrl+, opens the settings (S8) and Ctrl+H the history; none reaches the practice text. F6 moves
  // the focus from the typing field to the header (accessibility S2). A dialog opened by a shortcut
  // gives the focus back to the typing field; one opened by a header button, to that button.
  function onKeydown(e: KeyboardEvent) {
    if (isHeaderShortcut(e) && isTypingFocused()) {
      e.preventDefault();
      logo?.focus();
      return;
    }
    if (isSettingsShortcut(e)) {
      e.preventDefault();
      modal?.open();
      return;
    }
    if (isHistoryShortcut(e)) {
      e.preventDefault();
      history?.open();
      return;
    }
    if (isNewTextShortcut(e)) {
      e.preventDefault();
      practiceScreen?.newText();
      return;
    }
    if (isMetronomeShortcut(e)) {
      e.preventDefault();
      settings.setMetronome({ enabled: !settings.metronome.enabled });
      return;
    }
    if (e.key !== "F2" || e.repeat) return;
    e.preventDefault();
    settings.setShowKeyboard(!settings.showKeyboard);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<header>
  <div class="brand">
    <!-- The logo key toggles the on-screen keyboard (like F2); its bar lights while the keyboard shows. -->
    <button
      bind:this={logo}
      type="button"
      class="logo"
      aria-pressed={settings.showKeyboard}
      aria-label="Teclado en pantalla"
      title={settings.showKeyboard ? "Ocultar teclado (F2)" : "Mostrar teclado (F2)"}
      onclick={(e) => {
        settings.setShowKeyboard(!settings.showKeyboard);
        backToText(e);
      }}
    >
      <svg viewBox="16 16 224 224" width="28" height="28" aria-hidden="true">
        <defs>
          <linearGradient id="hdr-top" x1="0" y1="0" x2="0" y2="1"
            ><stop offset="0" stop-color="#b4cdff" /><stop offset="1" stop-color="#7aa2f7" /></linearGradient
          >
          <linearGradient id="hdr-side" x1="0" y1="0" x2="0" y2="1"
            ><stop offset="0" stop-color="#4f6fa8" /><stop offset="1" stop-color="#2a3b66" /></linearGradient
          >
        </defs>
        <rect x="16" y="30" width="224" height="210" rx="50" fill="url(#hdr-side)" />
        <rect x="34" y="16" width="188" height="184" rx="38" fill="url(#hdr-top)" />
        <text
          x="128"
          y="138"
          text-anchor="middle"
          font-family="Inter, 'Segoe UI', Helvetica, Arial, sans-serif"
          font-size="104"
          font-weight="800"
          fill="#1e1e2e">P</text
        >
        <rect class="led" class:on={settings.showKeyboard} x="100" y="160" width="56" height="10" rx="5" stroke="#1e1e2e" stroke-width="3" />
      </svg>
    </button>
    <h1>PlexusTyping</h1>
    <span>Del cerebro a los dedos.</span>
    {#if storageError !== null}
      <strong class="temporary" title={storageError}>Sin guardar: modo temporal</strong>
    {/if}
  </div>
  {#if settings.activeProfile}
    <div class="who">
      <!-- The active profile (S4): opens Ajustes → Perfiles to switch or manage profiles. -->
      <button
        type="button"
        class="profile"
        title="Perfil activo: {settings.activeProfile.name}. Cambiar en Ajustes → Perfiles"
        onclick={(e) => modal?.open("profiles", e.currentTarget)}>{settings.activeProfile.name}</button
      >
      <!-- Today's daily goal progress and streak (S5). -->
      {#if goalText}<span class="goal" title="Meta diaria del perfil activo">{goalText}</span>{/if}
    </div>
  {/if}
  <ModeSwitch {settings} />
  <button
    type="button"
    class="gear ai-toggle"
    class:active={settings.aiEnabled === true}
    aria-pressed={settings.aiEnabled === true}
    title={aiToggleTitle(settings.aiEnabled === true)}
    disabled={settings.aiEnabled === null || aiSaving}
    onclick={(e) => {
      void toggleAi();
      backToText(e);
    }}><span class="ai-label">IA</span></button
  >
  <button
    type="button"
    class="gear ai-toggle"
    class:active={settings.ghost}
    aria-pressed={settings.ghost}
    aria-label="Sesión fantasma"
    title={ghostToggleTitle(settings.ghost)}
    onclick={(e) => {
      settings.setGhost(!settings.ghost);
      backToText(e);
    }}
  >
    <!-- A ghost: round head, wavy hem, two eyes. -->
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <path
        d="M5 20.5V11a7 7 0 0 1 14 0v9.5l-2.33-1.75-2.34 1.75L12 18.75l-2.33 1.75-2.34-1.75z"
        stroke-width="2"
        stroke-linejoin="round"
      />
      <circle cx="9.5" cy="10.5" r="1.2" fill="currentColor" stroke="none" />
      <circle cx="14.5" cy="10.5" r="1.2" fill="currentColor" stroke="none" />
    </svg>
  </button>
  <button
    type="button"
    class="gear"
    class:spinning={loadingText !== null}
    aria-label={loadingText ?? "Nuevo texto"}
    title={loadingText ?? "Nuevo texto (F5)"}
    disabled={loadingText !== null}
    onclick={() => practiceScreen?.newText()}
  >
    <!-- Two arrows chasing each other round a circle. -->
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <path d="M19.5 12a7.5 7.5 0 0 1-12.8 5.3" stroke-width="2" stroke-linecap="round" />
      <path d="M4.5 12a7.5 7.5 0 0 1 12.8-5.3" stroke-width="2" stroke-linecap="round" />
      <path d="M17.8 2.8v4h-4" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      <path d="M6.2 21.2v-4h4" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
  <button type="button" class="gear" aria-label="Historial" title="Historial" onclick={(e) => history?.open(e.currentTarget)}>
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <circle cx="12" cy="12" r="8.5" stroke-width="2" />
      <path d="M12 7v5l3.5 2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
  <button type="button" class="gear" aria-label="Ajustes" title="Ajustes" onclick={(e) => modal?.open(undefined, e.currentTarget)}>
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <circle cx="12" cy="12" r="8.4" stroke-width="3" stroke-dasharray="3.3 3.3" />
      <circle cx="12" cy="12" r="5.6" stroke-width="2.6" />
    </svg>
  </button>
</header>

<!-- After the active profile is known, so the first text already uses its layout and language. -->
{#if settings.profilesReady}
  <Practice
    bind:this={practiceScreen}
    bind:loadingText
    mode={settings.mode}
    liveStats={settings.liveStats}
    ghost={settings.ghost}
    showKeyboard={settings.showKeyboard}
    flashes={settings.flashes}
    colorBlind={settings.colorBlind}
    dimPending={settings.dimPending}
    heatmap={settings.heatmap}
    targetCount={settings.targetCount}
    textLength={settings.textLength}
    aiRevision={settings.aiRevision}
    metronome={settings.metronome}
    onMetronomeChange={(change) => settings.setMetronome(change)}
    onTempo={(bpm) => settings.setMetronome({ bpm })}
    {practice}
    indent={settings.codeIndent}
    layoutId={settings.layout}
    languageId={settings.language}
    codeLines={settings.codeLines}
    profileRevision={settings.profileRevision}
    unlock={settings.unlock}
    onSaved={() => settings.sessionSaved()}
    screenReader={settings.screenReader}
    fontScale={settings.typography.fontScale}
    letterSpacing={settings.typography.letterSpacing}
    lineHeight={settings.typography.lineHeight}
    fontId={settings.practiceFont}
  />
{/if}

<SettingsModal bind:this={modal} {settings} />
<HistoryModal bind:this={history} />

<style>
  header {
    flex: none;
    display: flex;
    flex-wrap: nowrap;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    /* Fixed height: the same bar in prose and code, whatever the engine draws the selects at. */
    box-sizing: border-box;
    height: clamp(3.4rem, calc(2.4rem + 4vh), 4.4rem);
    padding: 0 clamp(1rem, 3vw, 2rem);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
  }
  .logo {
    align-self: center;
    display: flex;
    margin-right: -0.25rem;
    padding: 0;
    background: none;
    border: 0;
    border-radius: 0.3rem;
  }
  /* Off: an unlit bar; on: a green light, like Caps Lock. */
  .led {
    fill: #5b6078;
    /* Off under reduced motion, with every other transition (app.css, a11y S6). */
    transition: fill 150ms ease-out;
  }
  /* A vivid LED green with a glow, the same in every theme. */
  .led.on {
    fill: #2bff6a;
    filter: drop-shadow(0 0 6px #2bff6a);
  }
  h1 {
    margin: 0;
    font-size: 1.1rem;
  }
  .temporary {
    white-space: nowrap;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--warning);
  }
  /* The name keeps its room (up to 12rem); the goal text gives way first. */
  .profile {
    flex: none;
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0.3rem 0.7rem;
    font-size: 0.9rem;
    color: var(--muted);
  }
  .profile:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .who {
    flex: 0 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .goal {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.85rem;
    color: var(--muted);
  }
  .gear {
    flex: none;
    display: flex;
    padding: 0.35rem;
    color: var(--muted);
  }
  /* Lit like the active Prosa/Código button while the AI generates new texts. */
  /* The same box as the icon buttons: "IA" fills the 20 px an icon takes. */
  .ai-label {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    font-size: 0.8rem;
    font-weight: 700;
    line-height: 1;
  }
  .ai-toggle:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .ai-toggle.active,
  .ai-toggle.active:hover {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }
  .ai-toggle.active:focus-visible {
    outline-color: var(--on-accent);
    outline-offset: -3px;
  }
  /* While a new text is generated the arrows turn. */
  .spinning svg {
    animation: spin 0.9s linear infinite;
  }
  .gear:disabled {
    cursor: progress;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinning svg {
      animation: none;
    }
  }
  .gear:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .brand span {
    white-space: nowrap;
    color: var(--muted);
    font-size: 0.9rem;
  }
  /* Narrow windows keep the header to one row: drop the tagline, wrap the warning onto two lines. */
  @media (max-width: 1100px) {
    .brand span {
      display: none;
    }
  }
  @media (max-width: 900px) {
    .brand {
      align-items: center;
    }
    .temporary {
      max-width: 7.5rem;
      white-space: normal;
      line-height: 1.15;
    }
  }
</style>
