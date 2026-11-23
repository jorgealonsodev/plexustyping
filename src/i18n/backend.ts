import { invoke } from "@tauri-apps/api/core";
import type { Locale } from "./i18n.svelte";

/** Tells the backend the language of its messages; outside the app (plain browser) it only warns. */
export const setBackendLanguage = (language: Locale): void =>
  void invoke("set_ui_language", { language }).catch((err) => console.warn("set_ui_language failed", err));
