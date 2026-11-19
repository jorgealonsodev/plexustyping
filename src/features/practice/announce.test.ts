import { describe, expect, it } from "vitest";
import {
  errorMessage,
  freshen,
  keystrokeMessage,
  liveText,
  newTextMessage,
  nextCharMessage,
  resultMessage,
  spokenChar,
} from "./announce";

describe("spoken chars (S3)", () => {
  it("names the invisible chars", () => {
    expect(spokenChar(" ")).toBe("espacio");
    expect(spokenChar("\n")).toBe("salto de línea");
    expect(spokenChar("\t")).toBe("tabulador");
  });

  it("names the punctuation", () => {
    expect(spokenChar(",")).toBe("coma");
    expect(spokenChar(".")).toBe("punto");
    expect(spokenChar(";")).toBe("punto y coma");
    expect(spokenChar(":")).toBe("dos puntos");
    expect(spokenChar("¿")).toBe("abre interrogación");
    expect(spokenChar("?")).toBe("cierra interrogación");
    expect(spokenChar("¡")).toBe("abre exclamación");
    expect(spokenChar("!")).toBe("cierra exclamación");
    expect(spokenChar("(")).toBe("abre paréntesis");
    expect(spokenChar(")")).toBe("cierra paréntesis");
    expect(spokenChar("-")).toBe("guion");
    expect(spokenChar('"')).toBe("comillas");
    expect(spokenChar("'")).toBe("apóstrofo");
  });

  it("marks capitals and keeps lowercase letters and digits as they are", () => {
    expect(spokenChar("a")).toBe("a");
    expect(spokenChar("ñ")).toBe("ñ");
    expect(spokenChar("7")).toBe("7");
    expect(spokenChar("E")).toBe("e mayúscula");
    expect(spokenChar("Á")).toBe("á mayúscula");
  });
});

describe("announcements (S3)", () => {
  it("announces the next char", () => {
    expect(nextCharMessage("a")).toBe("siguiente: a");
    expect(nextCharMessage(" ")).toBe("siguiente: espacio");
    expect(nextCharMessage("\n")).toBe("siguiente: salto de línea");
    expect(nextCharMessage(undefined)).toBe("");
  });

  it("announces an error with the expected and the typed char", () => {
    expect(errorMessage("e", "r")).toBe("error: esperaba e, pulsaste r");
    expect(errorMessage(" ", ",")).toBe("error: esperaba espacio, pulsaste coma");
    expect(errorMessage("\t", "\n")).toBe("error: esperaba tabulador, pulsaste salto de línea");
  });

  it("announces the next char after a correct keystroke or a backspace", () => {
    expect(keystrokeMessage(null, "a", true)).toBe("siguiente: a");
  });

  it("announces the error after a wrong keystroke, then the next char only when the cursor moved", () => {
    expect(keystrokeMessage({ expected: "e", typed: "r" }, "e", false)).toBe("error: esperaba e, pulsaste r");
    expect(keystrokeMessage({ expected: "e", typed: "r" }, "f", true)).toBe("error: esperaba e, pulsaste r. siguiente: f");
  });

  it("announces nothing for the next char once the text is complete", () => {
    expect(keystrokeMessage(null, undefined, true)).toBe("");
    expect(keystrokeMessage({ expected: "o", typed: "p" }, undefined, true)).toBe("error: esperaba o, pulsaste p");
  });

  it("announces the start of a new text with its length and first char", () => {
    expect(newTextMessage(["H", "o", "l", "a"])).toBe("Texto nuevo de 4 caracteres. siguiente: h mayúscula");
    expect(newTextMessage(["x"])).toBe("Texto nuevo de 1 carácter. siguiente: x");
  });

  it("announces the result line", () => {
    const stats = { netWpm: 41.6, accuracy: 0.975, durationMs: 42_500, errors: 3 };
    expect(resultMessage(stats)).toBe("Resultado: 42 PPM netas, precisión 97,5 %, tiempo 0:42, 3 errores.");
    expect(resultMessage({ ...stats, errors: 1 })).toBe("Resultado: 42 PPM netas, precisión 97,5 %, tiempo 0:42, 1 error.");
    expect(resultMessage({ ...stats, errors: 0, accuracy: 1 })).toBe("Resultado: 42 PPM netas, precisión 100 %, tiempo 0:42, sin errores.");
  });

  it("changes a repeated message so the live region announces it again", () => {
    const first = freshen("siguiente: a", "");
    expect(first).toBe("siguiente: a");
    const second = freshen("siguiente: a", first);
    expect(second).not.toBe(first);
    expect(second.trim()).toBe("siguiente: a");
    expect(freshen("siguiente: a", second)).toBe(first);
    expect(freshen("siguiente: b", first)).toBe("siguiente: b");
  });

  it("announces nothing while the setting is off", () => {
    expect(liveText(false, "siguiente: a")).toBe("");
    expect(liveText(false, "error: esperaba e, pulsaste r")).toBe("");
    expect(liveText(true, "siguiente: a")).toBe("siguiente: a");
  });
});
