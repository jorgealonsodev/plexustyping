# Phase 3 · Import VS Code themes

Third block of phase 3 ("importar temas VS Code"). The PRD is not available on this machine; the spec below is the agent's proposal under the user's delegation (L1) and is an assumption until the user reviews it.
Branch `feat/phase-3-vscode-themes`. Delivery: one work unit, frontend only. Runner: `pnpm test` (vitest), `pnpm check`.

## Specs

- **S1 — Entry point.** "Ajustes → Apariencia" gains a button "Importar tema de VS Code…" that opens the system file chooser for one `.json` / `.jsonc` file. No new Tauri plugin or permission: the file is read in the WebView.
- **S2 — Accepted file.** A VS Code colour theme: JSON that may contain `//` and `/* */` comments and trailing commas, up to 1 MB, with `colors["editor.background"]` and `colors["editor.foreground"]`. Colours may be `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`; alpha is dropped.
- **S3 — Mapping to the app palette.**
  - `background` ← `editor.background`; `foreground` ← `editor.foreground`; `cursor` ← `editorCursor.foreground` (else the foreground); `selectionBackground` ← `editor.selectionBackground` when present.
  - The 16 ANSI colours ← `terminal.ansiBlack` … `terminal.ansiBrightWhite`; each missing or invalid one takes the colour of the built-in base scheme (PlexusTerm for dark themes, the first built-in light scheme for light ones).
  - Dark or light ← `type` (`dark`, `hc`, `hc-black` → dark; `light`, `hc-light` → light); otherwise from the background's luminance.
  - Name ← `name`, else the file name without extension; at most 60 characters. `tokenColors` and `include` are not read.
- **S4 — Errors (shown in the section, nothing is stored).** "El archivo es demasiado grande (máximo 1 MB)."; "El archivo no es un JSON válido."; "El archivo no parece un tema de VS Code: faltan editor.background y editor.foreground."; "Hay demasiados temas importados (máximo 30); elimina alguno."
- **S5 — Result.** The imported theme is stored locally (`plexustyping.customThemes`), applied at once, listed in the "Tema" selector under a group "Importados", and survives a restart. Importing a theme with the same name replaces the earlier import. Success shows "Tema importado: <nombre>". Invalid stored entries are ignored on load.
- **S6 — Removing.** While an imported theme is selected, a button "Eliminar tema importado" removes it and switches to the default theme. Built-in themes cannot be removed.
- **S7 — Docs.** README describes the import, its mapping and limits.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S7 | `themes/custom.ts` (JSONC, mapping, storage) + registry in `themes.ts` + store + Apariencia UI + README, with vitest | inline (one unit; no writer reason) | in progress | — |

## Log

- **L1 (user, verbatim):** "continua con lo que consideres mas optimo" (after the adaptive metronome). Remaining on the Trello card "Fase 3 · Completo": "todos los layouts e idiomas, todos los temas, importar temas VS Code, … perfiles". The agent chose the VS Code import: self-contained, no external data. "Todos los temas" is taken as covered by the 57 built-in schemes (phase 0).
- **L2 (exploration):** themes are `ThemeEntry { id, name, dark, palette }` with a terminal-style `Palette` (`src/features/themes/themes.ts`); `typingColors` derives every CSS variable, syntax colours included, from the ANSI palette (`colors.ts`); the theme id is stored in `plexustyping.theme` and validated by `isThemeId`; `main.ts` applies the theme before mount; no dialog/fs plugin and capabilities are `core:default`.
- **L3 (decision):** ANSI colours come only from `terminal.ansi*` because the app colours code from the ANSI palette; deriving them from `tokenColors` scopes is left out.
