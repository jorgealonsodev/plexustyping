# Feature: PlexusTyping — Phase 1A (adaptive engine + SQLite)

Source: `PRD-PlexusTyping.v0.1.md` phase 1 (MVP), first block. Branch: `feat/phase-1-adaptive` (stacked on `feat/phase-0-prototype`).
Delivery: `chain_strategy=feature-branch-chain` (cached from phase 0). Forecast: ~1,600 authored lines. Phase 1B (AI providers, keychain, metronome) follows separately.

## Specs

- **S1 — Weakness score.** PRD RF1: "Puntuación por carácter y bigrama: error (peso 0,7) + lentitud relativa a la media del usuario (peso 0,3), media móvil exponencial con α = 0,2." PRD §4.3: "Para cada carácter y cada bigrama («qu», «ción», «rr») se calcula una media móvil exponencial de error y de lentitud."
- **S2 — Target selection.** PRD §4.4: "Se eligen los 3–5 caracteres o bigramas peor puntuados, con algo de aleatoriedad para no repetir siempre los mismos." RF1: "Ajustes: número de objetivos por texto (3–5), densidad (2×–5×), longitud (50–600 caracteres)."
- **S3 — Confusions.** RF1: "Las confusiones frecuentes (pulsar «m» en vez de «n») se guardan como pares y también se entrenan."
- **S4 — Heatmap.** RF1: "Mapa de calor sobre el teclado con la puntuación de cada tecla." RF4: "Superposición opcional del mapa de calor."
- **S5 — SQLite persistence.** PRD §7 data model: `profiles` (id, nombre, idioma, layout, tema, metas), `sessions` (id, profile_id, inicio, modo, lenguaje_código, PPM, precisión, BPM), `keystrokes` (session_id, índice, esperado, pulsado, correcto, ms_desde_anterior, desvío_metrónomo_ms), `char_stats` (profile_id, modo, carácter_o_bigrama, ema_error, ema_lentitud, muestras), `confusions` (profile_id, esperado, pulsado, veces), `text_cache`, `settings` (clave, valor). §6: "Datos solo en local". Phase 1A creates profiles/sessions/keystrokes/char_stats/confusions/settings; `text_cache` arrives with the AI providers (1B).
- **S6 — Adaptive loop.** PRD §4: "El ciclo es: escribir → medir → recalcular debilidades → pedir un texto nuevo que las ataque." / "Generador sin IA: arma frases eligiendo palabras que contienen los caracteres débiles."

## Tasks

| ID | Specs | Route | Status | Commit |
| --- | --- | --- | --- | --- |
| T1 | S1, S2, S3 | `plexustyping-core::adaptive`: `WeaknessModel` (EMA, scores, confusions, seeded target selection) — writer A (parallel) | done | 79c7714 |
| T2 | S5 | New crate `plexustyping-store` (rusqlite, migrations, profile, sessions, keystrokes, stats rows, settings) — writer B (parallel) | done | 84e652f |
| T3 | S2, S4, S6 | Integration: corpus string targets, app state + commands, adaptive next text, heatmap overlay, settings — writer C | done | 46fe5e0 |
| T4 | all | Close: suite, real-app check, review assessment, Trello | done | e52071d, 1bd7612 |

## Log

- **L1 (user, verbatim):** "continua con lo que consideres mas optimo".
- **L2 (decision):** Phase 1 split. 1A = adaptive engine + SQLite + heatmap (core value, offline, prerequisite of the MVP exit criterion "un usuario de prueba reduce un 30 % el error en sus 5 caracteres peores en 5 sesiones"). 1B = 4 AI providers + keychain (need user API keys), metronome (needs ALSA dev headers: `pkg-config alsa` not found → `sudo apt install libasound2-dev`).
- **L3 (decisions, adaptive rules):** slowness sample = clamp(ms/mean − 1, 0, 1) (twice the user's mean interval → 1), intervals > 2,000 ms are pauses and are skipped for slowness and the mean; mean interval is itself an EMA (α 0.2); first sample initializes an EMA; candidates need ≥ 3 samples; bigrams = two consecutive expected chars without whitespace. Corpus targets become strings (chars or bigrams).
- **L4 (decisions, storage):** `rusqlite` with `bundled` SQLite (no system lib), schema versioned with `PRAGMA user_version`, DB at Tauri `app_data_dir()/plexustyping.sqlite3`; one default profile "Principal" (es, es-qwerty). Store crate exposes plain row structs so it does not depend on the adaptive model types (parallel writers).
- **L5 (T1/T2 evidence):** parallel writers. T1 `79c7714`: `adaptive.rs`, 22 tests; serde `{units, confusions, meanIntervalMs}`; `from_parts` keeps the last duplicate; score clamped to 0..=1. T2 `84e652f`: `plexustyping-store`, rusqlite 0.40 bundled (0.41+ need Rust 1.89; checked with `cargo +1.85`), schema v1, 12 tests; additive: `Store: Debug`, `StoreError::{Io, InvalidData}`, `session_keystrokes` → `NotFound` for unknown ids, `load_stats` sorted, error mode stored kebab-case. Seam: `cargo test --workspace` 82 passed, clippy/fmt clean.
- **L6 (T3 + T4 evidence):** T3 `46fe5e0` (corpus string targets, AppState + store, first_text/next_text/score_session persistence, weaknesses, heatmap; 21 app tests, 84 vitest). Real-app check (xdotool, 1000×760): diagnostic first, full text saved to `~/.local/share/com.plexustyping.app/plexustyping.sqlite3`, "Tus puntos débiles" shown, next text "Texto adaptativo · Practicando: b · s · es", heatmap + highlight OK. Test DB (only my filler session) deleted afterwards. Header wrapped to 3 rows at 720×480 → `e52071d` "Ajustes" popover; verified one row + panel overlay in the real app.
- **L7 (risk):** native assess on `49f94ab..e52071d` medium/under_budget; escalated item 1 (persisted schema v1 + migrations) → independent verify: S5 met, S6 met; non-blocking F1 (session and stats saves are separate transactions), F2 (WAL set before migrations modifies a foreign v0 file header), F3a (no per-session language/layout), F3b (no busy_timeout), F3d (no NFC normalization of units). Fixing F1, F2, F3a, F3b now (schema v1 unreleased, so v1 itself is amended, no v2 migration).
- **L8 (close):** `1bd7612` fixes F1 (`Store::save_session_with_stats`, one transaction), F2 (WAL after migrations, file DBs only), F3a (`sessions.language/layout`), F3b (busy timeout 2 s); 105 Rust tests, clippy/fmt clean, svelte-check 0. Local v1 DB from the header check (0 sessions) deleted since v1 was amended. Deferred: F3d unit NFC normalization; R3-003 newline typing; component tests. Not done: native review (assess not due; independent verify ran instead under escalated item 1).
