import { describe, expect, it } from "vitest";
import type { MetronomeApi, MetronomeConfig } from "./api";
import { MetronomeController } from "./controller";

const cfg = (bpm: number, volume = 0.5): MetronomeConfig => ({ bpm, sound: "click", volume, accentEvery: 4 });

function fakeApi(fail?: string) {
  const calls: string[] = [];
  let epoch = 1000;
  let running: MetronomeConfig | null = null;
  const api: MetronomeApi = {
    start: async (c) => {
      calls.push(`start ${c.bpm}`);
      if (fail) throw fail;
      running = c;
      return { epochMs: (epoch += 1000) };
    },
    update: async (c) => {
      calls.push(`update ${c.bpm}`);
      if (!running) return null;
      if (running.bpm !== c.bpm) epoch += 1000;
      running = c;
      return { epochMs: epoch };
    },
    stop: async () => {
      calls.push("stop");
      running = null;
    },
  };
  return { api, calls };
}

describe("MetronomeController", () => {
  it("starts, updates live, restarts beat 0 on a new tempo and stops", async () => {
    const { api, calls } = fakeApi();
    const c = new MetronomeController(api, () => {});
    await c.set(cfg(60));
    expect(c.anchor).toEqual({ epochMs: 2000, intervalMs: 1000, bpm: 60, accentEvery: 4 });
    await c.set(cfg(60, 0.9));
    expect(c.anchor?.epochMs).toBe(2000);
    await c.set(cfg(120));
    expect(c.anchor).toEqual({ epochMs: 3000, intervalMs: 500, bpm: 120, accentEvery: 4 });
    await c.set(cfg(120));
    await c.set(null);
    await c.set(null);
    expect(c.anchor).toBeNull();
    expect(calls).toEqual(["start 60", "update 60", "update 120", "stop"]);
  });

  it("applies only the latest wish when calls overlap", async () => {
    const { api, calls } = fakeApi();
    const c = new MetronomeController(api, () => {});
    void c.set(cfg(60));
    void c.set(cfg(90));
    await c.set(null);
    expect(c.anchor).toBeNull();
    expect(calls).toEqual([]);
  });

  it("reports a start error, stays stopped and clears the error once stopped", async () => {
    const { api } = fakeApi("No hay dispositivo de audio");
    const seen: (string | null)[] = [];
    const c = new MetronomeController(api, () => seen.push(c.error));
    await c.set(cfg(60));
    expect(c.anchor).toBeNull();
    expect(c.error).toBe("No hay dispositivo de audio");
    await c.set(null);
    expect(c.error).toBeNull();
    expect(seen).toEqual(["No hay dispositivo de audio", null]);
  });
});
