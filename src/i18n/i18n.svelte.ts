import { settingsStorage } from "../features/settings/persist";

/** Interface languages, the same five the app practises (ui-i18n S1); Spanish is the key source. */
export const LOCALES = ["es", "en", "pt", "fr", "de"] as const;
export type Locale = (typeof LOCALES)[number];

/** The interface-language choice: a language, or the computer's one (ui-i18n S4). */
export type UiLanguage = Locale | "system";

/** The language used when the computer's one is not supported (ui-i18n S2). */
export const FALLBACK_LOCALE: Locale = "en";

/** Each language in its own name, for the selector. */
export const LOCALE_NAMES: Record<Locale, string> = {
  es: "Español",
  en: "English",
  pt: "Português (Brasil)",
  fr: "Français",
  de: "Deutsch",
};

/** BCP 47 tag of each language for `Intl` (number and date formats). */
export const INTL_TAGS: Record<Locale, string> = {
  es: "es-ES",
  en: "en-US",
  pt: "pt-BR",
  fr: "fr-FR",
  de: "de-DE",
};

/** Stored interface language; absent means automatic. Global, not per profile (ui-i18n S4). */
export const UI_LANGUAGE_KEY = "plexustyping.uiLanguage";

const isLocale = (id: string | null): id is Locale => (LOCALES as readonly (string | null)[]).includes(id);

/** The first of `languages` (BCP 47 tags) that the interface supports, by base subtag; else English (S2). */
export function detectLocale(languages: readonly string[]): Locale {
  for (const tag of languages) {
    const base = tag.split(/[-_]/)[0].toLowerCase();
    if (isLocale(base)) return base;
  }
  return FALLBACK_LOCALE;
}

/** The computer's languages, most preferred first. */
export const systemLanguages = (): readonly string[] =>
  typeof navigator === "undefined" ? [] : navigator.languages?.length ? navigator.languages : [navigator.language ?? ""];

/** The language a choice shows: automatic detects it from `languages`. */
export const resolveLocale = (choice: UiLanguage, languages: readonly string[] = systemLanguages()): Locale =>
  choice === "system" ? detectLocale(languages) : choice;

/** The stored interface-language choice; automatic when absent or unknown. */
export function loadUiLanguage(storage: Storage = settingsStorage()): UiLanguage {
  const id = storage.getItem(UI_LANGUAGE_KEY);
  return isLocale(id) ? id : "system";
}

/** Stores a choice; automatic forgets the key, so a later computer language is followed again. */
export function saveUiLanguage(choice: UiLanguage, storage: Storage = settingsStorage()): void {
  if (choice === "system") storage.removeItem(UI_LANGUAGE_KEY);
  else storage.setItem(UI_LANGUAGE_KEY, choice);
}

/** The live interface language: every message read in a template follows it. */
class UiLocale {
  current = $state<Locale>(FALLBACK_LOCALE);
  private listener: ((locale: Locale) => void) | null = null;

  /** Shows `locale`: `<html lang>` and the backend's messages follow it. */
  set(locale: Locale): void {
    this.current = locale;
    if (typeof document !== "undefined") document.documentElement.lang = locale;
    this.listener?.(locale);
  }

  /** Called on every change (the backend's message language). */
  onChange(listener: ((locale: Locale) => void) | null): void {
    this.listener = listener;
  }

  /** The `Intl` tag of the current language. */
  get tag(): string {
    return INTL_TAGS[this.current];
  }
}

export const ui = new UiLocale();

/** Message parameters, interpolated as `{name}`. */
export type Params = Record<string, string | number>;
/** A message: text with `{placeholders}`, or a function for plurals and other grammar. */
export type Message = string | ((params: Params) => string);
export type Catalog<K extends string> = Record<K, Message>;

/** The `{placeholders}` of a text message, sorted and without repeats. */
export const placeholders = (text: string): string[] =>
  [...new Set([...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]))].sort();

const fill = (text: string, params: Params = {}) =>
  text.replace(/\{(\w+)\}/g, (all, name: string) => (name in params ? String(params[name]) : all));

/** A translator over one feature's catalogs: `es` defines the keys, every other language must have them all. */
export interface Translate<K extends string> {
  (key: K, params?: Params): string;
  /** Every catalog, for completeness tests. */
  readonly catalogs: Record<Locale, Catalog<K>>;
}

export function defineMessages<K extends string>(
  es: Catalog<K>,
  others: Record<Exclude<Locale, "es">, Catalog<K>>,
): Translate<K> {
  const catalogs: Record<Locale, Catalog<K>> = { es, ...others };
  const t = (key: K, params?: Params) => {
    const message = catalogs[ui.current][key] ?? es[key];
    return typeof message === "function" ? message(params ?? {}) : fill(message, params);
  };
  return Object.assign(t, { catalogs });
}

/** `one` or `other` by the plural rule of the interface language. */
export const plural = (n: number, forms: { one: string; other: string }): string =>
  new Intl.PluralRules(ui.tag).select(n) === "one" ? forms.one : forms.other;

/** `n` in the interface language's number format (decimal separator, grouping). */
export const formatNumber = (n: number, options?: Intl.NumberFormatOptions): string =>
  new Intl.NumberFormat(ui.tag, options).format(n);

/** Start-up: shows the stored choice, or the computer's language (S2). */
export function initUiLocale(storage: Storage = settingsStorage()): void {
  ui.set(resolveLocale(loadUiLanguage(storage)));
}
