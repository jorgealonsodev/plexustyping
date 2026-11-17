# AI target density

Fix: AI prose texts are rejected as "La IA no devolvió un texto válido" because the target-density check asks for an impossible count.
Branch `fix/ai-target-density`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`.

## Specs

- **S1 — Realistic requirement.** A target must appear at least `max(1, ceil(density × natural frequency of the target × text length))` times, where the natural frequency is how often the target occurs per character in the corpus of the requested language (case-insensitive, overlapping counts as `str::matches` does). The fixed `BASELINE_PER_CHAR = 0.02` is no longer used for that. Example from the log: a 340-char Spanish text with targets `ntr`, `ú`, `ti` must no longer need 21 occurrences each.
- **S2 — The prompt states the count.** The prompt names, for each target, the exact minimum the check will apply (e.g. "`ntr` al menos 3 veces"), computed by the same function the validator uses, so the model knows what will be checked. The prompt stays in Spanish.
- **S3 — Visible reason.** When a generated text is rejected, the concrete validation issues (length, characters not allowed, Markdown, low density per target with found/needed) are written to stderr, without any part of the API key or the prompt secrets. The user-facing notice stays "La IA no devolvió un texto válido".
- **S4 — Code mode** keeps working (code targets use the code corpus or, if none applies, a minimum of 1 per target); no regression in existing AI tests.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S4 | Natural-frequency requirement shared by prompt and validator, stderr reason, tests | worker (context backstop) | pending | |

## Log

- **L1 (user, verbatim, 2026-11-17):** "revisa por que falla la ia al generar un texto en prosa" — then "no parece dificil de generar".
- **L2 (diagnosis):** `validate.rs:94` needs `ceil(density × 0.02 × len)` per target; with density 3 and 340 chars that is 21 each, while the AI's own natural 340-char text has `ti`, `cos`, `art`, `ntr`, `ú` once and `ñ` twice. The prompt only says "al menos 3 veces más de lo habitual". No log records the rejection reason.
- **L3 (user, verbatim, 2026-11-17):** "tambien con openai podemos añadir login con oauth, que creo que esta permitido, para cosas como esta de codigo y demas, como en opencode o pi" — read as acceptance of the proposed fix plus a separate request (researched apart before any change).
