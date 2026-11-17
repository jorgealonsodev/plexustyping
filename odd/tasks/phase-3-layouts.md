# Phase 3 · Keyboard layouts

Fourth block of phase 3 ("todos los layouts"). The user chose the layouts (L1); everything else below is the agent's proposal and an assumption until the user reviews it (the PRD is not on this machine).
Branch `feat/phase-3-layouts`. Delivery: one work unit. Runners: `cargo test --workspace`, `pnpm test` (vitest), `pnpm check`.

## Specs

- **S1 — Layouts.** Three built-in layouts, the ones the user named: "es-es", "es-lat" and "qwerty us".
  - `es-qwerty` — "Español (España)": the existing layout; its id is kept because profiles and saved sessions already store it (assumption: no rename to `es-es`).
  - `latam-qwerty` — "Español (Latinoamérica)": ISO, dead keys `´ ¨ ^ `` ` ``, `@` on AltGr+Q, `\` on AltGr+`'`, `~` on AltGr+`+`.
  - `us-qwerty` — "English (US)": ANSI, no dead keys, no AltGr layer.
- **S2 — Choosing it.** "Ajustes → Teclado" gains a selector "Distribución" with the three layouts. The choice is stored locally (`plexustyping.layout`); a missing or unknown stored value loads as `es-qwerty`.
- **S3 — On-screen keyboard.** Labels, the highlighted next key (with Shift, AltGr or the dead key) and the heatmap follow the chosen layout. A layout without the ISO key next to the left Shift (US) is drawn with the ANSI shape: long left Shift, one-row Enter and `\` above it.
- **S4 — Texts.** Generated texts (corpus and AI) only contain characters the chosen layout can type. Changing the layout loads a new text. The diagnostic text of a new profile is shown only when the layout can type all of it; otherwise the first text is an adaptive one.
- **S5 — Sessions.** A saved session records the layout it was typed with (history and the CSV column `layout`). The typing itself still comes from the system keyboard layout: the app layout must match it.
- **S6 — Docs.** README describes the layouts, the selector and the limits.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S6 | Two layout files + registry, layout parameter in `first_text` / `score_session`, setting + selector, ANSI geometry, README, with tests | inline (one unit; no writer reason) | done | `f048ccb` |

## Log

- **L1 (user, verbatim, 2026-11-17):** "los layouts es-es es-lat y qwerty us" — answer to "¿Qué layouts e idiomas quieres en esta versión?". No language was named, so this block adds no corpus: prose stays Spanish on every layout.
- **L2 (exploration):** layouts are TOML files embedded by `crates/plexustyping-layouts` (`BUILTINS`); the frontend hardcodes `LAYOUT = "es-qwerty"` (`src/features/practice/api.ts`) and passes it to `get_layout`, `key_sequences` and `next_text`; `first_text` and the saved session use `profile.layout`; typing goes through a hidden text field, so the app layout only drives the drawn keyboard and the characters texts may contain; `ISO_ROWS` is the only geometry.
- **L3 (decision):** the layout is a local display setting like the other keyboard settings, not a profile column write: the stored profile row is not changed (profiles are a later block). `latam-qwerty` follows the keys Windows and XKB `latam` agree on.
- **L4 (progress, 2026-11-17, stopped at the user's request):** backend half of T1 written, uncommitted on `feat/phase-3-layouts`: `layouts/latam-qwerty.toml`, `layouts/us-qwerty.toml`, the three-entry registry, `diagnostic_for_new_profile(.., layout)`, `typing_profile`, `layout` parameter in the `first_text` and `score_session` commands. `cargo test --workspace` passes. Frontend half not written: `LAYOUTS` / `loadLayout` / `saveLayout` in `settings.ts`, store `layout` + `setLayout`, selector "Distribución" in `KeyboardSection.svelte`, `ANSI_ROWS` / `rowsFor` / `unlabelled` in `geometry.ts` (+ `.w225` and `rowsFor` in `Keyboard.svelte`), `generationKey` sixth argument, `layout` argument in `firstText` / `nextText` / `scoreSession` (drop the `LAYOUT` constant), `layoutId` prop in `Practice.svelte` (reload layout, sequences and text on change), README. Their vitest tests are already written and fail (6 RED in `settings.test.ts`, `prefetch.test.ts`, `keyboard.test.ts`).
- **L5 (evidence, 2026-11-17):** frontend half written; `pnpm test` 192/192, `pnpm check` 0 errors, `cargo test --workspace` ok, `pnpm build` ok. Committed as `f048ccb` (902 insertions, most of them layout TOML data; one work unit as planned). Native assess: medium, review not due. Not checked in the running app yet (S3 drawing, S4 US texts without ñ).
- **L6 (native review, 2026-11-17):** lineage `review-9c423646a1a784f8` over `origin/main..fbc2466` approved (risk + reliability), five advisory findings. User: "haz lo que consideres mas optimo". Fixed in `c870351`: `score_session` rejects an unknown layout (`unknown layout: <id>`, like `first_text`) instead of storing the profile's; the drawn keyboard clears its layout while the new one loads; the adaptive tempo applies only when every keystroke was measured (`tempoChangeFor`, phase-3 metronome S: no change when switched mid-text); imported themes replace by name and a different name whose slug is taken gets `-2`, `-3`… Left for a later block: component tests of the `Practice.svelte` wiring (finding R3-practice-wiring-untested). Checks: vitest 195/195, `pnpm check` 0, `cargo test --workspace` ok, `pnpm build` ok.
- **L7 (native review, 2026-11-17):** lineage `review-19f5863152517c79` over `c870351`+`f046a9a` approved (resilience + reliability), four advisory findings. Acted on: the adaptive tempo now skips a text only when the metronome was switched on or off after the first keystroke (`Track.switched`), so keystrokes typed before the first beat anchor arrives no longer block it; a save/load test pins suffixed theme ids (the loader only checks the `custom-` prefix). Not changed: a rejected `score_session` is already caught in `finish` ("No se pudo calcular el resultado: …" with retry). Checks: vitest 196/196, `pnpm check` 0, `pnpm build` ok.
