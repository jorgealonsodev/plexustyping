# Phase 4 · Accessibility audit and fixes

Close the PRD §6.3 accessibility requirements so the phase 4 checklist has 0 blockers. Built from the read-only audit of 2026-11-19 (L2). Branch `feat/phase-4-accessibility`.

## Specs

- **S1 — Contrast (PRD ~436).** PRD: "**Contraste:** todos los temas incluidos cumplen al menos el nivel AA." Every built-in theme (all of `THEMES`) meets WCAG AA for the derived CSS variables. Text pairs need ≥ 4.5:1:
  - main text, pending (not yet typed) text and muted text on `--bg`, `--surface` and `--surface-2`;
  - error text on its tint;
  - code pending plain text and comments on the code background;
  - key labels on highlighted, flashed and heatmap keys.

  UI component pairs need ≥ 3:1: the focus outline, the accent on `--bg`, and active vs inactive tabs and toggles.

  Derived colours that fall short are pushed toward the theme's foreground until they pass. They are never left below the threshold. Imported VS Code themes get the same correction. A vitest sweeps every built-in theme and every pair above.
- **S2 — Keyboard navigation (PRD §6.3).** PRD: "navegación completa por teclado y ARIA en los menús".
  - Every control in the header, the mode switch and the metronome toggle can be reached and used with the keyboard.
  - In prose and words modes, Tab and Shift+Tab move focus out of the typing field as usual. The typing field no longer pulls focus back from a control that received it by keyboard. A mouse click on the practice area still refocuses the field.
  - In code mode Tab keeps typing indentation.
  - In every mode, F6 moves focus from the typing field to the first header control. The typing hint names that key.
  - Escape restarts the text only when the typing field has focus.
  - The metronome quick panel:
    - moves focus into itself on open;
    - keeps Tab inside itself while open;
    - closes on Escape from the toggle or from inside the panel, without restarting the text;
    - returns focus to its toggle.
  - Closing Settings or History returns focus to the control that opened it, or to the typing field when a shortcut opened it.
  - The logo button has an `aria-label`.
- **S3 — Screen reader mode (PRD §6.3).** PRD: "en la práctica, un modo opcional que anuncia el siguiente carácter y los errores (`aria-live`)."
  - A setting "Anunciar con lector de pantalla" (off by default) turns on a polite `aria-live` region.
  - The region announces the next expected character after each keystroke, with spoken names for invisible or ambiguous characters ("espacio", "salto de línea", "tabulador", and the punctuation names). Example: «siguiente: a».
  - On a wrong keystroke it announces the error with the expected and the typed character. Example: «error: esperaba e, pulsaste r».
  - It announces the start of a new text and the result line.
  - The practice text is a named region: no `aria-label` on a `<p>`.
  - The typing field is linked to it with `aria-describedby`.
  - The setting is off by default and announces nothing while off.
- **S4 — Typography (PRD §6.3).** PRD: "tamaño de fuente, espaciado entre letras e interlineado ajustables".
  - Three settings apply to the practice text, both prose and code:
    - "Tamaño de letra": 80–200 %, step 10, default 100;
    - "Espaciado entre letras": 0–0.30 em, step 0.02, default 0;
    - "Interlineado": 1.2–2.4, step 0.1, default the current value.
  - They are stored with the other settings and survive a restart.
  - A value outside its range is clamped.
- **S5 — High contrast and colour-blind keyboard (PRD §6.3).** PRD: "temas de alto contraste y modo daltónico en el teclado".
  - Two built-in themes, "Alto contraste oscuro" and "Alto contraste claro", with ≥ 7:1 for text and ≥ 4.5:1 for UI.
  - A keyboard setting "Modo daltónico" (off by default) replaces the red/green flashes with an Okabe–Ito orange/blue pair. A wrong key also gets a non-colour cue (a bold underline).
  - The heatmap gets a non-colour cue: a numeric level 1–5 on the key or as its accessible label, plus a legend that names the levels.
