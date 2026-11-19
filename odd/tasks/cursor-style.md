# Cursor style

Let the user choose how the typing cursor looks in the practice text. Branch `feat/linux-packages`, ships in v0.1.0.

## Specs

- **S1 — Style.** User: "Tambien quiero que se pueda personalizar el tipo de cursor tipo barra y demas".
  - Where: Ajustes → Apariencia gets a "Cursor" select, applied to the practice text in prose and code.
  - Options:
    - "Resaltado": today's look, a tint with an underline bar. It is the default.
    - "Barra": a thin vertical bar before the next char.
    - "Bloque": the next char on a solid cursor-coloured block, its glyph in the cursor text colour.
    - "Subrayado": an underline only, with no tint.
    - "Contorno": a frame around the next char.
  - Every style uses the theme's `--cursor` colour. When the char under the cursor is wrong, the style is the same but in the error colour, as today.
- **S2 — Blink.** A toggle "Cursor parpadeante", off by default. It makes the cursor blink about once per second with a stepped animation. It is disabled by the global `prefers-reduced-motion` rule (a11y S6), which already turns animations off.
- **S3 — Persistence.** Both settings are stored with the other UI settings (`settingsStorage()`, store-backed) and survive a restart. An unknown stored id falls back to "Resaltado".
- **S4 — No regressions.**
  - The ghost cursor, error and space marks, scroll-into-view and dimmed pending text are unchanged.
  - With the default style, the cursor looks exactly as before.
  - Each style keeps the char width, so text never shifts: no borders that add width; use box-shadow or outline.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1–S4 | Settings module and tests, store, Apariencia select and toggle, attributes on the practice text wrapper, cursor CSS in Practice.svelte and CodeText.svelte, README | parent (inline) | done | see L3 |

## Log

- **L1 (user, verbatim, 2026-11-19):** "Tambien quiero que se pueda personalizar el tipo de cursor tipo barra y demas"
- **L2 (agent decisions, 2026-11-19):** the five styles of S1; blink is an opt-in toggle; and styles are switched with `data-cursor`/`data-cursor-blink` attributes on the practice text wrapper, read by `:global([data-cursor=…]) .cursor` rules in both components. The latter is needed because Svelte scopes each component's `.cursor` rule, and CodeText renders inside the wrapper.
- **L3 (evidence, 2026-11-19):** done inline by the parent, because a worker could not start: ENOSPC on `/tmp`, with the root filesystem full.
  - `src/features/settings/cursor.ts` holds the styles, the fallback and persistence under `plexustyping.cursorStyle` and `plexustyping.cursorBlink`. Tests in `cursor.test.ts`: RED (missing module), then GREEN 4/4.
  - Settings: store fields and setters, plus the "Cursor" select and "Cursor parpadeante" toggle in Apariencia.
  - Practice: `data-cursor` and `data-cursor-blink` on the practice text region, and `--cur` rules in Practice.svelte and CodeText.svelte. They are placed after the `.cursor.error` rules so the error colour applies in every style.
  - Checks: vitest 500/500, check 0, build ok.
  - Not covered by any test: the GUI look of each style.
