<script lang="ts">
  import { onMount } from "svelte";
  import { storageStatus } from "./features/practice/api";
  import Practice from "./features/practice/Practice.svelte";
  import { practiceMode } from "./features/code/api";
  import ModeSwitch from "./features/code/ModeSwitch.svelte";
  import HistoryModal from "./features/history/HistoryModal.svelte";
  import { isHistoryShortcut } from "./features/history/history";
  import { isMetronomeShortcut } from "./features/metronome/beat";
  import { isSettingsShortcut } from "./features/settings/sections";
  import SettingsModal from "./features/settings/SettingsModal.svelte";
  import { SettingsStore } from "./features/settings/store.svelte";
  import logo from "../assets/logo.svg";

  const settings = new SettingsStore();
  const practice = $derived(practiceMode(settings.practiceKind, settings.codeLanguage));
  let modal: SettingsModal | undefined = $state();
  let history: HistoryModal | undefined = $state();
  // Why sessions are not kept on disk; `null` while persistent or unknown.
  let storageError = $state<string | null>(null);

  onMount(() => {
    storageStatus()
      .then((s) => (storageError = s.persistent ? null : (s.error ?? "")))
      .catch((err) => console.warn("storage_status failed", err));
  });

  // F2 shows or hides the keyboard from anywhere (S11), F4 turns the metronome on or off (S7) and
  // Ctrl+, opens the settings (S8) and Ctrl+H the history; none reaches the practice text.
  function onKeydown(e: KeyboardEvent) {
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
    <img class="logo" src={logo} alt="" width="28" height="28" />
    <h1>PlexusTyping</h1>
    <span>Del cerebro a los dedos.</span>
    {#if storageError !== null}
      <strong class="temporary" title={storageError}>Sin guardar: modo temporal</strong>
    {/if}
  </div>
  <ModeSwitch {settings} />
  <button type="button" class="gear" aria-label="Historial" title="Historial" onclick={() => history?.open()}>
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <circle cx="12" cy="12" r="8.5" stroke-width="2" />
      <path d="M12 7v5l3.5 2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </button>
  <button type="button" class="gear" aria-label="Ajustes" title="Ajustes" onclick={() => modal?.open()}>
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor">
      <circle cx="12" cy="12" r="8.4" stroke-width="3" stroke-dasharray="3.3 3.3" />
      <circle cx="12" cy="12" r="5.6" stroke-width="2.6" />
    </svg>
  </button>
</header>

<Practice
  mode={settings.mode}
  liveStats={settings.liveStats}
  showKeyboard={settings.showKeyboard}
  flashes={settings.flashes}
  heatmap={settings.heatmap}
  targetCount={settings.targetCount}
  textLength={settings.textLength}
  aiRevision={settings.aiRevision}
  metronome={settings.metronome}
  onTempo={(bpm) => settings.setMetronome({ bpm })}
  {practice}
  indent={settings.codeIndent}
  layoutId={settings.layout}
  languageId={settings.language}
  codeLines={settings.codeLines}
/>

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
    padding: clamp(0.5rem, 2vh, 1rem) clamp(1rem, 3vw, 2rem);
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
    margin-right: -0.25rem;
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
  .gear {
    flex: none;
    display: flex;
    padding: 0.35rem;
    color: var(--muted);
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
