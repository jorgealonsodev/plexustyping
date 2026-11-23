import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import "./features/settings/fontFiles";
import { initSettingsStorage } from "./features/settings/persist";
import { initUiLocale, ui } from "./i18n/i18n.svelte";
import { setBackendLanguage } from "./i18n/backend";
import { applyTheme, initCustomThemes, loadThemeId, themeById } from "./features/themes";

// The UI settings come from the store before anything reads them, so the first render already
// shows them (settings-persistence S4); never rejects: an unreadable store falls back to localStorage.
await initSettingsStorage();
// The interface language (ui-i18n S2): the stored choice or the computer's, before the first paint;
// the backend writes its messages in it from here on.
ui.onChange(setBackendLanguage);
initUiLocale();
// Before mount, so the first paint already uses the saved theme; imported themes first, so a saved one is known.
initCustomThemes();
applyTheme(themeById(loadThemeId()));

const app = mount(App, { target: document.getElementById("app")! });

export default app;
