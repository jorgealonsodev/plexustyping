# Phase 3 · Adaptive metronome

Second block of phase 3 ("Metrónomo adaptativo"). The PRD is not available on this machine; the spec below is the agent's proposal under the user's delegation (L1) and every rule in it is an assumption until the user reviews it.
Branch `feat/phase-3-adaptive-metronome`. Delivery: one work unit, frontend only. Runner: `pnpm test` (vitest), `pnpm check`.

## Specs

- **S1 — Setting.** "Ajustes → Metrónomo" gains a toggle "Tempo adaptativo" (off by default, persisted with the other metronome settings in `plexustyping.metronome`). A stored value without the field, or with an invalid one, loads as off. Sound, volume, accent, F4 and the fixed tempo behave as before when it is off.
- **S2 — Rule (assumption).** After a finished text that was typed entirely at one metronome tempo, the next tempo is computed from that text's accuracy and its mean absolute offset relative to the beat interval (`60000 / bpm` ms):
  - raise when accuracy ≥ 97 % and the mean absolute offset ≤ 15 % of the interval;
  - lower when accuracy < 90 % or the mean absolute offset > 20 % of the interval;
  - keep otherwise.
  - Step: 5 % of the current tempo, rounded, at least 2 BPM. Result clamped to 30–300 BPM.
  - No change when the text had no single tempo or no measured offsets (metronome off, switched or retuned mid-text).
- **S3 — Applying it.** With the toggle on, the new tempo replaces the "Tempo" setting when the results appear, so the next text (and "Repetir") plays at it and the settings slider shows it. Changing the tempo by hand still works and is the new starting point.
- **S4 — Feedback.** The results screen shows, under the rhythm line, "Tempo: 60 → 63 BPM" when it changed and "Tempo: se mantiene en 60 BPM" when it did not; nothing when the toggle is off or S2 did not apply.
- **S5 — Docs.** README describes the adaptive tempo and its rule.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S5 | Pure `adaptTempo` + setting + toggle + apply on results + results line + README, with vitest | inline (one unit, ~150 lines; no writer reason) | in progress | — |

## Log

- **L1 (user, verbatim):** "donde estabamos" / "continua con lo que consideres mas optimo". Trello card "Fase 3 · Completo": "Metrónomo adaptativo, todos los layouts e idiomas, todos los temas, importar temas VS Code, histórico y exportar CSV, perfiles." History + CSV is done (`odd/tasks/phase-3-history.md`); the agent chose "Metrónomo adaptativo" next because the metronome, the per-key offset metric and the session tempo already exist.
- **L2 (exploration):** `MetronomeSettings` (`src/features/metronome/beat.ts`) is persisted as JSON by `src/features/settings/settings.ts` (`checkMetronome`); `Practice.svelte` `finish()` knows the single session tempo (`measured.bpms`) and receives `ScoredSession` with `result.stats.accuracy` (0–1) and `offsetsMs`; `rhythmSummary` gives the mean absolute offset; `toConfig` picks fields explicitly, so a new setting does not reach the audio backend. No backend change needed.
- **L3 (decision):** thresholds are relative to the beat interval because random typing averages 25 % of it; 15 % / 20 % leave a dead band so the tempo does not oscillate.