- **S6 — Reduced motion (PRD §6.3, ~335).** PRD: "se respeta `prefers-reduced-motion`, sin destellos ni animaciones" and "Con `prefers-reduced-motion` se sustituye por un subrayado."
  - Under `prefers-reduced-motion: reduce` the key flash becomes an underline (green or red, blue or orange in colour-blind mode) with no transition or animation.
  - The beat dot changes state without transition or animation.
  - The LED transition and every other CSS transition or animation in the app are off.
  - The metronome keeps a visual pulse equivalent to its sound (PRD: "el metrónomo tiene un pulso visual equivalente").

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1 | Contrast guard for derived theme colours, all-themes AA vitest, CodeText comment/plain colours | writer A (parallel with T2) | done | see L5 |
| T2 | S2 | Keyboard navigation: Tab/F6 out of the typing field, refocus rules, Escape scope, metronome panel focus, focus return in dialogs, logo label | writer B (parallel with T1) | done | see L4 |
| T3 | S3, S4 | Screen reader mode and typography settings (practice text, settings store, settings UI) | writer after T2 | pending | |
| T4 | S5, S6 | High-contrast themes, colour-blind keyboard mode, heatmap levels, reduced motion | writer after T1 | pending | |
| T5 | S1–S6 | Integration checks (cargo, vitest, check, build), README, native review, merge | parent | pending | |

## Log

- **L1 (user, verbatim, 2026-11-19):** "haz lo que consideres mas optimo" (after the agent proposed starting phase 4 with accessibility).
- **L2 (evidence, 2026-11-19):** read-only audit (gentle-ai-explore) against PRD §6.3. Blockers, in priority order:
  1. Built-in themes fail AA, the default included: pending text 3.41 in Mocha, 2.09 in Latte, 1.74 in Solarized Light; code comments 2.00.
  2. Tab is swallowed and focus is pulled back while typing (`Practice.svelte:411,428`).
  3. No `aria-live` anywhere.
  4. No typography settings.
  5. No high-contrast theme or colour-blind mode.
  6. Reduced motion is honoured only by the spinner.
  7. The metronome panel has no focus management, and Escape on its toggle restarts the text.

  Nice-to-haves folded into S2 and S3: focus return in the dialogs, the logo `aria-label`, and the `<p aria-label>`.
- **L3 (agent decisions, 2026-11-19, delegated by L1):**
  - In code mode Tab keeps typing indentation, and F6 leaves the typing field in every mode.
  - The colour-blind pair is Okabe–Ito orange `#E69F00` and blue `#0072B2`.
  - The contrast fix moves derived colours toward the theme foreground rather than changing imported palettes.
  - The typography ranges are the ones in S4.
- **L4 (evidence, 2026-11-19):** T2 done by writer B plus a parent fix.
  - Pure decisions live in `src/features/practice/focus.ts` (`tabTypes`, `escapeRestarts`, `isHeaderShortcut`, `isPointerClick`, `nextFocusIndex`, `typingHint`, `returnFocus`), with 17 vitest. RED 17/17, then GREEN.
  - Wiring: Practice.svelte, MetronomeQuick.svelte, SettingsModal.svelte, HistoryModal.svelte and App.svelte. F6 focuses the logo button, now labelled «Teclado en pantalla».
  - Parent fix: ModeSwitch.svelte refocused the typing field on every select change, which broke arrow-key use. It now refocuses only after a pointer pick.
  - Checks: vitest without themes 286/286; check 0.
  - Known limits:
    - A focused button that disappears or becomes disabled drops focus to the page, and the typing field then takes it.
    - Shift+Tab in code mode still types indentation.
    - The Svelte focus wiring has no component test; it needs a GUI check.
- **L5 (evidence, 2026-11-19):** T1 done by writer A.
  - `ensureContrast`/`correctColor` in `colors.ts`: a passing colour stays byte-identical; a failing one moves toward the foreground, then toward black or white.
  - Applied to text, pending and muted text on all three layers, error on its tint, every pending code class (new `--syntax-pending-*`, used by CodeText.svelte), key labels on highlighted, modifier and flashed keys, the accent and `--on-accent`.
  - A new `--heat-base` (the theme's `--danger`, weakened toward `--key-bg` only as far as needed) is used by the heatmap and its legend in Keyboard.svelte. Flashes keep `--danger`.
  - RED: 57/57 themes failing (2047 pairs). GREEN: `contrast.test.ts` sweeps all 57 themes and the imported VS Code path. The tightest heat tint is catppuccin-frappe at 4.50.
  - Checks: vitest 376/376, check 0.
  - Note: Mocha `--text-pending` and `--muted` now nearly match (both stop at 4.5 on `--surface-2`).
