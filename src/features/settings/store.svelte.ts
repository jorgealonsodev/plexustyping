import type { PracticeKind } from "../code/api";
import type { ErrorMode, IndentMode } from "../practice/engine";
import {
  DEFAULT_THEME_ID,
  TOO_MANY,
  addCustomTheme,
  applyTheme,
  initCustomThemes,
  loadThemeId,
  parseVsCodeTheme,
  saveCustomThemes,
  saveThemeId,
  setCustomThemes,
  themeById,
  type ThemeEntry,
} from "../themes";
import type { MetronomeSettings } from "../metronome/beat";
import { type Profile, type ProfileFields, type ProfilesApi, profilesApi } from "../profiles/api";
import {
  type Appearance,
  createProfile,
  deleteProfile,
  renameProfile,
  saveAppearance,
  startProfiles,
  switchProfile,
} from "../profiles/profiles";
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
  loadLanguage,
  loadLayout,
  saveLanguage,
  saveLayout,
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

/** The live, persisted settings shared by the app and every settings section. Layout, language and
 * theme belong to the active profile (S5); localStorage keeps them only as the first-paint cache. */
export class SettingsStore {
  private readonly profileApi: ProfilesApi;

  constructor(api: ProfilesApi = profilesApi) {
    this.profileApi = api;
  }

  /** Every profile, the active one and whether the start-up load finished (also when it failed). */
  profiles = $state.raw<Profile[]>([]);
  activeProfile = $state.raw<Profile | null>(null);
  profilesReady = $state(false);
  /** Bumped when another profile becomes active: the practice screen loads a new text and model. */
  profileRevision = $state(0);
  /** The last failed profile write (Spanish, from the backend); `null` when none. */
  profileError = $state<string | null>(null);

  mode = $state(loadErrorMode());
  liveStats = $state(loadLiveStats());
  showKeyboard = $state(loadShowKeyboard());
  flashes = $state(loadFlashes());
  /** Themes imported from VS Code; loaded before the theme id so a saved import is known. */
  customThemes = $state.raw(initCustomThemes());
  themeId = $state(loadThemeId());
  heatmap = $state(loadHeatmap());
  /** Drawn keyboard and the chars texts may contain (S1–S4). */
  layout = $state(loadLayout());
  /** Language of prose texts (S1). */
  language = $state(loadLanguage());
  targetCount = $state(loadTargetCount());
  textLength = $state(loadTextLength());
  metronome = $state.raw(loadMetronome());
  /** Prose, words or code (S1), the code language, its indentation (S4) and snippet lines (S2). */
  practiceKind = $state<PracticeKind>(loadPracticeKind());
  codeLanguage = $state(loadCodeLanguage());
  codeIndent = $state<IndentMode>(loadCodeIndent());
  codeLines = $state(loadCodeLines());
  /** Bumped whenever AI settings or keys change: a prefetched next text is then stale. */
  aiRevision = $state(0);
  /** The stored `AiSettings.enabled`, shared by the header "IA" toggle and Ajustes → IA; `null` until loaded. */
  aiEnabled = $state<boolean | null>(null);

  bumpAi() {
    this.aiRevision++;
  }

  /** Changing the mode restarts the current text (the practice screen watches it). */
  setMode(next: ErrorMode) {
    this.mode = next;
    saveErrorMode(next);
  }

  /** Switches the theme live and saves it to the active profile. */
  setTheme(id: string) {
    this.showTheme(id);
    this.saveToProfile({ theme: id });
  }

  private showTheme(id: string) {
    this.themeId = id;
    applyTheme(themeById(id));
    saveThemeId(id);
  }

  /** Imports a VS Code theme file and switches to it; the error (Spanish, for the user) when it cannot. */
  importTheme(text: string, fileName: string): { ok: true; name: string } | { ok: false; error: string } {
    const parsed = parseVsCodeTheme(text, fileName);
    if (!parsed.ok) return parsed;
    const next = addCustomTheme(this.customThemes, parsed.theme);
    if (!next) return { ok: false, error: TOO_MANY };
    this.storeCustomThemes(next.themes);
    this.setTheme(next.theme.id);
    return { ok: true, name: next.theme.name };
  }

