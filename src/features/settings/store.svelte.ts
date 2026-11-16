import type { ErrorMode } from "../practice/engine";
import { applyTheme, loadThemeId, saveThemeId, themeById } from "../themes";
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
} from "./settings";

/** The live, persisted settings shared by the app and every settings section. */
export class SettingsStore {
  mode = $state(loadErrorMode());
  liveStats = $state(loadLiveStats());
  showKeyboard = $state(loadShowKeyboard());
  flashes = $state(loadFlashes());
  themeId = $state(loadThemeId());
  heatmap = $state(loadHeatmap());
  targetCount = $state(loadTargetCount());
  textLength = $state(loadTextLength());

  /** Changing the mode restarts the current text (the practice screen watches it). */
  setMode(next: ErrorMode) {
    this.mode = next;
    saveErrorMode(next);
  }

  /** Switches the theme live and remembers it. */
  setTheme(id: string) {
    this.themeId = id;
    applyTheme(themeById(id));
    saveThemeId(id);
  }

  setLiveStats(on: boolean) {
    this.liveStats = on;
    saveLiveStats(on);
  }

  setShowKeyboard(on: boolean) {
    this.showKeyboard = on;
    saveShowKeyboard(on);
  }

  setFlashes(on: boolean) {
    this.flashes = on;
    saveFlashes(on);
  }

  setHeatmap(on: boolean) {
    this.heatmap = on;
    saveHeatmap(on);
  }

  /** Applies from the next text. */
  setTargetCount(count: number) {
    this.targetCount = count;
    saveTargetCount(count);
  }

  /** Applies from the next text. */
  setTextLength(length: number) {
    this.textLength = length;
    saveTextLength(length);
  }
}
