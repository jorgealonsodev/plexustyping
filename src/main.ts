import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { applyTheme, initCustomThemes, loadThemeId, themeById } from "./features/themes";

// Before mount, so the first paint already uses the saved theme; imported themes first, so a saved one is known.
initCustomThemes();
applyTheme(themeById(loadThemeId()));

const app = mount(App, { target: document.getElementById("app")! });

export default app;
