<script lang="ts">
  import { onMount } from "svelte";
  import { storageStatus } from "./features/practice/api";
  import type { ErrorMode } from "./features/practice/engine";
  import Practice from "./features/practice/Practice.svelte";
  import SettingsBar from "./features/settings/SettingsBar.svelte";
  import {
    loadErrorMode,
    loadFlashes,
    loadHeatmap,
    loadLiveStats,
    loadShowKeyboard,
    loadTargetCount,
    loadTextLength,
    saveErrorMode,
    saveFlashes,
    saveHeatmap,
    saveLiveStats,
    saveShowKeyboard,
    saveTargetCount,
    saveTextLength,
  } from "./features/settings/settings";
  import { applyTheme, loadThemeId, saveThemeId, themeById } from "./features/themes";

  let mode = $state(loadErrorMode());
  let liveStats = $state(loadLiveStats());
  let showKeyboard = $state(loadShowKeyboard());
  let flashes = $state(loadFlashes());
  let themeId = $state(loadThemeId());
  let heatmap = $state(loadHeatmap());
  let targetCount = $state(loadTargetCount());
  let textLength = $state(loadTextLength());
  // Why sessions are not kept on disk; `null` while persistent or unknown.
  let storageError = $state<string | null>(null);

  function setMode(next: ErrorMode) {
    mode = next;
    saveErrorMode(next);
  }

  function setTheme(id: string) {
    themeId = id;
    applyTheme(themeById(id));
    saveThemeId(id);
  }

  function setLiveStats(on: boolean) {
    liveStats = on;
    saveLiveStats(on);
  }

  function setShowKeyboard(on: boolean) {
    showKeyboard = on;
    saveShowKeyboard(on);
  }

  function setFlashes(on: boolean) {
    flashes = on;
    saveFlashes(on);
  }

  function setHeatmap(on: boolean) {
    heatmap = on;
    saveHeatmap(on);
  }

  function setTargetCount(count: number) {
    targetCount = count;
    saveTargetCount(count);
  }

  function setTextLength(length: number) {
    textLength = length;
    saveTextLength(length);
  }

  onMount(() => {
    storageStatus()
      .then((s) => (storageError = s.persistent ? null : (s.error ?? "")))
      .catch((err) => console.warn("storage_status failed", err));
  });

  // F2 shows or hides the keyboard from anywhere (S11); it never reaches the practice text.
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "F2" || e.repeat) return;
    e.preventDefault();
    setShowKeyboard(!showKeyboard);
  }
</script>

<svelte:window onkeydown={onKeydown} />

<header>
  <div class="brand">
    <h1>PlexusTyping</h1>
    <span>Del cerebro a los dedos.</span>
    {#if storageError !== null}
      <strong class="temporary" title={storageError}>Sin guardar: modo temporal</strong>
    {/if}
  </div>
  <SettingsBar
    {mode}
    {themeId}
    {liveStats}
    {showKeyboard}
    {flashes}
    {heatmap}
    {targetCount}
    {textLength}
    onMode={setMode}
    onTheme={setTheme}
    onLiveStats={setLiveStats}
    onShowKeyboard={setShowKeyboard}
    onFlashes={setFlashes}
    onHeatmap={setHeatmap}
    onTargetCount={setTargetCount}
    onTextLength={setTextLength}
  />
</header>

<Practice {mode} {liveStats} {showKeyboard} {flashes} {heatmap} {targetCount} {textLength} />

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
