import type { PracticeKind } from "../code/api";
import type { ErrorMode, IndentMode } from "../practice/engine";
import { applyTheme, loadThemeId, saveThemeId, themeById } from "../themes";
import type { MetronomeSettings } from "../metronome/beat";
import {
  MAX_CODE_LINES,
  MIN_CODE_LINES,
  checkMetronome,
  loadCodeIndent,
  loadCodeLanguage,
  loadCodeLines,
  loadPracticeKind,
  saveCodeIndent,
  saveCodeLanguage,
  saveCodeLines,
  savePracticeKind,
  loadErrorMode,
  loadFlashes,
  loadHeatmap,
  loadLiveStats,
  loadMetronome,
  loadShowKeyboard,
  loadTargetCount,
  loadTextLength,
  saveErrorMode,
  saveFlashes,
  saveHeatmap,
  saveLiveStats,
  saveMetronome,
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
  metronome = $state.raw(loadMetronome());
  /** Prose or code (S1), the code language, its indentation (S4) and snippet lines (S2). */
  practiceKind = $state<PracticeKind>(loadPracticeKind());
  codeLanguage = $state(loadCodeLanguage());
  codeIndent = $state<IndentMode>(loadCodeIndent());
  codeLines = $state(loadCodeLines());
  /** Bumped whenever AI settings or keys change: a prefetched next text is then stale. */
  aiRevision = $state(0);

  bumpAi() {
    this.aiRevision++;
  }

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

  /** Changes apply live; an out-of-range field keeps its current value. */
  setMetronome(patch: Partial<MetronomeSettings>) {
    this.metronome = checkMetronome({ ...this.metronome, ...patch }, this.metronome);
    saveMetronome(this.metronome);
  }

  /** Switching prose/code or the language loads a new text (the practice screen watches them). */
  setPracticeKind(kind: PracticeKind) {
    this.practiceKind = kind;
    savePracticeKind(kind);
  }

  setCodeLanguage(id: string) {
    this.codeLanguage = id;
    saveCodeLanguage(id);
  }

  /** Restarts a code text being typed. */
  setCodeIndent(indent: IndentMode) {
    this.codeIndent = indent;
    saveCodeIndent(indent);
  }

  /** Applies from the next text; an out-of-range value keeps the current one. */
  setCodeLines(lines: number) {
    if (!Number.isInteger(lines) || lines < MIN_CODE_LINES || lines > MAX_CODE_LINES) return;
    this.codeLines = lines;
    saveCodeLines(lines);
  }
}
