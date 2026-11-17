# Words practice mode

A third practice kind next to Prosa and Código: loose words that train the weak units.
Branch `feat/words-mode` (from main after the AI fixes and Common Voice sentences). Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Mode.** The header switch reads "Prosa | Palabras | Código". "Palabras" is stored like the other kinds (`plexustyping.practiceKind = "words"`); a missing or unknown stored value still loads as prose. Switching loads a new text.
- **S2 — Text.** Loose words only ("palabras sueltas"): real words of the chosen language from the local corpus (no AI, even when the AI is enabled), all lowercase, separated by single spaces, no punctuation, no capitals. Words that contain the weak units are preferred (the same targets as prose); only characters the chosen layout can type. Length follows the "Longitud del texto" setting.
- **S3 — Sessions and statistics.** Words sessions feed the prose statistics model (same language and layout) and are saved like prose sessions; the history shows them with the mode "Palabras" (CSV `modo` column `palabras`).
- **S4 — Docs.** README describes the mode.
- **S5 — AI length margin (separate fix on this branch).** An AI text is accepted from 80 % to 125 % of the chosen length (300 → 240–375 characters), and the prompt states that range.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S4 | Words generator in the corpus crate, `words` practice mode through backend, history and frontend, README, tests | worker | done | see L4 |

## Log

- **L1 (user, verbatim, 2026-11-17):** "igual podemos poenr una opcion en vez de prosa, palabras".
- **L3 (evidence, 2026-11-18):** the running app logged "AI text from Gemini rejected: length 366 / 369 / 370 / 375" for a 300 setting (accepted 300–360) → S5, under the user's "haz los que creas mas optimo".
- **L4 (evidence, 2026-11-18):** `Corpus::generate_words`; `PracticeMode::Words` (no AI, no cache, prose stats, sessions mode `words`, CSV `palabras`); header "Prosa | Palabras | Código"; history filter "Palabras"; S5 `ai_length` 80–125 % for prose AI requests only (corpus range unchanged 300..361). RED observed (compile errors, 4 vitest failures). cargo 365, vitest 207, check 0, build ok, fmt clean.
- **L2 (user choices, 2026-11-17):** origin "Solo del corpus local"; format "Minúsculas, sin puntuación"; and (verbatim) "y seran palabras sueltas".
