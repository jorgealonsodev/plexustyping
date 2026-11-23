// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  LOCALES,
  UI_LANGUAGE_KEY,
  defineMessages,
  detectLocale,
  formatNumber,
  loadUiLanguage,
  placeholders,
  plural,
  resolveLocale,
  saveUiLanguage,
  ui,
} from "./i18n.svelte";

const memory = (entries: Record<string, string> = {}): Storage => {
  const map = new Map(Object.entries(entries));
  return {
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (k) => map.get(k) ?? null,
    key: (i) => [...map.keys()][i] ?? null,
    removeItem: (k) => void map.delete(k),
    setItem: (k, v) => void map.set(k, v),
  };
};

afterEach(() => ui.set("es"));

describe("detectLocale (S2)", () => {
  it("takes the first supported system language by its base subtag", () => {
    expect(detectLocale(["de-AT", "en-US"])).toBe("de");
    expect(detectLocale(["pt-PT"])).toBe("pt");
    expect(detectLocale(["ja-JP", "fr-CA", "es"])).toBe("fr");
    expect(detectLocale(["ES-mx"])).toBe("es");
  });

  it("falls back to English when no system language is supported", () => {
    expect(detectLocale(["ja-JP", "zh-CN"])).toBe("en");
    expect(detectLocale([])).toBe("en");
  });
});

describe("interface-language setting (S3, S4)", () => {
  it("is automatic unless a supported language is stored", () => {
    expect(loadUiLanguage(memory())).toBe("system");
    expect(loadUiLanguage(memory({ [UI_LANGUAGE_KEY]: "xx" }))).toBe("system");
    expect(loadUiLanguage(memory({ [UI_LANGUAGE_KEY]: "fr" }))).toBe("fr");
  });

  it("stores a language and forgets the key for automatic", () => {
    const s = memory();
    saveUiLanguage("de", s);
    expect(s.getItem(UI_LANGUAGE_KEY)).toBe("de");
    saveUiLanguage("system", s);
    expect(s.getItem(UI_LANGUAGE_KEY)).toBeNull();
  });

  it("resolves automatic to the system language and a choice to itself", () => {
    expect(resolveLocale("system", ["pt-BR"])).toBe("pt");
    expect(resolveLocale("system", ["ko"])).toBe("en");
    expect(resolveLocale("de", ["pt-BR"])).toBe("de");
  });

  it("never touches the practice-text language key", () => {
    const s = memory({ "plexustyping.language": "es" });
    saveUiLanguage("en", s);
    expect(s.getItem("plexustyping.language")).toBe("es");
  });
});

describe("defineMessages", () => {
  const t = defineMessages(
    { hello: "Hola, {name}", n: (p) => plural(Number(p.count), { one: "1 sesión", other: `${p.count} sesiones` }) },
    {
      en: { hello: "Hello, {name}", n: (p) => plural(Number(p.count), { one: "1 session", other: `${p.count} sessions` }) },
      pt: { hello: "Olá, {name}", n: (p) => `${p.count} sessões` },
      fr: { hello: "Bonjour, {name}", n: (p) => `${p.count} sessions` },
      de: { hello: "Hallo, {name}", n: (p) => `${p.count} Sitzungen` },
    },
  );

  it("follows the current interface language and fills placeholders", () => {
    ui.set("es");
    expect(t("hello", { name: "Ana" })).toBe("Hola, Ana");
    ui.set("de");
    expect(t("hello", { name: "Ana" })).toBe("Hallo, Ana");
  });

  it("runs function messages with the params, for plurals", () => {
    ui.set("en");
    expect(t("n", { count: 1 })).toBe("1 session");
    expect(t("n", { count: 3 })).toBe("3 sessions");
  });

  it("lists the placeholders of a message", () => {
    expect(placeholders("De {min} a {max} ({min})")).toEqual(["max", "min"]);
  });
});

describe("formatNumber", () => {
  it("uses the decimal separator of the interface language", () => {
    ui.set("es");
    expect(formatNumber(1234.5, { maximumFractionDigits: 1 })).toBe("1234,5");
    ui.set("en");
    expect(formatNumber(1234.5, { maximumFractionDigits: 1 })).toBe("1,234.5");
  });
});

describe("ui.set", () => {
  it("sets <html lang> and tells the backend", () => {
    const backend = vi.fn();
    ui.onChange(backend);
    ui.set("fr");
    expect(document.documentElement.lang).toBe("fr");
    expect(backend).toHaveBeenLastCalledWith("fr");
    ui.onChange(null);
  });

  it("lists the five app languages", () => {
    expect(LOCALES).toEqual(["es", "en", "pt", "fr", "de"]);
  });
});

describe("expectComplete", () => {
  it("accepts complete catalogs", async () => {
    const { expectComplete } = await import("./testing");
    const t = defineMessages({ a: "x {n}" }, { en: { a: "y {n}" }, pt: { a: "z {n}" }, fr: { a: "w {n}" }, de: { a: "v {n}" } });
    expectComplete(t);
  });

  it("rejects a translation that loses a placeholder", async () => {
    const { expectComplete } = await import("./testing");
    const t = defineMessages({ a: "x {n}" }, { en: { a: "y" }, pt: { a: "z {n}" }, fr: { a: "w {n}" }, de: { a: "v {n}" } });
    expect(() => expectComplete(t)).toThrow();
  });
});
