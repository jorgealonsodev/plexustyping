# Phase 3 · History charts by day and by character

PRD RF5 "Histórico: … gráficas por día y por carácter en la fase 3". Adds two views to the existing history window (odd/tasks/phase-3-history.md); no schema change, read-only over existing data of the active profile.
Branch `feat/phase-3-history-charts`. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Views.** The history window gains a view switch "Sesiones / Por día / Por carácter" (the current content is "Sesiones", the default). The "Todo / Prosa / Código" filter applies to all three. Export CSV stays as it is (sessions).
- **S2 — By day.** The last 30 local days that have sessions (local day from `tz_offset_min`, as in the daily goal), oldest left: an SVG chart of the day's mean net PPM (line) and practised minutes (bars), and below it a table "Día · Sesiones · Minutos · PPM netas · Precisión", newest first. Day mean PPM and accuracy are weighted by session duration. Spanish formatting as the session table (date `dd/mm/aaaa`, precision `90,5 %`, PPM integer, minutes rounded down).
- **S3 — By character.** From the active profile's per-character statistics (the same data that drives weaknesses and the heatmap) for the filter's mode ("Todo" merges prose and code by samples): a table of characters with at least 5 samples, "Carácter · Muestras · Error · Lentitud", worst first (by error, then slowness), and an SVG bar chart of the error of the 20 worst. Space shows as "espacio", Enter as "Intro", Tab as "Tab". Error as a percentage `4,2 %`; slowness as the stored relative value with one decimal.
- **S4 — Empty states.** No sessions: the existing "Aún no hay sesiones guardadas." in every view. "Por carácter" with no character at 5 samples: "Aún no hay bastantes pulsaciones por carácter.".
- **S5 — Backend contract.** `history_by_day(mode: Option<String>, tz_offset_min: i32, days: Option<u32>) -> DayRow[]` (`{ day: "YYYY-MM-DD", sessions, minutes, netWpm, accuracy }`, newest first, `days` default 30, max 366); `history_by_char(mode: Option<String>) -> CharRow[]` (`{ unit, samples, error, slowness }`, worst first, `samples >= 5`). Unknown mode and out-of-range offset are refused with the existing history messages. Temporary mode: empty.
- **S6 — Rules.** SVG without libraries and without `style` attributes (CSP), readable at 720×480 inside the window, accessible titles (`<title>`) on the charts. README history section updated.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S6 | Backend queries + commands + tests; frontend view switch, day and character charts and tables, vitest for the pure formatting/scaling, README | worker | pending | |
| T2 | S1–S6 | Integration checks, merge, push, Trello | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-18):** "continua con lo que consideres mas optimo" — after the daily goal, the agent picked the history charts (smallest remaining phase 3 item, read-only) and designed S1–S6.
- **L2 (PRD, verbatim):** "gráficas por día y por carácter en la fase 3;"
