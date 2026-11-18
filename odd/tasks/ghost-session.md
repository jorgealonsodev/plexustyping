# Ghost session

When a text is typed again, a ghost of the profile's fastest earlier session with exactly that text races along, like the ghost car of a lap record. No schema change: `sessions.text` and the per-keystroke timings in `keystrokes` are already stored.
Branch `feat/ghost-session`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Which ghost.** "Tu mejor sesión con ese texto": among the active profile's saved sessions whose `text` is exactly the text being typed (same mode; for code, same code language), the one with the highest net PPM (ties: the earliest). None → no ghost. The session being typed never races itself: a record set in this attempt becomes the ghost of the next attempt.
- **S2 — Ghost timeline.** From the record session's keystrokes, the elapsed time (ms from its first keystroke) at which its cursor first moved past each text position; positions it never reached have no time. Built in the backend from stored data; it works for every error mode (a position counts as passed when the record's cursor advanced beyond it).
- **S3 — When.** Active whenever a record exists for the current text (in practice: "Repetir" on the results screen, the fixed diagnostic text, or any text that comes back). Ajustes → Práctica gains "Sesión fantasma" ("Corre contra tu mejor sesión con el mismo texto"), on by default, stored locally like the other practice settings.
- **S4 — Look.** "Cursor fantasma + diferencia en vivo": a second, translucent cursor moves through the text at the record's pace, starting with the user's first keystroke; next to the live metrics a delta "+1,2 s" (behind the record) or "−0,8 s" (ahead), comparing the user's elapsed time with the record's time at the same position (one decimal, Spanish comma, sign always shown, "0,0 s" when equal). Before the first keystroke the ghost waits at the start. When the ghost reaches the end it stays there. No `style` attributes (CSP): position via classes on the text's character spans.
- **S5 — Result.** The results screen of a raced text shows "Récord anterior: 52 PPM" and, when beaten, "¡Nuevo récord para este texto!".
- **S6 — Contract.** `ghost_for_text(text: String, mode) -> null | { sessionId, netWpm, times: (number | null)[] }` for the active profile (`times[i]` = ms at which position `i` was passed). Temporary mode: from in-memory sessions. Docs: README practice section.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S6 | Backend query + timeline + command + tests; frontend setting, ghost cursor, live delta, result lines, vitest for the pure timeline/delta functions; README | worker | pending | |
| T2 | S1–S6 | Integration checks, GUI check in an isolated dev app, review, merge, push, Trello | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-18):** "Estaba pensando añadir una sesion fantasma para el mismo texto de la sesion mas rapida registrada. Como en los juegos de coches cuando puedes ver el coche fantasma del que hizo el record"
- **L2 (user decisions, 2026-11-18):** asked which session, how it looks and when it is on; chose "Tu mejor sesión con ese texto (Recomendado)", "Cursor fantasma + diferencia en vivo (Recomendado)" and "Siempre que haya récord, con interruptor (Recomendado)".
