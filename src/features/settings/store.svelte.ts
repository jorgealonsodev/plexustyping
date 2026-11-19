import { settingsStorage } from "./persist";
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
import { type GoalsApi, goalsApi } from "../goals/api";
import { type GoalChoice, type GoalSnapshot, crossedGoal, localDay, tzOffsetMin } from "../goals/goals";
import { type Profile, type ProfileFields, type ProfilesApi, profilesApi } from "../profiles/api";
import { type UnlockApi, type UnlockChange, type UnlockStatus, unlockApi } from "../unlock/api";
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
  loadColorBlind,
  loadDimPending,
  loadFlashes,
  loadGhost,
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
  saveColorBlind,
  saveDimPending,
  saveFlashes,
  saveGhost,
  saveHeatmap,
  saveLiveStats,
  saveMetronome,
  saveShowKeyboard,
  saveTargetCount,
  saveTextLength,
  type Typography,
  checkTypography,
  loadScreenReader,
  loadTypography,
  saveScreenReader,
  saveTypography,
} from "./settings";
import { loadPracticeFont, savePracticeFont } from "./fonts";
import { loadCursorBlink, loadCursorStyle, saveCursorBlink, saveCursorStyle } from "./cursor";

/** The live, persisted settings shared by the app and every settings section. Layout, language and
 * theme belong to the active profile (S5); the settings storage keeps them only as the first-paint cache. */
export class SettingsStore {
  private readonly profileApi: ProfilesApi;
  private readonly goalApi: GoalsApi;
  /** Latest goal read: an older reply (e.g. of the previous profile) is stale. */
  private goalRun = 0;
  private readonly unlockApi: UnlockApi;
  /** Latest unlock read: an older reply (another profile, language or layout) is stale. */
  private unlockRun = 0;

  constructor(api: ProfilesApi = profilesApi, goals: GoalsApi = goalsApi, unlocks: UnlockApi = unlockApi) {
    this.profileApi = api;
    this.goalApi = goals;
    this.unlockApi = unlocks;
  }

  /** Progressive unlock of the active profile in the current language and layout (S1); `null` until read. */
  unlock = $state.raw<UnlockStatus | null>(null);

  /** The active profile's daily goal, today's progress and streak (S5); `null` until read. */
  goal = $state.raw<GoalSnapshot | null>(null);

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
  /** Race the ghost of the best earlier session with the same text (ghost S3). */
  ghost = $state(loadGhost());
  showKeyboard = $state(loadShowKeyboard());
  flashes = $state(loadFlashes());
  /** Okabe–Ito blue/orange flashes and an underlined wrong key (a11y S5); off by default. */
  colorBlind = $state(loadColorBlind());
  /** Pending practice text dimmed to about 2.5:1 (a11y S7); on by default. */
  dimPending = $state(loadDimPending());
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
  /** Screen reader announcements in the practice (a11y S3); off by default. */
  screenReader = $state(loadScreenReader());
  /** Size, letter spacing and line height of the practice text, prose and code (a11y S4). */
  typography = $state.raw<Typography>(loadTypography());
  /** Typeface id of the practice text, prose and code (practice-font S1). */
  practiceFont = $state(loadPracticeFont());
  /** Look of the typing cursor and whether it blinks (cursor-style S1, S2). */
  cursorStyle = $state(loadCursorStyle());
  cursorBlink = $state(loadCursorBlink());
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

  setGhost(on: boolean) {
    this.ghost = on;
    saveGhost(on);
  }

  setShowKeyboard(on: boolean) {
    this.showKeyboard = on;
    saveShowKeyboard(on);
  }

  setFlashes(on: boolean) {
    this.flashes = on;
    saveFlashes(on);
  }

  setColorBlind(on: boolean) {
    this.colorBlind = on;
    saveColorBlind(on);
  }

  setDimPending(on: boolean) {
    this.dimPending = on;
    saveDimPending(on);
  }

