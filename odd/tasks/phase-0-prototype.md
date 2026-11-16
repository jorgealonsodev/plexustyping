# Feature: PlexusTyping — Phase 0 (prototype)

Source of truth: `PRD-PlexusTyping.v0.1.md` (roadmap phase 0). Branch: `feat/phase-0-prototype`.
Delivery: `chain_strategy=feature-branch-chain` (~400-line slices). Forecast: ~2,000 authored lines.

## Specs

- **S1 — Phase 0 scope.** PRD §8: "**0 · Prototipo** | Pantalla de práctica, medición por carácter, QWERTY ES, corpus local, PPM y precisión".
- **S2 — Per-character measurement.** PRD §4.2: "Por cada carácter esperado se guarda: acierto o fallo, qué se pulsó en su lugar, y tiempo desde la tecla anterior."
- **S3 — Error modes.** PRD RF10: "Modos de error: parar hasta corregir, permitir seguir, o borrar con Retroceso." User: "Los 3, por defecto 'permitir seguir'".
- **S4 — Speed and accuracy.** PRD RF5: "**En vivo (activable):** PPM, pulsaciones por minuto, precisión y tiempo." and "**Al terminar:** PPM brutas y netas, precisión". Phase 0 covers these items only (S1: "PPM y precisión").
- **S5 — QWERTY ES layout.** PRD RF7: "Layouts en ficheros TOML: tecla física → carácter por capa (normal, Shift, AltGr) y teclas muertas." and "Composición de teclas muertas (´ + a = á) contada como un carácter." File example in PRD: `layouts/es-qwerty.toml` with `name = "Español QWERTY"`, `dead_keys = ["´", "`", "^", "¨"]`, `[keys.KeyQ] normal = "q" shift = "Q"`, `[keys.Semicolon] normal = "ñ" shift = "Ñ"`, `[keys.Digit2] normal = "2" shift = "\"" altgr = "@"`.
- **S6 — Local corpus.** PRD RF8: "Por idioma: listas de palabras por frecuencia, frases y textos de diagnóstico, con licencia libre." / "Generador sin IA: arma frases eligiendo palabras que contienen los caracteres débiles." / "Se usa sin clave, sin conexión o cuando la IA falla." PRD §4: "**Texto de diagnóstico.** Texto fijo del corpus (~300 caracteres) que cubre todas las letras, tildes, ñ, signos y mayúsculas del idioma." User: word list from "hermitdave/FrequencyWords" (CC BY-SA 4.0, ~5,000 most frequent words, no profanity or proper names, with attribution); diagnostic text and sentences are original.
- **S7 — Stack and identity.** PRD: Rust + Tauri 2; crates `plexustyping-core`, `plexustyping-layouts`, `plexustyping-corpus`, `plexustyping-app`; "`plexustyping` en minúsculas (crates, binario, carpeta de datos, bundle id `com.plexustyping.app`)". User: frontend "Svelte 5 + Vite", package manager "pnpm", license "MIT OR Apache-2.0".
- **S9 — Themes from PlexusTerm.** User: "añade los mismos temas que plexusterm /mnt/1tb/Repos/Personal/20261019_plexusterm/20261019_plexusterm_git/PlexusTerm". PlexusTerm ships 57 schemes: its own "PlexusTerm" palette plus 56 imported from iTerm2-Color-Schemes (MIT, commit `21f281d0834846eef984f75774638e7d16bac068`) via `scripts/import-terminal-themes.py`, default `catppuccin-mocha`, credited in `THIRD_PARTY_NOTICES.md`. PRD RF9: "Cada tema define: fondo, texto pendiente, texto acertado, error, cursor, teclado" and "Cambio de tema en caliente".
- **S8 — Non-functional (phase 0 subset).** PRD §6: "< 16 ms entre tecla y pintado"; "CSP estricta en Tauri"; risk table: "Capturar en el frontend con `KeyboardEvent.code` + `key`".

## Tasks

| ID | Specs | Route | Status | Commit |
| --- | --- | --- | --- | --- |
| T1 | S7, S8 | Scaffold Cargo workspace, Tauri 2 app, Svelte 5 frontend, licenses, README — inline | done | T1 commit |
| T2 | S2, S3, S4 | `plexustyping-core`: input replay, 3 error modes, per-keystroke records, stats — writer A | pending | — |
| T3 | S5 | `plexustyping-layouts`: TOML loader, `es-qwerty.toml`, dead-key composition, allowed chars — writer B | pending | — |
| T4 | S6 | `plexustyping-corpus`: FrequencyWords import + attribution, diagnostic text, sentences, offline generator — writer C | pending | — |
| T7 | S9 | Themes: same 57 schemes as PlexusTerm, role mapping, hot switching, notices — inline | pending | — |
| T5 | S1, S3, S4, S8, S9 | App integration: Tauri commands, practice screen, live stats, results, error-mode selector — inline | pending | — |
| T6 | S1–S8 | Close: full suite, builds, review assessment, report | pending | — |

## Log

- **L1 (user request, verbatim):** "/mnt/1tb/Repos/Personal/20261116_plexustyping/20261116_plexustyping_git/plexustyping/PRD-PlexusTyping.v0.1.md empieza a desarrollar" — follow-up: "hazme las preguntas sobre lo que te falte".
- **L2 (user decisions):** PRD source "v0.1 (el que te pasé)"; scope "Fase 0 · Prototipo (Recomendado)"; frontend "Svelte 5 + Vite (Recomendado)"; error modes "Los 3, por defecto 'permitir seguir'"; corpus "Lista de frecuencias libre externa" → "hermitdave/FrequencyWords (Recomendado)"; license "MIT OR Apache-2.0"; package manager "pnpm (Recomendado)"; delivery "Cadena sobre rama de feature".
- **L3 (environment):** cargo/rustc 1.98.1, node 24.15, pnpm 11.12; webkit2gtk-4.1 2.52, libsoup-3.0 3.4, gtk3 present. No global Tauri CLI → `@tauri-apps/cli` as a devDependency.
- **L4 (architecture decision):** The frontend captures `keydown` (`code` + `key`, ignoring `key === "Dead"` so the composed character counts once), keeps a TS display state for < 16 ms painting, and timestamps raw events with `performance.now()`. On finish it sends the raw event log to Rust; `plexustyping-core::replay` is the authoritative source of records and stats. Both apply the same error-mode rules and are tested with the same fixtures.
- **L5 (scope decision):** Without the adaptive engine (phase 1), the next offline text targets the characters failed in the previous text, passed to the corpus generator.
- **L6 (licensing):** FrequencyWords content is CC BY-SA 4.0 (README: "CC-by-sa-4.0 for content"), derived from OpenSubtitles 2018. The filtered list ships with its own `LICENSE`/attribution and stays CC BY-SA 4.0; app code is MIT OR Apache-2.0.
- **L7 (user request, verbatim):** "añade los mismos temas que plexusterm /mnt/1tb/Repos/Personal/20261019_plexusterm/20261019_plexusterm_git/PlexusTerm" → S9, T7 (pulls part of RF9 into phase 0).
- **L8 (user request, verbatim):** "Puedes crear un tablero en trello para seguir las fases" → Trello board tracking the PRD phases (outside the repo).
- **L9 (T1 evidence):** `cargo build --workspace` ok (tauri 2.11.6); `pnpm check` 0 errors; `pnpm build` ok; `pnpm test` passes with no tests. TypeScript pinned to ~6.0.3 (svelte-check peer `^5 || ^6`); tsconfig follows the create-vite svelte-ts template. Prod CSP has `style-src 'self'` without `unsafe-inline`: avoid Svelte transitions that inject `<style>`.
- **L10 (Trello):** Board "PlexusTyping — Roadmap" https://trello.com/b/yTLEs12d/plexustyping-roadmap (private). Lists: Pendiente, En curso, En revisión, Hecho. One card per phase with checklist "Alcance"; phase 0 card https://trello.com/c/A9h89nMm lists T1–T7 (T1 checked). Update its checklist on each task transition.
- **L11 (RDD):** T1 commit `214a894` assessment unavailable: "untracked files require an explicit declaration" (the two PRD files stay untracked; not user-committed) and the root commit has no base.
