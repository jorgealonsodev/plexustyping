# Phase 3 · Local profiles

Several local profiles (PRD RF10: "en la fase 3 varios. Cada perfil tiene su historial, layout, idioma y metas"). This block: create, rename, delete and switch profiles; layout, language and theme per profile; history and stats already follow `profile_id`. Goals/streak and kids mode are later blocks. No schema migration: the v1 `profiles` table already has every column used here.
Branch `feat/phase-3-profiles`. Backend (T1) and frontend (T2) are written in parallel against the contract in S2–S3. Runners: `cargo test --workspace`, `pnpm test`, `pnpm check`, `pnpm build`.

## Specs

- **S1 — Store API** (`crates/plexustyping-store`). `profiles() -> Vec<Profile>` (by id), `profile(id) -> Option<Profile>`, `create_profile(name, language, layout) -> Profile`, `update_profile(id, ProfileUpdate { name, language, layout, theme })` (each field optional; `theme: Some(None)` clears it), `delete_profile(id)` (cascades to its sessions, keystrokes and stats; other profiles untouched). Names are trimmed, 1–40 characters, unique ignoring case; errors are typed (`StoreError` variants) and map to the Spanish messages "El nombre del perfil no puede estar vacío", "El nombre del perfil admite como máximo 40 caracteres", "Ya existe un perfil con ese nombre", "No se puede borrar el único perfil", "Ese perfil no existe". Deleting the last remaining profile is refused and changes nothing. A rejected call leaves every row unchanged.
- **S2 — Active profile** (`src-tauri`). `AppState` holds the active profile behind a lock; every command that used `state.profile` reads the current one (clone before any `.await`). The active id is saved in the SQLite `settings` table, key `active_profile`; at start a missing, unparsable or deleted id falls back to `default_profile()`. Deleting the active profile switches to the first remaining one. Temporary (in-memory) mode supports profiles in memory.
- **S3 — Commands** (camelCase JSON, `Profile` = `{id, name, language, layout, theme, createdAtMs}`): `profiles_list() -> { active: i64, profiles: Profile[] }`, `profile_create(name, language, layout) -> Profile` (does not switch), `profile_update(id, name?, language?, layout?, theme?) -> Profile` (`theme: null` leaves it, empty string clears it), `profile_delete(id) -> { active: Profile }`, `profile_switch(id) -> Profile`. Errors are the Spanish strings of S1. Unknown language or layout ids are refused ("Idioma desconocido", "Distribución desconocida").
- **S4 — UI.** Ajustes gains a section "Perfiles": the list with the active one marked, "Usar" on the others, "Nuevo perfil" (name; it starts with the active profile's language and layout), "Renombrar", and "Borrar" with a confirmation "Se borrarán el historial y las estadísticas de «<nombre>». ¿Continuar?". The header shows the active profile's name (button that opens Ajustes → Perfiles). Errors are shown in Spanish.
- **S5 — Per-profile settings.** Layout, language and theme are read from and written to the active profile (`profile_update`); every other setting stays global in localStorage. Switching profile applies its layout, language and theme (a null theme keeps the current one and saves it to the profile), loads a new text and refreshes heatmap, weaknesses and history. Imported VS Code themes stay global.
- **S6 — Carry-over.** On the first start with this version, the stored `plexustyping.layout`, `plexustyping.language` and `plexustyping.theme` are written once to the active profile (only valid ids), then a flag `plexustyping.profilesMigrated` is set; the localStorage values are left in place. Later starts take these three from the profile.
- **S7 — Docs.** README: profiles, what is per profile, deletion.

## Tasks

| ID | Specs | Task | Route | Status | Commit |
|----|-------|------|-------|--------|--------|
| T1 | S1–S3 | Backend: store API + errors + tests; AppState lock, `active_profile` setting, commands, tests | writer A (parallel) | pending | |
| T2 | S4–S7 | Frontend: profiles API bindings, Ajustes → Perfiles, header name, per-profile layout/language/theme, carry-over, vitest, README | writer B (parallel) | pending | |
| T3 | S1–S7 | Integration, full checks, independent verification (Risk item 1: profile deletion deletes data; item 4: shared state lock) + native review | parent + gentle-ai-verify | pending | |

## Log

- **L1 (user, verbatim, 2026-11-18):** "continua con lo que consideres mas optimo" — after the agent named local profiles as the next phase 3 block. The agent chose this scope and design (S1–S7); goals/streak and kids mode are separate later blocks.
- **L2 (PRD, verbatim):** "Perfiles locales: en el MVP hay un perfil único por defecto, y en la fase 3 varios. Cada perfil tiene su historial, layout, idioma y metas." / "Datos del perfil: borrado y exportación de todos sus datos."
