# More text lengths and unlimited mode

More length choices, plus an "Ilimitado" length: a text that keeps growing while you type, continued by the AI in the background. Branch `feat/unlimited-text` (on top of `fix/settings-advisories`).

## Specs

- **S1 — More lengths.** User: "mas opciones de tamaño".
  - "Longitud" offers 100, 200, 300, 450, 600, 800, 1000, 1500 and 2000 characters, plus "Ilimitado". The default stays 300.
  - The backend accepts lengths up to 2000; the clamp in `NextTextRequest::length` goes from 50..=600 to 50..=2000.
  - The AI and corpus paths produce texts at those lengths. The AI accepted range stays `ai_length` (80–125 %).
  - Code mode keeps its own line limit and ignores the length.
- **S2 — Unlimited.** User: "Y un ilimitado, que correra con la ia por detras, que genere textos largos y couando vaya por la mitad qeu continue el texto", "generando mas".
  - "Ilimitado" applies to prose and words. In code mode it behaves like 600.
  - The session starts with a 600-character text from the usual source (AI, cache or corpus).
  - When the cursor reaches half of the current text, a continuation is requested in the background, at most one at a time. When it arrives, it is appended to the text after a single space, without interrupting typing, and the next request triggers at half of the new total. So there is always text ahead.
  - Prose continuation: the AI continues the text from its last ~400 characters, in the same language, at about 600 characters, using only the allowed characters (layout and progressive unlock). It does not repeat the given text, and it does not start a new title or a list.
  - Without the AI, or when it fails, the continuation is a corpus text of about 600 characters, with the reason as a non-blocking notice, as today.
  - Words mode continues with more corpus words, never the AI.
  - Continuations are not cached.
- **S3 — Ending an unlimited session.** Escape or a "Terminar" button ends it. That scores the text typed so far (chars 0..cursor) as a completed session with the usual results, saving, statistics, daily goal and unlock progress. A session with no typed chars is discarded without scoring.
  - The typing hint names "Esc terminar" in unlimited mode.
  - The ghost session and the adaptive tempo change are off in unlimited mode. The metronome itself keeps working.
- **S4 — Contract between frontend and backend.**
  - New Tauri command `continue_text`, with camelCase arguments `{ language, layout, mode, tail, targetCount, seed, day }`. It returns the same `PracticeText`/`NextText` shape as `next_text` (text, targets, source, provider, model, notice).
    - `mode` is "prose" or "words".
    - `tail` is the last ≤ 400 characters of the current text; the backend refuses more than 2000 with a Spanish error.
  - Validation is the same as `next_text`: unknown language or layout gives the existing errors. Usage is counted as for `next_text`.
  - Privacy: the tail is generated text, never typed input.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1 (backend), S2 (continuation), S4 | Length clamp to 2000, `continue_text` command, continuation prompt (AI crate), corpus and words fallback, tests | worker A (parallel) | pending | |
| T2 | S1 (frontend), S2 (session growth), S3 | Length choices with "Ilimitado", engine `extend`, Practice wiring (trigger at half, append, end with Esc or Terminar, score the prefix, ghost and adaptive off), api `continueText`, hint, README, tests | worker B (parallel) | pending | |
| T3 | S1–S4 | Integration: cargo, clippy, vitest, check, build; .deb for the user's try-out | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-19):** "mas opciones de tamaño. Y un ilimitado, que correra con la ia por detras, que genere textos largos y couando vaya por la mitad qeu continue el texto" and then "generando mas".
- **L2 (agent decisions, 2026-11-19, under the standing "haz lo que consideres mas optimo"):**
  - Lengths: 800, 1000, 1500 and 2000 are added.
  - Unlimited starts at 600 and continues in chunks of about 600.
  - The trigger is half of the current text.
  - It ends with Esc or "Terminar", scoring the typed prefix.
  - Ghost and adaptive tempo are off in unlimited mode.
  - Words mode continues from the corpus.
  - Code ignores unlimited.
  - Continuations are not cached.
- **L3 (evidence, 2026-11-19):**
  - Today's frontend lengths are `TEXT_LENGTHS = [100, 200, 300, 450, 600]` (src/features/settings/settings.ts:140).
  - The backend clamps to 50..=600 (src-tauri/src/commands.rs:380) and computes the AI range in `ai_length` (commands.rs:400).
  - Providers cap the output at `MAX_TOKENS = 4096` (crates/plexustyping-ai/src/providers.rs:41), which is enough for 2000 characters.
