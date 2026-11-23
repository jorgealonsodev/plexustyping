import { afterEach, describe, expect, it } from "vitest";
import { t } from "./app-messages";
import { ui } from "./i18n/i18n.svelte";
import { expectComplete } from "./i18n/testing";

afterEach(() => ui.set("es"));

describe("app header messages", () => {
  it("are complete in every language (S5)", () => expectComplete(t));

  it("keep today's Spanish and follow the interface language", () => {
    expect(t("profileTitle", { name: "Ana" })).toBe("Perfil activo: Ana. Cambiar en Ajustes → Perfiles");
    ui.set("en");
    expect(t("profileTitle", { name: "Ana" })).toBe("Active profile: Ana. Change it in Settings → Profiles");
    expect(t("hideKeyboard")).toBe("Hide keyboard (F2)");
  });
});
