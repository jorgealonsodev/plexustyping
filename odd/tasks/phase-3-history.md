# Phase 3 · History and CSV export

First block of phase 3 ("histórico y exportar CSV"). The PRD is not available on this machine; the spec below was proposed by the agent and approved by the user (L2).
Branch `feat/phase-3-history`. Backend and frontend are written in parallel against the contract in S6.

## Specs

- **S1 — History entry point.** A "Historial" button in the header, next to the settings gear (clock icon, `aria-label="Historial"`), and the shortcut Ctrl+H, open a history window built like the settings modal (native `<dialog>`, Escape closes it without restarting the text being typed).
- **S2 — Filter.** The window has a filter "Todo / Prosa / Código"; summary, chart, table and export follow the active filter.
- **S3 — Summary.** "resumen: número de sesiones, PPM netas media y mejor, y precisión media." Empty history shows "Aún no hay sesiones guardadas." and no chart, table rows or export.
- **S4 — Evolution chart.** "Gráfica de evolución de las PPM netas y la precisión en las últimas 50 sesiones, dibujada en SVG sin librerías." Oldest left, newest right. No `style` attributes (CSP `style-src 'self'`): classes and SVG presentation attributes only.
- **S5 — Session table.** "Tabla de sesiones: fecha, modo y lenguaje, PPM netas, precisión, tiempo y BPM, de la más reciente a la más antigua." Spanish formatting as in `src/features/practice/format.ts` (precision `90,5 %`, time `m:ss`, PPM integer); BPM empty when null; the table scrolls inside the window at 720×480.
- **S6 — Backend contract.**
  - Store: a sessions query with an optional mode filter (`"prose"` | `"code"`) and an optional limit, newest first, returning the existing `SessionSummary` (camelCase: `id, startedAtMs, mode, codeLanguage, language, layout, durationMs, grossWpm, netWpm, accuracy, keystrokes, bpm`).
  - Tauri command `session_history(mode: Option<String>, limit: Option<u32>) -> Vec<SessionSummary>` for the active profile; unknown mode → error.
  - Tauri command `export_sessions_csv(mode: Option<String>, tz_offset_min: i32) -> { path: String, rows: u32 }` (camelCase result); `tz_offset_min` = minutes east of UTC from the frontend (`-new Date().getTimezoneOffset()`).
  - In temporary mode (no storage) both commands behave as an empty history.
- **S7 — CSV export.** "Exportar CSV" exports all sessions of the active filter. User decisions (verbatim): "Directo en Descargas" and "Estándar".
  - File: the user's Downloads directory, name `plexustyping-sesiones-AAAAMMDD.csv` (local date); if the name exists, `-2`, `-3`… is appended; never overwrite.
  - Format: UTF-8 without BOM, separator `,`, decimal point `.`, RFC 4180 quoting, `\n` line ends, header row exactly `fecha,modo,lenguaje_codigo,idioma,layout,duracion_s,ppm_brutas,ppm_netas,precision,pulsaciones,bpm`.
  - Values: `fecha` ISO 8601 local time with offset (`2026-11-17T09:30:05+01:00`); `modo` `prose`|`code`; `lenguaje_codigo` empty for prose; `duracion_s`, `ppm_brutas`, `ppm_netas`, `precision` (percent 0–100) with one decimal; `pulsaciones` integer; `bpm` empty when null. Rows newest first.
  - No sessions → no file, error "No hay sesiones que exportar.". Success shows "Exportado: <ruta> (N sesiones)" in the window ("(1 sesión)" for one, L4).
- **S8 — Docs.** README lists the history window, Ctrl+H and the CSV export.

## Tasks

| ID | Specs | Task | Status | Commit |
|----|-------|------|--------|--------|
| T1 | S6, S7 | Backend: store query + `session_history` + `export_sessions_csv` (CSV writer, local ISO dates, unique file name) with tests — writer A (parallel) | pending | — |
| T2 | S1–S5, S7 (UI), S8 | Frontend: header button + Ctrl+H, HistoryModal (filter, summary, SVG chart, table, export button and notices), API bindings, vitest, README — writer B (parallel) | pending | — |
| T3 | all | Close: suite, real-app check, review, main, Trello | pending | — |

## Log

- **L1 (user, verbatim):** "haz lo que consideres mas optimo" → chose the first phase-3 block: "Histórico y exportar CSV". Context: PRD not on this machine ("no esta ninguna rama en remoto el prd?" — it is untracked on the work clone); Trello card "Fase 3 · Completo" only says "histórico y exportar CSV".
- **L2 (user decisions, verbatim):** "¿Dónde guarda el CSV? — Directo en Descargas"; "¿Qué formato de CSV? — Estándar"; "¿Apruebas el resto de la especificación (botón Historial con resumen, gráfica, tabla y filtro)? — Sí, adelante".
- **L3 (exploration):** sessions table already stores `started_at_ms, mode, code_language, language, layout, duration_ms, gross_wpm, net_wpm, accuracy, keystrokes, bpm`; `recent_sessions` exists in store and Tauri but has no filter and no frontend caller; no router (App renders header + Practice + SettingsModal); no dialog/fs plugin, capabilities only `core:default`; no chart lib; CSP `style-src 'self'`; schema v2.
- **L4 (decision):** the parent changed the success notice to the singular "(1 sesión)" for one row (the S7 wording read "(1 sesiones)"); reported to the user.
