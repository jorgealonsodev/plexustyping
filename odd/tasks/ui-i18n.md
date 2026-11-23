# Interface in the app's languages

Translate the whole interface into the five languages the app already practises (es, en, pt, fr, de), detected from the computer with English as the fallback, chosen apart from the practice-text language. Branch `feat/ui-i18n` (on top of `feat/unlimited-text`). Delivery: ask-on-risk. Runners: `pnpm test` (vitest), `pnpm check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`.

## Specs

- **S1** — "haz que hacer las ui en los mismos idiomas qeu hay en la appp": every user-visible interface text exists in es, en, pt (Brasil), fr and de: labels, hints, buttons, notices, aria-label/title/placeholder, screen-reader announcements, chart captions, and backend messages the UI shows verbatim. Numbers, percentages and dates follow the interface language. Not in scope: practice corpora, AI prompts, CSV export headers (a data format), endonyms of layouts and languages (they stay in their own language).
- **S2** — "debe detectar el idioma del equipo donde se instala, y si no en ingles todo por defecto": with no explicit choice, the interface uses the first of the computer's languages (`navigator.languages`, base subtag: `pt-PT` → pt) that is one of the five; otherwise English.
- **S3** — "los idiomas de la interfaz y del testo tienen que poder elegirse de manera separada": the interface language is its own setting; the practice-text language keeps its current behaviour (default Spanish, per profile) and neither changes the other.
- **S4** — The interface-language setting lives in Settings → Appearance, with "Automatic (system language)" plus the five endonyms; it applies live, is stored as `plexustyping.uiLanguage` (absent = automatic) and is global, not per profile (assumption). `<html lang>` follows it.
- **S5** — Every catalog has every key of the Spanish one (typecheck) and the same `{placeholders}` (test). The Spanish texts stay as they are today, so the existing tests keep running in Spanish.
- **S6** — README documents the setting; the `.desktop` file gets `en`, `pt_BR`, `fr` and `de` entries.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S2–S5, S6 (README) | Core: `src/i18n` (locales, detection, `defineMessages`, plural and number helpers), setting and selector, init in `main.ts`, `<html lang>`, vitest setup in Spanish; Rust `i18n` module and `set_ui_language` command | parent (inline: the others depend on it) | done | 52966b9 |
| T2 | S1 | Practice, keyboard, metronome, code, unlock, goals, themes | writer A (parallel, disjoint surface) | done | e74cfd7 |
| T3 | S1 | Settings sections and modules, profiles | writer B (parallel) | done | 43c8b4a |
| T4 | S1 | History (modal, charts, history.ts), App.svelte | writer C (parallel) | done | f54b43b |
| T5 | S1 | Rust messages shown by the UI (src-tauri, plexustyping-ai) | writer D (parallel) | done | 6e71585 |
| T6 | S1–S6 | Integration: seam check (vitest, check, cargo, clippy, build), `.desktop`, README | parent | done | see L7 |

Route evidence: T2–T5 are independent units with disjoint files, each several hundred changed lines in five languages (writer trigger a).

## Log

- **L1** (2026-11-23) — "haz que hacer las ui en los mismos idiomas qeu hay en la appp"
- **L2** (2026-11-23) — "debe detectar el idioma del equipo donde se instala, y si no en ingles todo por defecto"
- **L3** (2026-11-23) — "los idiomas de la interfaz y del testo tienen que poder elegirse de manera separada"
- **L4** — Map: about 400–450 frontend strings (no strings module; `es-ES` hard-coded in `AiSection.svelte`, `formatPercent`, `history.ts` dates), about 60 Rust strings shown verbatim through `String(err)` and `AiFailure.message`, and about 120 test assertions on Spanish text. Decision: a dependency-free typed catalog per feature (`messages.ts`, Spanish as the key source); Rust gets a global locale set by the frontend and message functions that take the locale explicitly, so tests never mutate the global.
- **L5** — T1 done (52966b9): `src/i18n` (detection, `defineMessages`, `plural`, `formatNumber`, `expectComplete`), `UiLanguageField` first in Apariencia, `initUiLocale` before mount, `<html lang>` follows, `set_ui_language` + `src-tauri/src/i18n.rs`. Checks: vitest 38 files / 565 tests green, svelte-check 0 errors, cargo i18n tests 3 passed, clippy clean. Writers A–D launched in parallel. README "Interface language" section and `.desktop` translations written (T6, uncommitted; `desktop-file-validate` clean).
- **L6** — T2, T3, T4 done. Seam: frontend vitest 49 files / 599 tests green, svelte-check 0 errors. Fixed one cross-writer mismatch (the length hint named the English button "Finish"; the button is "End"). Note: e74cfd7 renamed `TOO_LARGE`/`TOO_MANY` to functions whose settings call sites land in 43c8b4a, so e74cfd7 alone does not typecheck; the pair does.
- **L7** — T5 done (6e71585): `src-tauri/src/messages.rs` catalog with a placeholder-parity test; `PlanMessage` and `AuthError::recognize` added to `plexustyping-ai` (additions only, Spanish output byte-identical). Left in English on purpose: invariant errors with built-in ids (`unknown layout`, `unknown code language`, `unknown practice mode`, `could not generate a practice text`) and technical details inside messages (database, IO, HTTP codes). T6 seam check: vitest 49 files / 599 tests, svelte-check 0 errors, `pnpm build` ok, `cargo test --workspace` 628 passed / 0 failed, clippy `-D warnings` clean; register consistent (German "du", French "vous"). Pending: a try-out in the real app (WebKitGTK `navigator.languages` from the system locale).
