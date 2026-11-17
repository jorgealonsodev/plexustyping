import { describe, expect, it } from "vitest";
import { SYNTAX_CLASSES, type Token, codeKeyEvent, newlineMark, tokenClasses } from "./syntax";
import { practiceKey, practiceMode } from "./api";

describe("tokenClasses", () => {
  it("gives every char the class of the token covering it", () => {
    const tokens: Token[] = [
      { start: 0, end: 2, class: "keyword" },
      { start: 2, end: 3, class: "plain" },
      { start: 3, end: 4, class: "function" },
    ];
    expect(tokenClasses(4, tokens)).toEqual(["keyword", "keyword", "plain", "function"]);
  });

  it("leaves chars without a token plain and ignores tokens past the end", () => {
    expect(tokenClasses(3, [{ start: 1, end: 9, class: "string" }])).toEqual(["plain", "string", "string"]);
    expect(tokenClasses(2, undefined)).toEqual(["plain", "plain"]);
    expect(tokenClasses(0, [])).toEqual([]);
  });

  it("lists the 14 semantic classes of plexustyping-highlight", () => {
    expect(SYNTAX_CLASSES).toEqual([
      "plain",
      "keyword",
      "string",
      "number",
      "comment",
      "function",
      "type",
      "variable",
      "constant",
      "operator",
      "punctuation",
      "tag",
      "attribute",
      "property",
    ]);
  });
});

describe("codeKeyEvent", () => {
  it("maps Enter to a newline char and Tab to a Tab event in code mode", () => {
    expect(codeKeyEvent("enter", true, 12)).toEqual({ kind: "char", ch: "\n", tMs: 12 });
    expect(codeKeyEvent("tab", true, 13)).toEqual({ kind: "tab", tMs: 13 });
  });

  it("types nothing in prose or for other keys", () => {
    expect(codeKeyEvent("enter", false, 1)).toBeNull();
    expect(codeKeyEvent("tab", false, 1)).toBeNull();
    expect(codeKeyEvent("backspace", true, 1)).toBeNull();
    expect(codeKeyEvent("escape", true, 1)).toBeNull();
    expect(codeKeyEvent(null, true, 1)).toBeNull();
  });
});

describe("newlineMark", () => {
  it("marks a newline only at the cursor or when it was typed wrong", () => {
    expect(newlineMark("\n", "pending", true)).toBe("↵");
    expect(newlineMark("\n", "error", false)).toBe("↵");
    expect(newlineMark("\n", "pending", false)).toBe("");
    expect(newlineMark("\n", "correct", false)).toBe("");
    expect(newlineMark("a", "error", true)).toBe("");
  });
});

describe("practice modes", () => {
  it("builds the IPC mode and a key that changes with the kind and the language", () => {
    expect(practiceMode("prose", "rust")).toBe("prose");
    expect(practiceMode("code", "rust")).toEqual({ kind: "code", lang: "rust" });
    expect(practiceKey("prose")).toBe("prose");
    expect(practiceKey({ kind: "code", lang: "rust" })).toBe("code:rust");
    expect(practiceKey({ kind: "code", lang: "python" })).toBe("code:python");
  });
});
