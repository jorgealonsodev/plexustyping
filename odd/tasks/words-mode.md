# Words practice mode

A third practice kind next to Prosa and Código: loose words that train the weak units.
Branch `feat/words-mode` (from main after the AI fixes and Common Voice sentences). Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Mode.** The header switch reads "Prosa | Palabras | Código". "Palabras" is stored like the other kinds (`plexustyping.practiceKind = "words"`); a missing or unknown stored value still loads as prose. Switching loads a new text.
- **S2 — Text.** Loose words only ("palabras sueltas"): real words of the chosen language from the local corpus (no AI, even when the AI is enabled), all lowercase, separated by single spaces, no punctuation, no capitals. Words that contain the weak units are preferred (the same targets as prose); only characters the chosen layout can type. Length follows the "Longitud del texto" setting.
- **S3 — Sessions and statistics.** Words sessions feed the prose statistics model (same language and layout) and are saved like prose sessions; the history shows them with the mode "Palabras" (CSV `modo` column `palabras`).
- **S4 — Docs.** README describes the mode.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S4 | Words generator in the corpus crate, `words` practice mode through backend, history and frontend, README, tests | inline or worker (decided when it starts) | pending | |

## Log

- **L1 (user, verbatim, 2026-11-17):** "igual podemos poenr una opcion en vez de prosa, palabras".
- **L2 (user choices, 2026-11-17):** origin "Solo del corpus local"; format "Minúsculas, sin puntuación"; and (verbatim) "y seran palabras sueltas".
