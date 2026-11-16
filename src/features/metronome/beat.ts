// Pure metronome helpers (S7): beat timing, offsets and the rhythm summary.

import type { MetronomeConfig, Sound } from "./api";

/** Sounds in display order; the first is the default. */
export const SOUNDS: { id: Sound; label: string }[] = [
  { id: "click", label: "Clic" },
  { id: "wood", label: "Madera" },
  { id: "beep", label: "Bip" },
];

export const MIN_BPM = 30;
export const MAX_BPM = 300;
export const MAX_ACCENT_EVERY = 16;

/** The persisted metronome settings; volume in percent. */
export interface MetronomeSettings {
  enabled: boolean;
  bpm: number;
  sound: Sound;
  volume: number;
  accentEvery: number;
}

export const toConfig = (s: MetronomeSettings): MetronomeConfig => ({
  bpm: s.bpm,
  sound: s.sound,
  volume: s.volume / 100,
  accentEvery: s.accentEvery,
});

/** Beat 0 (epoch ms) and the tempo of the running metronome. */
export interface BeatAnchor {
  epochMs: number;
  intervalMs: number;
  bpm: number;
  accentEvery: number;
}

/** Ms from the nearest beat at or after beat 0 to `tMs`: negative = early, positive = late. */
export function beatOffsetMs(tMs: number, anchorMs: number, intervalMs: number): number {
  // Exactly halfway counts against the earlier beat (late).
  const beat = Math.max(Math.ceil((tMs - anchorMs) / intervalMs - 0.5), 0);
  return tMs - (anchorMs + beat * intervalMs);
}

/** The beat that last sounded at `tMs` and the time since it; `null` before beat 0. */
export function beatAt(tMs: number, anchorMs: number, intervalMs: number): { index: number; sinceMs: number } | null {
  if (tMs < anchorMs) return null;
  const index = Math.floor((tMs - anchorMs) / intervalMs);
  return { index, sinceMs: tMs - (anchorMs + index * intervalMs) };
}

/** Beat 0 and every `accentEvery`-th beat are accented; 0 disables the accent. */
export const isAccent = (index: number, accentEvery: number) => accentEvery > 0 && index % accentEvery === 0;

export interface RhythmSummary {
  /** Mean absolute offset, ms. */
  meanAbsMs: number;
  /** Shares of measured keys before and after the beat, in percent. */
  earlyPct: number;
  latePct: number;
}

/** Summary of the measured offsets; `null` when none was measured. */
export function rhythmSummary(offsets: (number | null)[]): RhythmSummary | null {
  const measured = offsets.filter((o): o is number => o !== null);
  if (measured.length === 0) return null;
  const share = (n: number) => (n / measured.length) * 100;
  return {
    meanAbsMs: measured.reduce((sum, o) => sum + Math.abs(o), 0) / measured.length,
    earlyPct: share(measured.filter((o) => o < 0).length),
    latePct: share(measured.filter((o) => o > 0).length),
  };
}

type ShortcutKey = Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "metaKey" | "shiftKey" | "repeat">;

/** A bare F4 turns the metronome on or off. */
export const isMetronomeShortcut = (k: ShortcutKey) =>
  k.key === "F4" && !k.ctrlKey && !k.altKey && !k.metaKey && !k.shiftKey && !k.repeat;
