// @vitest-environment jsdom
// Unlimited S2, S5 through the real Practice wiring: the hidden field's input events, the
// continuation request, the appended text and the waiting line at the end.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";

const calls: { cmd: string; args: Record<string, unknown> }[] = [];
type Pending = { resolve: (v: unknown) => void; reject: (e: unknown) => void };
const continuations: Pending[] = [];

// 600 chars, not ending in a space: a continuation is appended after one.
const FIRST = `${"perro casa ".repeat(54)}perros`;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (cmd: string, args: Record<string, unknown> = {}) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case "first_text":
      case "next_text":
        return Promise.resolve({ text: FIRST, targets: [], source: "ai", provider: "openai", model: "m" });
      case "continue_text":
        return new Promise((resolve, reject) => continuations.push({ resolve, reject }));
      case "key_sequences":
        return Promise.resolve([...(args.text as string)].map(() => null));
      case "weaknesses":
      case "heatmap":
      case "code_languages":
        return Promise.resolve([]);
      case "ghost_for_text":
        return Promise.resolve(null);
      case "get_layout":
        return Promise.reject(new Error("no layout in tests"));
      default:
        return Promise.resolve(null);
    }
  },
}));

const { default: Practice } = await import("./Practice.svelte");

const settle = async () => {
  for (let i = 0; i < 20; i++) {
    await Promise.resolve();
    flushSync();
  }
};

let target: HTMLElement;
let app: Record<string, unknown> | null = null;

beforeEach(() => {
  calls.length = 0;
  continuations.length = 0;
  Element.prototype.scrollIntoView ??= () => {};
  target = document.createElement("div");
  document.body.append(target);
});

afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
});

async function start() {
  app = mount(Practice, {
    target,
    props: {
      mode: "allow-continue",
      liveStats: false,
      showKeyboard: false,
      flashes: false,
      heatmap: false,
      targetCount: 3,
      textLength: 0, // «Ilimitado»
      aiRevision: 0,
      metronome: { enabled: false, bpm: 60, adaptive: false } as never,
      onTempo: () => {},
      practice: "prose",
      indent: "manual",
      codeLines: 12,
      layoutId: "es",
      languageId: "es",
    },
  });
  await settle();
}

const field = () => target.querySelector("textarea") as HTMLTextAreaElement;
const shown = () => [...target.querySelectorAll(".text .ch")].map((e) => e.textContent).join("");
const cursorAt = () => [...target.querySelectorAll(".text .ch")].findIndex((e) => e.classList.contains("cursor"));
const waiting = () => [...target.querySelectorAll(".hint")].some((e) => e.textContent === "Generando más texto…");
const continueCalls = () => calls.filter((c) => c.cmd === "continue_text");

/** Types `n` chars (wrong ones advance in allow-continue) through the field's `input` event. */
function type(n: number) {
  for (let i = 0; i < n; i++) {
    field().value = "x";
    field().dispatchEvent(new InputEvent("input", { inputType: "insertText", data: "x", bubbles: true }));
    flushSync();
  }
}

describe("Practice in unlimited mode", () => {
  it("asks for a continuation before the end, appends it, waits at the end and chains offline after a failure", async () => {
    await start();
    expect(shown()).toBe(FIRST);
    expect(target.textContent).toContain("Esc terminar");

    // 600 chars: at most 400 ahead is reached at cursor 200.
    type(199);
    expect(continueCalls()).toHaveLength(0);
    type(1);
    expect(continueCalls()).toHaveLength(1);
    expect(continueCalls()[0].args.offline).toBe(false);

    // The reply is appended after a single space without interrupting typing.
    continuations[0].resolve({ text: "sigue el texto", targets: [], source: "ai" });
    await settle();
    expect(shown()).toBe(`${FIRST} sigue el texto`);
    expect(cursorAt()).toBe(200);

    // The next request: at most 400 ahead of the new total (615) — cursor 215.
    type(14);
    expect(continueCalls()).toHaveLength(1);
    type(1);
    expect(continueCalls()).toHaveLength(2);
    expect(continueCalls()[1].args.offline).toBe(false);
    type(615 - 215);
    expect(cursorAt()).toBe(-1);
    expect(waiting()).toBe(true);

    // A failed continuation shows a notice; the next one chains a corpus text offline.
    continuations[1].reject("sin red");
    await settle();
    expect(target.querySelector(".notice")?.textContent).toContain("No se pudo continuar el texto");
    await new Promise((r) => setTimeout(r, 2600));
    await settle();
    expect(continueCalls()).toHaveLength(3);
    expect(continueCalls()[2].args.offline).toBe(true);
  }, 10_000);
});
