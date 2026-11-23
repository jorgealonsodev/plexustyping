import { expect } from "vitest";
import { LOCALES, type Translate, placeholders } from "./i18n.svelte";

/** Every language has every Spanish key, no extra ones, and the same `{placeholders}` (ui-i18n S5). */
export function expectComplete<K extends string>(t: Translate<K>): void {
  const keys = Object.keys(t.catalogs.es).sort();
  for (const locale of LOCALES) {
    const catalog = t.catalogs[locale];
    expect(Object.keys(catalog).sort(), `${locale} keys`).toEqual(keys);
    for (const key of keys as K[]) {
      const es = t.catalogs.es[key];
      const other = catalog[key];
      expect(typeof other, `${locale}.${key} kind`).toBe(typeof es);
      if (typeof es === "string" && typeof other === "string") {
        expect(placeholders(other), `${locale}.${key} placeholders`).toEqual(placeholders(es));
        expect(other.trim(), `${locale}.${key} empty`).not.toBe("");
      }
    }
  }
}
