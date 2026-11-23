// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { flushSync, mount, unmount } from "svelte";
import UiLanguageField from "./UiLanguageField.svelte";
import { UI_LANGUAGE_KEY, ui } from "./i18n.svelte";

describe("UiLanguageField (S3, S4)", () => {
  let app: ReturnType<typeof mount> | undefined;
  afterEach(() => {
    if (app) unmount(app);
    app = undefined;
    localStorage.clear();
    ui.set("es");
  });

  it("switches the interface live, stores the choice and leaves the text language alone", () => {
    localStorage.setItem("plexustyping.language", "fr");
    app = mount(UiLanguageField, { target: document.body });
    const select = document.querySelector("select")!;
    expect(select.value).toBe("system");
    expect(document.querySelector("label")!.textContent).toContain("Idioma de la interfaz");

    select.value = "de";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    flushSync();
    expect(ui.current).toBe("de");
    expect(localStorage.getItem(UI_LANGUAGE_KEY)).toBe("de");
    expect(localStorage.getItem("plexustyping.language")).toBe("fr");
    expect(document.querySelector("label")!.textContent).toContain("Sprache der Oberfläche");

    select.value = "system";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    flushSync();
    expect(localStorage.getItem(UI_LANGUAGE_KEY)).toBeNull();
    // jsdom's navigator says en-US.
    expect(ui.current).toBe("en");
  });
});
