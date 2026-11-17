# AI target density

Fix: AI prose texts are rejected as "La IA no devolvió un texto válido" because the target-density check asks for an impossible count.
Branch `fix/ai-target-density`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`.

## Specs

- **S1 — Realistic requirement.** A target must appear at least `max(1, ceil(density × natural frequency of the target × text length))` times, where the natural frequency is how often the target occurs per character in the corpus of the requested language (case-insensitive, overlapping counts as `str::matches` does). The fixed `BASELINE_PER_CHAR = 0.02` is no longer used for that. Example from the log: a 340-char Spanish text with targets `ntr`, `ú`, `ti` must no longer need 21 occurrences each.
- **S2 — The prompt states the count.** The prompt names, for each target, the exact minimum the check will apply (e.g. "`ntr` al menos 3 veces"), computed by the same function the validator uses, so the model knows what will be checked. The prompt stays in Spanish.
- **S3 — Visible reason.** When a generated text is rejected, the concrete validation issues (length, characters not allowed, Markdown, low density per target with found/needed) are written to stderr, without any part of the API key or the prompt secrets. The user-facing notice stays "La IA no devolvió un texto válido".
- **S5 — Simpler prompt with a word-run fallback.** The prose prompt is shorter and easier to satisfy (a few plain Spanish instructions). It allows: natural sentences first and, when the targets cannot be reached with sentences that make sense, the rest may be a run of real words of the language containing the targets, separated by spaces (like the corpus pseudo-sentences). Such a text passes validation.
- **S6 — Fewer targets.** Each text practises the 2 or 3 worst units (setting "Objetivos por texto": 2 or 3, default 2; earlier stored 4 or 5 load as 2; backend clamps to 2..=3).
- **S4 — Code mode** keeps working (code targets use the code corpus or, if none applies, a minimum of 1 per target); no regression in existing AI tests.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S6 | Natural-frequency requirement shared by prompt and validator, stderr reason, tests | worker (context backstop) | pending | |

## Log

- **L1 (user, verbatim, 2026-11-17):** "revisa por que falla la ia al generar un texto en prosa" — then "no parece dificil de generar".
- **L2 (diagnosis):** `validate.rs:94` needs `ceil(density × 0.02 × len)` per target; with density 3 and 340 chars that is 21 each, while the AI's own natural 340-char text has `ti`, `cos`, `art`, `ntr`, `ú` once and `ñ` twice. The prompt only says "al menos 3 veces más de lo habitual". No log records the rejection reason.
- **L3 (user, verbatim, 2026-11-17):** "tambien con openai podemos añadir login con oauth, que creo que esta permitido, para cosas como esta de codigo y demas, como en opencode o pi" — read as acceptance of the proposed fix plus a separate request (researched apart before any change).
- **L4 (user, verbatim, 2026-11-17):** "crea un prompt que sea mas sencillo de cumplir, y si el texto no tiene sentido a partir de un punto, pues palabras conectadas con las letras qeu tenemos que practicar" → S5, sent to the running worker.
- **L5 (worker evidence + parent fix, 2026-11-17):** requirement = `max(1, ceil(min(density × frequency, frequency + 0.05) × len))`, frequency from the language's corpus sentences; without the 5-point cap a frequent letter like `a` would need 123 of 340 chars. Log example now needs ntr 2, ú 1, ti 3. Prompt rewritten (7 short lines, per-target counts, word-run fallback). Rejections logged to stderr. `cargo test --workspace` 330 passed, vitest 201 passed, `pnpm check` 0, `cargo fmt` applied.
- **L6 (user, verbatim, 2026-11-17):** "para el tema de generera texto, con que coja las 2 o 3 letras que mas se fallan llega" → S6.
