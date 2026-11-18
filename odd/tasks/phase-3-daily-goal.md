# Phase 3 · Daily goal and streak

PRD RF10: "Meta diaria en minutos o en textos, con racha de días." Per profile (the `goals` TEXT column of `profiles`, unused until now; no schema migration).
Branch `feat/phase-3-daily-goal`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Goal per profile.** A goal is off (default), minutes or texts. Allowed amounts: minutes 5, 10, 15, 20, 30, 45, 60, 90, 120; texts 1, 3, 5, 10, 15, 20, 30. Stored as JSON in the active profile's `goals` column (`{"daily":{"kind":"minutes","amount":20}}`, `null` or absent = off; an unreadable value counts as off and is never overwritten except by a new goal). Other profiles keep their own goal.
- **S2 — Today's progress.** Counts the active profile's saved sessions (prose and code) started on the local day: minutes = sum of `duration_ms` (shown rounded down to whole minutes, met when the exact sum reaches the amount), texts = number of sessions. The local day comes from `tz_offset_min` sent by the frontend (`-new Date().getTimezoneOffset()`, minutes east of UTC).
- **S3 — Streak.** Consecutive local days, ending today when today's goal is met, otherwise ending yesterday, on which the current goal was met (the current goal is applied to past days). No goal → no streak. A day without sessions breaks it.
- **S4 — Commands** (camelCase JSON). `goal_set(kind: "off" | "minutes" | "texts", amount: Option<u32>) -> GoalStatus` for the active profile; anything outside S1 is refused with "Meta diaria no válida" and changes nothing. `goal_status(tz_offset_min: i32) -> GoalStatus` = `{ goal: null | { kind, amount }, today: number, met: bool, streak: number }` (`today` in whole minutes or texts). Temporary mode: the goal lives in memory, progress from the in-memory sessions.
- **S5 — UI.** Ajustes → Perfiles gains "Meta diaria" for the active profile: "Desactivada", "Minutos al día", "Textos al día" and the amount selector with the S1 values. The header, next to the profile name, shows the progress when a goal is set: "Hoy 12/20 min · racha 3" (texts: "Hoy 2/5 textos · racha 3"; streak 0 omits "· racha 0"); met → "Meta cumplida · racha 4". It refreshes after each saved session, on profile switch and on goal change. The result screen of the session that meets the goal shows once "¡Meta diaria cumplida!".
- **S6 — Docs.** README: daily goal and streak, per profile.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S6 | Backend (goal parse/store, progress and streak with tz offset, commands, tests) + frontend (API, Perfiles control, header progress, result notice, vitest) + README | worker | done | see L3 |
| T2 | S1–S6 | Integration checks + structural review; merge, push, Trello | parent | done | see L3 |

## Log

- **L1 (user, verbatim, 2026-11-18):** "continua con lo que consideres mas optimo" — after local profiles, the agent picked the daily goal (smallest remaining RF10 item) and designed S1–S6 (amount lists, streak rule, texts) as its own assumptions.
- **L2 (PRD, verbatim):** "Meta diaria en minutos o en textos, con racha de días."
- **L3 (evidence, 2026-11-18):** T1 by one worker, test-first (RED: compile failures and 14 failing stubs, then GREEN): store `DailyGoal`/`GoalKind`/`MINUTE_GOALS`/`TEXT_GOALS`, `daily_goal`/`set_daily_goal` (keeps other keys of `goals`, `{"daily":null}` = off), `src-tauri/src/goals.rs` (`progress` by local day, streak loop), commands `goal_set(kind, amount, tz_offset_min?)` and `goal_status`, `src/features/goals/*`, Perfiles control, header text, `crossedGoal` notice, README. Assumptions: `goal_set` takes an optional `tz_offset_min` (default 0) to return local progress; "off" with an amount is refused; offsets beyond ±18 h give "Zona horaria no válida: N"; default amounts 20 min / 5 texts when switching kind; the notice is skipped when the previous status is unknown. Parent checks: cargo 419, vitest 242, clippy 0, check 0, build ok; structural readback of the local-day and streak code. No component test of the Results notice; not tried in the running app.
