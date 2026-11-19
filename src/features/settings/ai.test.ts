import { describe, expect, it } from "vitest";
import {
  fillMissingModel,
  type ChainEntry,
  type ProviderInfo,
  MAX_CHAIN,
  addEntry,
  aiToggleTitle,
  densityLabel,
  failureMessage,
  CHATGPT_USAGE_URL,
  chatgptAccount,
  chatgptReady,
  modelChoices,
  modelOptions,
  moveEntry,
  needsKey,
  signOutNotice,
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
  { id: "chatgpt", label: "ChatGPT (tu suscripción)", name: "ChatGPT", defaultModel: "" },
];

const claude: ChainEntry = { provider: "claude", model: "claude-haiku-5-5" };
const gemini: ChainEntry = { provider: "gemini", model: "g" };

describe("fallback chain editing", () => {
  it("adds the first unused provider with its default model, up to five (the backend's limit)", () => {
    let chain = [claude];
    chain = addEntry(chain, PROVIDERS)!;
    expect(chain).toEqual([claude, { provider: "openai", model: "gpt-5-mini" }]);
    chain = addEntry(chain, PROVIDERS)!;
    chain = addEntry(chain, PROVIDERS)!;
    chain = addEntry(chain, PROVIDERS)!;
    expect(chain.map((e) => e.provider)).toEqual(["claude", "openai", "gemini", "opencode-go", "chatgpt"]);
    expect(chain[3].model).toBe("");
    expect(chain[4]).toEqual({ provider: "chatgpt", model: "" });
    expect(MAX_CHAIN).toBe(5);
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
    expect(ids(providerChoices([claude, gemini], 1, PROVIDERS))).toEqual(["openai", "gemini", "opencode-go", "chatgpt"]);
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

describe("modelOptions", () => {
  it("shows display names, stores slugs and keeps an unlisted current model", () => {
    const listed = [
      { id: "gpt-6.2-sol", name: "GPT-6.2 Sol" },
      { id: "gpt-6.1-sol", name: "GPT-6.1 Sol" },
    ];
    expect(modelOptions(listed, "gpt-6.1-sol")).toEqual(listed);
    expect(modelOptions(listed, "old-slug")).toEqual([{ id: "old-slug", name: "old-slug" }, ...listed]);
    expect(modelOptions(listed, "")).toEqual(listed);
    expect(modelOptions([], "x")).toEqual([{ id: "x", name: "x" }]);
  });
});

describe("ChatGPT account", () => {
  const signedIn = { signedIn: true, email: "ana@example.com", planEnabled: true };

  it("takes no API key; every other provider does", () => {
    expect(needsKey("chatgpt")).toBe(false);
    for (const id of ["claude", "openai", "gemini", "opencode-go"] as const) expect(needsKey(id)).toBe(true);
  });

  it("is ready only when signed in with plan use enabled", () => {
    expect(chatgptReady(signedIn)).toBe(true);
    expect(chatgptReady({ ...signedIn, planEnabled: false })).toBe(false);
    expect(chatgptReady({ signedIn: false, email: null, planEnabled: false })).toBe(false);
    expect(chatgptReady(null)).toBe(false);
  });

  it("describes the account and whether plan use is enabled", () => {
    expect(chatgptAccount(signedIn)).toEqual({
      account: "Sesión iniciada como ana@example.com",
      plan: "El uso del plan está activado",
      ok: true,
    });
    expect(chatgptAccount({ signedIn: true, email: null, planEnabled: false })).toEqual({
      account: "Sesión iniciada",
      plan: "El uso del plan no está activado: inicia sesión de nuevo y permítelo",
      ok: false,
    });
    expect(chatgptAccount({ signedIn: false, email: null, planEnabled: false })).toEqual({
      account: "Sin sesión iniciada",
      plan: "",
      ok: false,
    });
  });

  it("warns when the sign-out could not be confirmed", () => {
    expect(signOutNotice({ revoked: false })).toBe("No se pudo confirmar la revocación");
    expect(signOutNotice({ revoked: true })).toBe("");
  });

  it("links the plan usage page", () => {
    expect(CHATGPT_USAGE_URL).toBe("https://chatgpt.com/settings/usage");
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

describe("fillMissingModel", () => {
  const list = [
    { id: "kimi-k2", name: "Kimi K2" },
    { id: "glm-5", name: "GLM 5" },
  ];

  it("picks the first listed model for a row of that provider without one", () => {
    // "cuando se elige un proveedor de ia, automaticamente tiene que elegirse un modelo"
    const chain = [
      { provider: "claude" as const, model: "claude-haiku-5-5" },
      { provider: "opencode-go" as const, model: "" },
    ];
    expect(fillMissingModel(chain, "opencode-go", list)).toEqual([
      { provider: "claude", model: "claude-haiku-5-5" },
      { provider: "opencode-go", model: "kimi-k2" },
    ]);
  });

  it("keeps a chosen model and returns null when nothing changes", () => {
    const chain = [{ provider: "opencode-go" as const, model: "glm-5" }];
    expect(fillMissingModel(chain, "opencode-go", list)).toBeNull();
    expect(fillMissingModel([{ provider: "chatgpt" as const, model: "" }], "chatgpt", [])).toBeNull();
    expect(fillMissingModel([{ provider: "chatgpt" as const, model: "" }], "opencode-go", list)).toBeNull();
  });
});