  /** Loads a new text for the layout (the practice screen watches it); saved to the active profile. */
  setLayout(id: string) {
    this.layout = id;
    saveLayout(id);
    this.saveToProfile({ layout: id });
    void this.refreshUnlock();
  }

  /** Loads a new prose or words text in the language (the practice screen watches it); saved to the active profile. */
  setLanguage(id: string) {
    this.language = id;
    saveLanguage(id);
    this.saveToProfile({ language: id });
    void this.refreshUnlock();
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
  async loadProfiles(storage: Storage = settingsStorage()) {
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
    void this.refreshGoal();
    void this.refreshUnlock();
  }

  private activate(profile: Profile, appearance: Appearance) {
    this.setActive(profile);
    this.applyAppearance(appearance);
    this.profileRevision++;
    void this.refreshGoal();
    void this.refreshUnlock();
  }

  /** Applies an unlock reply unless a newer read started meanwhile. */
  private async readUnlock(read: (language: string, layout: string) => Promise<UnlockStatus>) {
    const id = ++this.unlockRun;
    const status = await read(this.language, this.layout);
    if (id === this.unlockRun) this.unlock = status;
  }

  /** Reads the unlock progress again (after a session, a switch of profile, language or layout). */
  async refreshUnlock() {
    try {
      await this.readUnlock((language, layout) => this.unlockApi.status(language, layout));
    } catch (err) {
      console.warn("unlock_status failed", err);
    }
  }

  /** Turns the mode on or off or sets its target speed; the backend's Spanish error when refused, else `null`. */
  async setUnlock(change: UnlockChange): Promise<string | null> {
    try {
      await this.readUnlock((language, layout) => this.unlockApi.set(language, layout, change));
      return null;
    } catch (err) {
      return String(err);
    }
  }

  /** Applies a goal reply unless a newer read started meanwhile. */
  private async readGoal(read: (tz: number) => Promise<GoalSnapshot["status"]>): Promise<GoalSnapshot | null> {
    const id = ++this.goalRun;
    const now = new Date();
    const status = await read(tzOffsetMin(now));
    const snapshot = { status, day: localDay(now) };
    if (id === this.goalRun) this.goal = snapshot;
    return snapshot;
  }

  /** Reads the active profile's goal progress again (after a session, a switch or a goal change). */
  async refreshGoal(): Promise<GoalSnapshot | null> {
    try {
      return await this.readGoal((tz) => this.goalApi.status(tz));
    } catch (err) {
      console.warn("goal_status failed", err);
      return null;
    }
  }

  /** After a saved session: refreshes the progress; `true` when this session met the goal (S5). */
  async sessionSaved(): Promise<boolean> {
    void this.refreshUnlock();
    const before = this.goal;
    const after = await this.refreshGoal();
    return after !== null && crossedGoal(before, after);
  }

  /** Sets the active profile's daily goal; the backend's Spanish error when refused, else `null`. */
  setGoal(kind: GoalChoice, amount: number | null) {
    return this.attempt(async () => {
      await this.readGoal((tz) => this.goalApi.set(kind, amount, tz));
    });
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

  setScreenReader(on: boolean) {
    this.screenReader = on;
    saveScreenReader(on);
  }

  /** Applies live; a value outside its range is clamped, an invalid one keeps the current value. */
  setTypography(patch: Partial<Typography>) {
    this.typography = checkTypography({ ...this.typography, ...patch }, this.typography);
    saveTypography(this.typography);
  }

  /** Applies live; an unknown id falls back to the default font. */
  setPracticeFont(id: string) {
    this.practiceFont = id;
    savePracticeFont(id);
  }

  setCursorStyle(id: string) {
    this.cursorStyle = id;
    saveCursorStyle(id);
  }

  setCursorBlink(on: boolean) {
    this.cursorBlink = on;
    saveCursorBlink(on);
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
