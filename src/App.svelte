<script lang="ts">
  import type { ErrorMode } from "./features/practice/engine";
  import Practice from "./features/practice/Practice.svelte";
  import SettingsBar from "./features/settings/SettingsBar.svelte";
  import {
    loadErrorMode,
    loadFlashes,
    loadLiveStats,
    loadShowKeyboard,
    saveErrorMode,
    saveFlashes,
    saveLiveStats,
    saveShowKeyboard,
  } from "./features/settings/settings";
  import { applyTheme, loadThemeId, saveThemeId, themeById } from "./features/themes";

  let mode = $state(loadErrorMode());
  let liveStats = $state(loadLiveStats());
  let showKeyboard = $state(loadShowKeyboard());
  let flashes = $state(loadFlashes());
  let themeId = $state(loadThemeId());

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
  </div>
  <SettingsBar
    {mode}
    {themeId}
    {liveStats}
    {showKeyboard}
    {flashes}
    onMode={setMode}
    onTheme={setTheme}
    onLiveStats={setLiveStats}
    onShowKeyboard={setShowKeyboard}
    onFlashes={setFlashes}
  />
</header>

<Practice {mode} {liveStats} {showKeyboard} {flashes} />

<style>
  header {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem 2rem;
    padding: clamp(0.5rem, 2vh, 1rem) 2rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
  }
  h1 {
    margin: 0;
    font-size: 1.1rem;
  }
  .brand span {
    color: var(--muted);
    font-size: 0.9rem;
  }
</style>
