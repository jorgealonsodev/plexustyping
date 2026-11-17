# AI toggle in the header

A header button "IA" next to the new-text button decides where the next text comes from.
Branch `fix/ai-target-density` (follows the AI fixes there). Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Button.** The header shows a toggle button labelled "IA" right before the new-text (round arrows) button, same style as the other header buttons, `aria-pressed` true when on, title "IA activada: los textos nuevos se generan con IA" / "IA desactivada: textos guardados". It is the same setting as "Activar IA" in Ajustes → IA (`AiSettings.enabled`): flipping one updates the other, and it is saved like that setting. A change invalidates the prefetched next text (as other AI settings do).
- **S2 — On.** With "IA" on, a new text (button, F5, "Siguiente texto") is generated with the AI exactly as today (fallback chain; on failure an unused cached text, else the corpus).
- **S3 — Off.** With "IA" off, a new prose or code text is a saved text: a cached AI text of the same language and mode, preferring the ones sharing the most weak targets and otherwise the least recently shown one, so consecutive texts do not repeat (`take_cached_text`, which rotates); when there is no saved text of that language and mode, the local corpus. The source label stays "Texto del caché" / "Corpus local"; no AI request is made and no notice is shown.
- **S4 — Docs.** README describes the button.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S4 | Header toggle bound to the AI enabled setting; AI-off path serves rotating cached texts then corpus; tests; README | worker (context backstop) | done | see Log L3 |

## Log

- **L1 (user, verbatim, 2026-11-17):** "aqui pon un boton que diga ia y si esta pulsado y le das a regenerar, crea un texto nuevo con ia, si no esta pulsado al regenerar saca l os textos guardados".
- **L2 (decision):** the button is the existing "Activar IA" setting, not a second switch; "textos guardados" are the cached AI texts, served in rotation (least recently shown first) so they do not repeat back to back.
- **L3 (evidence, 2026-11-17):** worker: AI-off `next_text` serves `take_cached_text` (rotating) then corpus, no request; header toggle bound to `AiSettings.enabled` through `SettingsStore.aiEnabled`, synced with AiSection. RED observed (backend `Corpus` vs `Cache`; vitest helpers missing). cargo 346, vitest 203, check 0, build ok. Not covered by automated tests: the button's look and the header/Ajustes sync (no DOM test runner).
