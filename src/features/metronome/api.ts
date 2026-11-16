import { invoke } from "@tauri-apps/api/core";

export type Sound = "click" | "wood" | "beep";

/** `plexustyping-audio` `MetronomeConfig`; volume is a 0–1 gain. */
export interface MetronomeConfig {
  bpm: number;
  sound: Sound;
  volume: number;
  accentEvery: number;
}

/** Beat 0 of the running metronome, in ms since the UNIX epoch (device output latency excluded). */
export interface MetronomeStarted {
  epochMs: number;
}

export interface MetronomeApi {
  start(config: MetronomeConfig): Promise<MetronomeStarted>;
  /** `null` when the metronome is stopped. A new tempo restarts beat 0. */
  update(config: MetronomeConfig): Promise<MetronomeStarted | null>;
  stop(): Promise<void>;
}

export const metronomeApi: MetronomeApi = {
  start: (config) => invoke<MetronomeStarted>("metronome_start", { config }),
  update: (config) => invoke<MetronomeStarted | null>("metronome_update", { config }),
  stop: () => invoke<void>("metronome_stop"),
};
