# Phase 3 · Languages

Fifth block of phase 3: the languages the agent recommended and the user accepted (L1), each with its native layout. Everything not quoted in the Log is the agent's proposal and an assumption (the PRD is not on this machine).
Branch `feat/phase-3-languages`. Runners: `cargo test --workspace`, `pnpm test` (vitest), `pnpm check`, `pnpm build`.

## Specs

- **S1 — Languages.** Prose practice in five languages: `es` "Español" (default, existing), `en` "English", `pt` "Português (Brasil)", `fr` "Français", `de` "Deutsch". "Ajustes → Práctica" gains a selector "Idioma"; the choice is stored locally (`plexustyping.language`); a missing or unknown stored value loads as `es`. Changing it loads a new text. Code practice is unaffected.
- **S2 — Corpora.** Each new language has, in `crates/plexustyping-corpus/data/<lang>/`, the same files as Spanish: `words.txt` (5,000 lowercase words in frequency order, adapted from hermitdave/FrequencyWords 2018 at the commit Spanish uses; Portuguese from `pt_br`), `sentences.txt` (at least 40 original sentences), `diagnostic.txt` (one original paragraph, 280–340 chars, covering the language's letters and accents) and `LICENSE.md`. `scripts/import-frequencywords.py` builds any of them from a language argument. `THIRD_PARTY_NOTICES.md` lists each word list.
- **S3 — Layouts.** Three new built-in layouts, the native ones of the new languages: `pt-abnt2` "Português (Brasil)" (ABNT2: dead keys `´ ` ^ ~ ¨`, `ç` key, extra `/ ?` key `IntlRo` beside the right Shift, drawn on screen), `fr-azerty` "Français (France)" (AZERTY: dead `^ ¨`, digits on Shift), `de-qwertz` "Deutsch (Deutschland)" (QWERTZ: `ß ä ö ü` keys, dead `^ ´ ``). They appear in "Distribución" after the existing three.
- **S4 — Texts.** Corpus and AI texts are in the chosen language and only contain characters the chosen layout can type (existing rule). The diagnostic text of a new profile is the chosen language's when the layout can type it.
- **S5 — Sessions.** A saved session records the language it was typed in (history and the CSV column `idioma`), not the profile's; an unknown language is an error, like an unknown layout.
- **S6 — Docs.** README describes the languages, the selector, the new layouts and the limits (the statistics model is shared across languages).

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1, S4, S5 | Language plumbing: corpus registry by language id, `language` per call in `score_session` (and `first_text`/`next_text` already), frontend setting "Idioma", text reload, generation key | worker A (`src-tauri/**`, `src/**` except keyboard, `crates/plexustyping-corpus/src/**`, `crates/plexustyping-corpus/tests/**`) | pending | |
| T2 | S2 | Import script by language + `en`, `pt`, `fr`, `de` data + notices | worker B (`crates/plexustyping-corpus/data/**`, `scripts/import-frequencywords.py`, `THIRD_PARTY_NOTICES.md`) | pending | |
| T3 | S3 | `pt-abnt2`, `fr-azerty`, `de-qwertz` + `IntlRo` geometry and finger | worker C (`layouts/**`, `crates/plexustyping-layouts/**`, `src/features/keyboard/**`) | pending | |
| T4 | S1, S3, S6 | Integrate: register the new corpora and layouts in the frontend lists and tests, README, full checks | inline | pending | |

## Log

- **L1 (user, verbatim, 2026-11-17):** "mete las de tu recomnedacion" — answer to the recommendation "Inglés … Portugués con ABNT2 … Francés (AZERTY) y alemán (QWERTZ)".
- **L2 (exploration):** corpus crate embeds only `SPANISH` (`corpus()`/`languages()` hardcoded); `plan_text` restricts chars by `layout.allowed_chars()`; the AI prompt is already language-parametrised (`language_name` knows en/pt/fr/de); `sessions.language` is written from `profile.language` (`commands.rs:549`), so every session is tagged `es` today; dead keys `´ ` ^ ~ ¨` are supported by NFC composition; ABNT2's `IntlRo` key is in no geometry row.
- **L3 (decision):** the language is a local setting like the layout; the profile row is not changed; the statistics model stays one per profile (shared across languages).
