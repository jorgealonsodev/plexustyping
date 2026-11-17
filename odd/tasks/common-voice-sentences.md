# Common Voice sentences

More real sentences for the offline corpus of every language, from Mozilla Common Voice (CC0).
Branch `feat/common-voice-sentences` (from `fix/ai-target-density`). Runners: `cargo test --workspace`, `pnpm test`.

## Specs

- **S1 — Source.** Sentences come from the Common Voice text corpus in its repository (`common-voice/common-voice`, `server/data/<lang>/*.txt`, CC0 1.0), for `es`, `en`, `pt`, `fr`, `de`, pinned to one commit recorded in each output file header (like the FrequencyWords lists).
- **S2 — Filter.** Kept sentences: 40–120 chars after trimming; only the letters of the language (the same alphabets as the word lists) plus space and `. , ; : ? !` (and `¿ ¡` for Spanish); every char typeable on the language's native layout (`es-qwerty`, `us-qwerty`, `pt-abnt2`, `fr-azerty`, `de-qwertz`); no digits, quotes, parentheses, hyphens, apostrophes or abbreviations with dots inside words; start with a capital letter and end with `.`, `?` or `!`; no duplicates; no profanity (the word-list blocklists). Up to 2,000 per language, a deterministic selection (stable order, e.g. sorted by a hash of the sentence), reproducible by re-running the script.
- **S3 — Bundling.** Each language gets `crates/plexustyping-corpus/data/<lang>/sentences-cv.txt` with `#` header lines (source, commit, licence, filters) and one sentence per line. The corpus loads it together with the existing original `sentences.txt` (both feed `sentences()` and the frequency sample). The diagnostic texts are unchanged.
- **S4 — Script and notices.** A standard-library Python script (extend `scripts/` with a Common Voice importer, or a mode of the existing importer) produces the files from downloaded sources; downloads go under `target/` (the root disk is full), never `/tmp`. `THIRD_PARTY_NOTICES.md` and each `data/<lang>/LICENSE.md` name the source, commit and CC0 licence. README mentions the larger corpus.
- **S5 — Checks.** Corpus tests assert for every language: at least 1,000 sentences in `sentences-cv.txt`, every sentence passes the S2 rules, and every char is typeable on the native layout.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S5 | Importer, five sentence files, corpus loading, tests, notices, README | worker | done | see L3 |

## Log

- **L1 (user, verbatim, 2026-11-17):** "hay mas textos o repositorios de mecanografia en español que podamos usar" → recommended Common Voice (CC0); Monkeytype and GNU Typist are GPL and cannot be bundled.
- **L2 (user, verbatim, 2026-11-17):** "hay que pensar que necesitamos para otros idiomas tambien no?" then "ok pues usa esa".
- **L3 (evidence, 2026-11-17):** Common Voice `sentence-collector.txt` at commit 605c25e4; kept es 1,264 (of 13,026; below the 2,000 cap), en/pt/fr/de 2,000 each; reruns byte-identical. CV sentences feed `sentences()` and the frequency sample, which moved one AI test's expected counts (ntr 2→1, ti 3→4; parent edit). cargo 349, vitest 203, fmt clean. Known quality limits: some archaic sentences and a few real people's names (no name blocklist).
