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
- **S5 — Hardening (review-fc4bf6e9e18ff737 advisories).**
  - In unlimited mode, reaching the end of the text never finishes the session. The typing waits at the end with a quiet "Generando más texto…" line, keeps accepting Esc and "Terminar", and goes on as soon as the continuation arrives.
  - The first continuation is requested when at most 400 characters remain ahead, not only at half of the text, so a slow AI chain has more room.
  - A continuation that arrives without a notice clears the previous continuation notice.
  - Ending a session scores only the events up to the cursor's last position, so a wrong keystroke right before Esc, or erased characters past the cursor, never make the replay of the prefix fail. The backend replay of the prefix must succeed for every error mode (backspace-correct, stop-on-error, allow-continue) and is tested with each.

- **S6 — Repeating an ended unlimited text.** Under the standing "haz lo que consideres mas optimo" (L8).
  - "Repetir" (button or Esc) after ending an unlimited text repeats only the typed prefix that was scored, as a normal text: it ends at its last char, asks no continuation, shows no "Terminar" or "Esc terminar", and races the attempt just saved as its ghost.
  - The next text ("Siguiente") is unlimited again.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1 (backend), S2 (continuation), S4 | Length clamp to 2000, `continue_text` command, continuation prompt (AI crate), corpus and words fallback, tests | worker A (parallel) | done | see L4 |
| T2 | S1 (frontend), S2 (session growth), S3 | Length choices with "Ilimitado", engine `extend`, Practice wiring (trigger at half, append, end with Esc or Terminar, score the prefix, ghost and adaptive off), api `continueText`, hint, README, tests | worker B (parallel) | done | see L4 |
| T4 | S5 | Hardening of the unlimited session | worker | done | see L5 |
| T5 | S6 | Repeat the typed prefix after ending; jsdom test; README | parent (inline) | done | see L8 (the fix commit) |
| T3 | S1–S4 | Integration: cargo, clippy, vitest, check, build; .deb for the user's try-out | parent | done | see L4 |

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
- **L4 (evidence, 2026-11-19):**
  - Backend (worker A): lengths are clamped to 50..=2000 (MIN_LENGTH/MAX_LENGTH).
    - `continue_text(language, layout, mode, tail, targetCount, seed, day)` is backed by `GenerationRequest::continue_from` and a continuation prompt.
    - Continuations are not cached but count usage; `ai_text` is shared with `next_text`.
    - Corpus or words fallback; a tail over 2000 is refused with «El texto a continuar admite como máximo 2000 caracteres».
  - Frontend (worker B):
    - Lengths 100–2000 plus «Ilimitado» (stored as 0). loadChoice now treats a missing stored value as the default.
    - `TypingSession.extend`; `unlimited.ts` (start 600, tail 400, trigger at half, retry 50 chars after a failure).
    - `escapeAction` and «Terminar» score the typed prefix. Ghost and adaptive tempo are off in unlimited mode.
    - The hint names «Esc terminar»; README updated.
  - RED observed in both. Integration: cargo 605/605, clippy 0, fmt ok, vitest 530/530, check 0, build ok, tarball 62/62.
  - Known limits:
    - «Repetir» after ending repeats the whole grown text.
    - Lengths 800–2000 were tested with the Spanish corpus only.
    - With the AI off, a continuation is a corpus text without a notice, as next_text does.
    - Scoring the prefix ignores keystrokes made after the prefix point that were then backspaced.
    - The Svelte wiring is untested; it needs a GUI check.
- **L5 (user report and fix, 2026-11-19):** the user wrote "he llegado al final y noo ha generado texto nuevo", with a screenshot of ChatGPT and the end of the text reached. Root cause: a failed continuation was retried only after the cursor advanced 50 chars, which is impossible at the end of the text, and the failure was only logged with console.warn: a silent, permanent stall.
  - T4 (worker):
    - The trigger also fires at ≤400 chars ahead.
    - The end of the text waits with «Generando más texto…» instead of finishing.
    - `retryPlan` retries on a 2.5 s timer near the end, with the notice «No se pudo continuar el texto: …; reintentando…».
    - `noticeAfterContinuation` clears a stale notice.
    - `prefixEvents` scores only the events up to the cursor's first arrival at its final position, consistent with the replay. A Rust test saves an ended prefix in every error mode.
  - Parent: `withDeadline` (25 s) turns a silent provider into a failure and so into a retry.
  - Checks: vitest 546/546, check 0; cargo 606 and clippy 0 from the worker.
  - Not covered by any test: the Svelte wiring, which needs a GUI check.
- **L6 (user, verbatim, 2026-11-19):** "si no esta por ia, vamos encadenando textos no ia". `continue_text` accepts an optional `offline` flag that goes straight to the corpus (Rust test: no AI request, no usage). After one failed, empty or timed-out continuation, the frontend sets it for the rest of the text, so later pieces chain local texts with no AI wait. RED first in Rust and in the api test. Checks: vitest 547, check 0, cargo 607, clippy 0.
- **L7 (user report, root cause and fix, 2026-11-19):** the user wrote "no puede avanzar … esta ilimitado no deberia parar nunca de aparecer texto". Reproduced by the parent on the real release build under Xvfb with isolated data (target/e2e/unlimited.sh, AI off).
  - Symptom: the text never grew on screen, while the on-screen keyboard already pointed at the continuation's next char.
  - Root cause: `session` is `$state.raw` and `extend` grows `session.chars` in place, while the markup iterated `session.chars`, so it never re-rendered.
  - Fix (worker): `view` carries a copy of the chars (`view.chars`), and the prose text, CodeText and the waiting line read it.
  - Regression test `Practice.unlimited.test.ts` mounts Practice in jsdom (dev dependency `jsdom` 31.0.1, Svelte browser condition under vitest in vite.config.ts). RED: the text did not grow. GREEN after the fix.
  - Checks: vitest 548/548, check 0, build ok.
  - Re-run of the e2e test on the rebuilt binary: 320 keys on the diagnostic text, then 700 and 200 keys on the unlimited text. The text kept growing with corpus continuations and there was always text ahead.
  - The AI path is still not exercised on a GUI build (isolated data has no AI).
- **L8 (user, verbatim, 2026-11-20):** "haz lo que consideres mas optimo", after the list of what remained.
  - Agent decision: of the known limits in L4, "Repetir" repeating the whole grown text is fixed (S6). The others stay as documented: the notice-less corpus fallback with the AI off matches next_text, and backspaced keystrokes past the prefix are excluded by design (S5).
  - Incident: `node_modules` lacked `jsdom`, although it is in the lockfile; `pnpm install --frozen-lockfile` restored it.
  - T5: RED in `Practice.unlimited.test.ts` (the repeat showed the whole 600-char text), GREEN after `restart` swaps in the ended prefix. Checks: vitest 549/549, check 0. The ghost race of the repeat is not asserted by a test.
  - Still pending: the AI continuation on a GUI build, with the user's account.
