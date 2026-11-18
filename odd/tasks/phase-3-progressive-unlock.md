# Phase 3 · Progressive unlock

Beginner mode (PRD §4.3): practice starts with the home row and unlocks one key at a time. Last block of phase 3 (kids mode was dropped on 2026-11-18). No schema migration: per-profile state lives in the existing SQLite `settings` table.
Branch `feat/phase-3-progressive-unlock`. Backend (T1) and frontend (T2) are written in parallel against the contract in S7. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Mode.** "Desbloqueo progresivo (modo principiante, opcional)". Off by default, per profile, in Ajustes → Práctica: "Desbloqueo progresivo" ("Empieza por la fila base y añade una tecla cada vez que dominas las activas") and "Velocidad objetivo" (20, 30, 40, 50 PPM; default 30). Prose only: words and code modes ignore it. Progress is kept per profile, language and layout, and survives turning the mode off and on.
- **S2 — Start.** PRD: "Arranque: solo están activas las teclas de la fila base." The home row = the letters the layout's `normal` layer puts on KeyA…KeyL and Semicolon (Spanish: a s d f g h j k l ñ). Space is always active.
- **S3 — Mastered key.** PRD: "se considera dominada cuando cumple las tres condiciones: `n ≥ 30` muestras; `ema_error ≤ 0,05`; latencia mediana dentro del objetivo de velocidad (por defecto 30 PPM ≈ 400 ms por pulsación, configurable)." Samples and ema_error come from the profile's prose per-character statistics; the median latency is the median `ms_since_previous` of the profile's correct prose keystrokes on that character (the last 200 at most). Target ms per keystroke = 12 000 / PPM (30 PPM → 400 ms).
- **S4 — Unlock.** PRD: "se añade una tecla cada vez, cuando todas las activas están dominadas o tras 3 sesiones sin progreso." Checked after each saved prose session in this mode; "progress" = at least one active key became mastered in that session; the counter resets on every unlock. Order, PRD: "primero la fila base y después por frecuencia del idioma. En español: f j d k s l a ñ → e o n r i t u c m p…" — after the home row, the remaining lowercase letters the layout can type, by their frequency in the language corpus. Uppercase of an active letter is active too (Shift is always available); digits and punctuation stay locked until every letter is unlocked, then all of them unlock together.
- **S5 — Texts.** PRD: "Con menos de 15 letras desbloqueadas no se pueden formar frases con sentido. Se usan palabras reales del corpus y pseudopalabras pronunciables, sin IA. La IA entra a partir de 15 letras." Below 15 letters: lowercase words separated by spaces, only active letters; real words from the language corpus first, completed with pronounceable pseudowords (alternating consonant and vowel groups built only from active letters), the new key and weak keys weighted up, the usual length setting. From 15 letters: the normal generation (AI when on, corpus otherwise) restricted to the active characters.
- **S6 — UI.** The on-screen keyboard dims locked keys. Above the text: "Teclas: 9/27 · siguiente: e". The result of the session that unlocks a key: "Nueva tecla desbloqueada: e". All strings in Spanish as written here.
- **S7 — Contract.** Commands (camelCase JSON), all for the active profile: `unlock_status(language, layout) -> UnlockStatus` with `{ enabled, targetWpm, active: string[], mastered: string[], next: string | null, total, stalledSessions }` (`total` = letters of the full order); `unlock_set(language, layout, enabled?, targetWpm?) -> UnlockStatus` (refuses other target values with "Velocidad objetivo no válida"). `score_session`'s result gains `unlocked: string | null` (the key unlocked by this session). `first_text`/`next_text` apply S5 when the mode is on for that language and layout. Temporary mode: state in memory.
- **S8 — Docs.** README: the mode, the unlock rule, the texts below 15 letters.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S5, S7 | Backend: order per language/layout, mastery from stats + keystroke medians, unlock after scoring, word/pseudoword generator, text plan integration, commands, tests | writer A (parallel) | pending | |
| T2 | S1, S6–S8 | Frontend: settings controls, API bindings, dimmed keys, progress line, unlock notice, vitest, README | writer B (parallel) | pending | |
| T3 | S1–S8 | Integration checks, GUI check in an isolated dev app, review, merge, push, Trello | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-18):** "continua con lo que consideres mas optimo" — after the agent recommended progressive unlock to close phase 3. Everything not quoted from the PRD (per-profile storage, target options, punctuation rule, pseudoword shape, UI strings) is the agent's design.
- **L2 (PRD §4.3, verbatim):** quoted in S2–S5.
