# Practice font choice

Let the user pick the typeface of the practice text from a curated, bundled list: the fonts typing programs ship most, plus the web fonts with the best legibility evidence. Branch `feat/linux-packages`, so it ships in v0.1.0.

## Specs

- **S1 — Setting.** User: "hay que poder elegir la fuente, busca las fuentes mas usadas en programas de mecanografia y las tipo web con mejor legitibilidad".
  - Where: Ajustes → Apariencia gets a "Fuente" select. It applies to the practice text, both prose and code; the rest of the UI keeps its font.
  - Persistence: stored with the other settings, survives a restart. An unknown stored id falls back to the default.
  - Default: today's font, so nothing changes for existing users.
  - Each option is shown in its own typeface.
- **S2 — List (L2).** Every font is bundled with the app; the app works offline and loads no font from the network. Only the subsets the app's languages need (latin and latin-ext: es, en, pt, fr, de), weights 400 and 700.
  - Typing staples (Monkeytype's list; its default is Roboto Mono): Roboto Mono, JetBrains Mono, Fira Code (ligatures off), Source Code Pro, IBM Plex Mono, Ubuntu Mono, Inconsolata.
  - Legibility: Atkinson Hyperlegible Mono (Braille Institute, low vision), Intel One Mono (designed with low-vision developers), Lexend Deca (proportional, reading-proficiency research).
  - Optional: OpenDyslexic, only if a packaged OFL build exists. Its hint does not claim a benefit, because the evidence does not show one (L2).
- **S3 — Licences.** Every bundled font is OFL-1.1 or Apache-2.0, and THIRD_PARTY_NOTICES.md lists each one with its licence.
- **S4 — Size.** The fonts add at most about 3 MB to the frontend build.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1–S4 | Font packages, the font list module with tests, the setting and its persistence, the Apariencia select, CSS for prose and code, notices, README | worker | done | see L3 |

## Log

- **L1 (user, verbatim, 2026-11-19):** "2 detalles, hay que poder elegir la fuente, busca las fuentes mas usadas en programas de mecanografia y las tipo web con mejor legitibilidad. Y cuando se elegie un proveedor de ia, automaticamente tiene qeu elegirse un modelo, no se si los provedoores marcan un modelo por defecto, pero si no es asi, conge uno random o el primero". The second part was done in 4de90fb: a provider without a default model gets its first listed model.
- **L2 (research, 2026-11-19):**
  - Monkeytype `packages/schemas/src/fonts.ts` lists 43 fonts. Its default is Roboto_Mono, and the list includes Source_Code_Pro, Inconsolata, Fira_Code, JetBrains_Mono, IBM_Plex_Mono, Ubuntu_Mono, Atkinson_Hyperlegible, Lexend_Deca, Open_Dyslexic and others.
  - Atkinson Hyperlegible (Mono) was made for the Braille Institute for low-vision readers.
  - Intel One Mono was designed with low-vision and blind developers.
  - The Readability Group study (2021: 20 typefaces, more than 3,000 respondents) found commercially marketed dyslexia fonts performed worse than several general sans-serif fonts.
- **L3 (evidence, 2026-11-19):** T1 done by the worker.
  - Code: `src/features/settings/fonts.ts` (the list, "Fuente", fallback, persistence), `fontFiles.ts` (the Fontsource imports), the Apariencia select with Mecanografía and Legibilidad groups plus a preview line, and `--practice-font`/`--practice-features` in Practice.svelte and CodeText.svelte.
  - Fonts: 10 OFL-1.1 Fontsource packages at 5.3.x: Roboto Mono, JetBrains Mono, Fira Code (no ligatures), Source Code Pro, IBM Plex Mono, Inconsolata, Atkinson Hyperlegible Mono, Intel One Mono, Lexend Deca and OpenDyslexic (latin only).
  - Skipped: Ubuntu Mono (UFL-1.0 licence, not OFL/Apache).
  - Size: dist grows by 1.51 MiB (woff2 0.70 MiB plus unused woff fallbacks 0.80 MiB). No remote font URL.
  - RED (missing module), then GREEN: vitest 424/424, check 0, build ok.
  - Parent decisions:
    - Accepted: `--mono` starts with "JetBrains Mono", so the default now always gets the bundled JetBrains Mono. The look is the same on every machine instead of depending on installed fonts. It also applies where the UI uses `var(--mono)`.
    - Removed the empty `fonts.css` leftover.
