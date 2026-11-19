# Settings organisation and quick search

Reorganise the Ajustes window by topic and add a quick search. Branch `feat/linux-packages`, ships in v0.1.0.

## Specs

- **S1 — Organisation.** User: "Organiza bien el menu de ajustes".
  - Section order: Práctica, Texto, Teclado, Metrónomo, Código, Apariencia, Accesibilidad, IA, Perfiles.
  - "Texto" (new) holds everything that changes how the practice text looks: Fuente (with a field to try it), Tamaño de letra, Espaciado entre letras, Interlineado, Cursor, Cursor parpadeante and Atenuar texto pendiente.
  - "Apariencia" keeps the theme and the VS Code theme import.
  - "Accesibilidad" (new) holds "Anunciar con lector de pantalla" (from Práctica) and "Modo daltónico" (from Teclado), plus a note pointing to the high-contrast themes and to Texto.
- **S2 — Quick search.** User: "añade un buscador rapido".
  - A search box in the window header gets the focus when the window opens; `Ctrl+F` returns to it from anywhere in the window.
  - Every section shows only the rows whose text (name, description and option labels) holds every typed word, ignoring case and accents. The matching sections are listed with their names and a polite match count ("N ajustes" / "Ningún ajuste coincide").
  - `Enter` focuses the first match. `Escape` clears a non-empty search, and a second `Escape` closes the window. Clicking a section tab clears the search.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1–S2 | TextSection and AccessibilitySection, registry order, search in SettingsModal, `matchesSettingQuery`/`isSearchShortcut` with tests, README | parent (inline) | done | see L2 |

## Log

- **L1 (user, verbatim, 2026-11-19):** "Organiza bien el menu de ajustes y añade un buscador rapido"
- **L2 (evidence, 2026-11-19):** done inline, because workers cannot start while `/` is full (ENOSPC on `/tmp`).
  - RED: the section order and the search tests failed. GREEN: vitest 503/503, check 0, build ok.
  - The search looks at every `.field` at any depth, because AiSection wraps its rows in one container. Other top-level content stays only beside a matching row.
  - Not tested automatically: the search UI wiring in the modal (no component tests); it needs a GUI check.
