<script lang="ts">
  import type { ErrorMode } from "./features/practice/engine";
  import Practice from "./features/practice/Practice.svelte";
  import SettingsBar from "./features/settings/SettingsBar.svelte";
  import { loadErrorMode, loadLiveStats, saveErrorMode, saveLiveStats } from "./features/settings/settings";
  import { applyTheme, loadThemeId, saveThemeId, themeById } from "./features/themes";

  let mode = $state(loadErrorMode());
  let liveStats = $state(loadLiveStats());
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
</script>

<header>
  <div class="brand">
    <h1>PlexusTyping</h1>
    <span>Del cerebro a los dedos.</span>
  </div>
  <SettingsBar {mode} {themeId} {liveStats} onMode={setMode} onTheme={setTheme} onLiveStats={setLiveStats} />
</header>

<main>
  <Practice {mode} {liveStats} />
</main>

<style>
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 1rem 2rem;
    padding: 1rem 2rem;
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
  main {
    padding: clamp(1.5rem, 6vh, 4rem) 2rem;
  }
</style>
