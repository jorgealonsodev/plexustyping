import { describe, expect, it } from "vitest";
import {
  type ChainEntry,
  type ProviderInfo,
  MAX_CHAIN,
  addEntry,
  aiToggleTitle,
  densityLabel,
  failureMessage,
  modelChoices,
  moveEntry,
  providerChoices,
  removeEntry,
  setModel,
  setProvider,
  toggleEnabled,
  usageCells,
} from "./ai";

const PROVIDERS: ProviderInfo[] = [
  { id: "claude", label: "Claude (Anthropic)", name: "Claude", defaultModel: "claude-haiku-5-5" },
  { id: "openai", label: "OpenAI", name: "OpenAI", defaultModel: "gpt-5-mini" },
  { id: "gemini", label: "Google Gemini", name: "Gemini", defaultModel: "gemini-3.5-flash-lite" },
  { id: "opencode-go", label: "OpenCode Go", name: "OpenCode Go", defaultModel: "" },
];

const claude: ChainEntry = { provider: "claude", model: "claude-haiku-5-5" };
const gemini: ChainEntry = { provider: "gemini", model: "g" };

describe("fallback chain editing", () => {
  it("adds the first unused provider with its default model, up to four", () => {
    let chain = [claude];
    chain = addEntry(chain, PROVIDERS)!;
    expect(chain).toEqual([claude, { provider: "openai", model: "gpt-5-mini" }]);
    chain = addEntry(chain, PROVIDERS)!;
    chain = addEntry(chain, PROVIDERS)!;
    expect(chain.map((e) => e.provider)).toEqual(["claude", "openai", "gemini", "opencode-go"]);
    expect(chain[3].model).toBe("");
    expect(chain).toHaveLength(MAX_CHAIN);
    expect(addEntry(chain, PROVIDERS)).toBeNull();
  });

  it("removes a row but never the last one", () => {
    expect(removeEntry([claude, gemini], 0)).toEqual([gemini]);
    expect(removeEntry([claude], 0)).toEqual([claude]);
    expect(removeEntry([claude, gemini], 5)).toEqual([claude, gemini]);
  });

  it("moves rows up and down within bounds without mutating the input", () => {
    const chain = [claude, gemini];
    expect(moveEntry(chain, 1, -1)).toEqual([gemini, claude]);
    expect(moveEntry(chain, 0, 1)).toEqual([gemini, claude]);
    expect(moveEntry(chain, 0, -1)).toEqual(chain);
    expect(moveEntry(chain, 1, 1)).toEqual(chain);
    expect(chain).toEqual([claude, gemini]);
  });

  it("changes a row's provider to an unused one with its default model, never to a duplicate", () => {
    expect(setProvider([claude, gemini], 0, "openai", PROVIDERS)).toEqual([
      { provider: "openai", model: "gpt-5-mini" },
      gemini,
    ]);
    expect(setProvider([claude, gemini], 0, "gemini", PROVIDERS)).toEqual([claude, gemini]);
    expect(setProvider([claude], 0, "claude", PROVIDERS)).toEqual([claude]);
  });

  it("offers each row its own provider plus the unused ones", () => {
    const ids = (list: ProviderInfo[]) => list.map((p) => p.id);
    expect(ids(providerChoices([claude, gemini], 1, PROVIDERS))).toEqual(["openai", "gemini", "opencode-go"]);
  });

  it("sets a trimmed model", () => {
    expect(setModel([claude, gemini], 1, "  gemini-pro ")).toEqual([claude, { provider: "gemini", model: "gemini-pro" }]);
  });
});

describe("modelChoices", () => {
  it("keeps the listed models and adds the current one when missing", () => {
    expect(modelChoices(["a", "b"], "b")).toEqual(["a", "b"]);
    expect(modelChoices(["a"], "custom")).toEqual(["custom", "a"]);
    expect(modelChoices(["a"], "")).toEqual(["a"]);
    expect(modelChoices([], "x")).toEqual(["x"]);
  });
});

describe("labels", () => {
  it("shows the density as a multiplier with a Spanish decimal comma", () => {
    expect(densityLabel(3)).toBe("3×");
    expect(densityLabel(2.5)).toBe("2,5×");
  });

  it("reads the message of a typed AI failure or any other error", () => {
    expect(failureMessage({ kind: "auth", message: "Claude rechazó la clave" })).toBe("Claude rechazó la clave");
    expect(failureMessage("boom")).toBe("boom");
    expect(failureMessage(new Error("x"))).toBe("Error: x");
  });

  it("sums the usage tokens per row with the provider name", () => {
    const row = {
      provider: "gemini",
      callsToday: 2,
      callsTotal: 3,
      inputTokensToday: 30,
      outputTokensToday: 10,
      inputTokensTotal: 1300,
      outputTokensTotal: 50,
    };
    expect(usageCells(row, PROVIDERS)).toEqual({ name: "Gemini", calls: 3, today: 40, total: 1350 });
    expect(usageCells({ ...row, provider: "otro" }, PROVIDERS).name).toBe("otro");
  });
});

describe("header AI toggle", () => {
  it("titles the button by the AI state", () => {
    expect(aiToggleTitle(true)).toBe("IA activada: los textos nuevos se generan con IA");
    expect(aiToggleTitle(false)).toBe("IA desactivada: textos guardados");
  });

  it("flips only enabled and keeps the rest of the settings", () => {
    const s = { enabled: false, chain: [claude], density: 3, topic: "comida" };
    expect(toggleEnabled(s)).toEqual({ ...s, enabled: true });
    expect(toggleEnabled({ ...s, enabled: true })).toEqual(s);
    expect(s.enabled).toBe(false);
  });
});