  /** Removes an imported theme; the default theme takes over when it was the active one. */
  removeTheme(id: string) {
    this.storeCustomThemes(this.customThemes.filter((t) => t.id !== id));
    if (this.themeId === id) this.setTheme(DEFAULT_THEME_ID);
  }

  private storeCustomThemes(themes: ThemeEntry[]) {
    this.customThemes = themes;
    setCustomThemes(themes);
    saveCustomThemes(themes);
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

  /** Loads a new text for the layout (the practice screen watches it); saved to the active profile. */
  setLayout(id: string) {
    this.layout = id;
    saveLayout(id);
    this.saveToProfile({ layout: id });
  }

  /** Loads a new prose or words text in the language (the practice screen watches it); saved to the active profile. */
  setLanguage(id: string) {
    this.language = id;
    saveLanguage(id);
    this.saveToProfile({ language: id });
  }

  private appearance(): Appearance {
    return { layout: this.layout, language: this.language, theme: this.themeId };
  }

  /** Shows a profile's layout, language and theme without writing them back to it. */
  private applyAppearance(a: Appearance) {
    if (a.layout !== this.layout) {
      this.layout = a.layout;
      saveLayout(a.layout);
    }
    if (a.language !== this.language) {
      this.language = a.language;
      saveLanguage(a.language);
    }
    if (a.theme !== this.themeId) this.showTheme(a.theme);
  }

  private setActive(profile: Profile) {
    this.activeProfile = profile;
    this.profiles = this.profiles.map((p) => (p.id === profile.id ? profile : p));
  }

  private saveToProfile(fields: ProfileFields) {
    const active = this.activeProfile;
    if (!active) return;
    saveAppearance(this.profileApi, active.id, fields)
      .then((saved) => {
        if (this.activeProfile?.id === saved.id) this.setActive(saved);
      })
      .catch((err) => (this.profileError = String(err)));
  }

  /** Start-up: the active profile (after the one-time carry-over, S6) and its layout, language and theme. */
  async loadProfiles(storage: Storage = localStorage) {
    try {
      const started = await startProfiles(this.profileApi, storage, this.appearance());
      this.profiles = started.profiles;
      this.activeProfile = started.active;
      this.applyAppearance(started.appearance);
    } catch (err) {
      console.warn("profiles failed", err);
      this.profileError = String(err);
    } finally {
      this.profilesReady = true;
    }
  }

  private activate(profile: Profile, appearance: Appearance) {
    this.setActive(profile);
    this.applyAppearance(appearance);
    this.profileRevision++;
  }

  /** Runs a profile command; the backend's Spanish error when it fails, else `null`. */
  private async attempt(run: () => Promise<void>): Promise<string | null> {
    try {
      await run();
      this.profileError = null;
      return null;
    } catch (err) {
      return String(err);
    }
  }

  /** Makes `id` the active profile and applies its layout, language and theme (S5). */
  useProfile(id: number) {
    return this.attempt(async () => {
      const { active, appearance } = await switchProfile(this.profileApi, id, this.appearance());
      this.activate(active, appearance);
    });
  }

  /** A new profile with the active one's language and layout (S4); the active profile stays. */
  createProfile(name: string) {
    return this.attempt(async () => {
      const listed = await createProfile(this.profileApi, name, { language: this.language, layout: this.layout });
      this.profiles = listed.profiles;
    });
  }

  renameProfile(id: number, name: string) {
    return this.attempt(async () => {
      this.profiles = await renameProfile(this.profileApi, id, name, this.profiles);
      const active = this.profiles.find((p) => p.id === this.activeProfile?.id);
      if (active) this.activeProfile = active;
    });
  }

  /** Deletes a profile and its data; deleting the active one applies the profile the backend switched to. */
  deleteProfile(id: number) {
    return this.attempt(async () => {
      const activeId = this.activeProfile?.id ?? id;
      const result = await deleteProfile(this.profileApi, id, activeId, this.appearance());
      this.profiles = result.profiles;
      if (result.appearance) this.activate(result.active, result.appearance);
      else this.activeProfile = result.active;
    });
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

  /** Switching prose/words/code or the language loads a new text (the practice screen watches them). */
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
