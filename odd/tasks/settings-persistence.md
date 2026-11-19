# Settings persistence in the store

UI settings and header toggles survive closing the app, in the dev app and the installed .deb alike, by keeping them in the app's SQLite store instead of the webview's localStorage. Branch `feat/linux-packages`, ships in v0.1.0.

## Specs

- **S1 — Persist on close.** User: "se debe guardar los ajustes y configuracion de los botones como lo dejamos al cerrar la app". Every UI setting the app keeps today in `localStorage` (`plexustyping.*` keys: error mode, live stats, ghost, keyboard, flashes, colour-blind, dim pending, themes and custom themes, heatmap, layout, language, target count, text length, metronome, practice kind, code language, indent, lines, screen reader, typography, practice font and the others found by searching for `localStorage`) is read from and written to the store's `settings` table. A change is written to SQLite when it is made, not when the window closes, so closing or killing the app right after a change keeps it.
- **S2 — One place for every build.** The dev app (`http://localhost:1420`) and the installed app (`tauri://localhost`) read the same values, because they share `~/.local/share/com.plexustyping.app/plexustyping.sqlite3`.
- **S3 — Migration.** On the first start where the store holds no UI settings, every `plexustyping.*` key present in the current webview's `localStorage` is copied into the store once, so nothing configured before is lost. Later starts read only the store. The localStorage values are left in place, untouched.
- **S4 — Same behaviour.** The existing `load*`/`save*` helpers keep their validation, defaults and clamping. Only where the values are stored changes. The app shows the stored settings from its first render, with no flash of defaults.
- **S5 — Failure.** If the store cannot be read at start, the app starts with localStorage as today and logs the error. If a write fails, the in-memory value stays and the error is logged. No crash in either case.

## Tasks

| ID | Specs | Work | Route | Status | Commit |
|---|---|---|---|---|---|
| T1 | S1–S5 | Backend commands over the `settings` table (UI keys only), a synchronous `Storage` adapter preloaded before the app mounts, serialized writes, one-time migration, every `localStorage` use routed through it, tests (Rust and vitest) | worker | done | see L3 |

## Log

- **L1 (user, verbatim, 2026-11-19):** "se debe guardar los ajustes y configuracion de los botones como lo dejamos al cerrar la app"
- **L2 (evidence, 2026-11-19):** `~/.local/share/com.plexustyping.app/localstorage/` holds two separate origins.
  - `http_localhost_1420` (the dev app) has heatmap, keyboard, language, layout and practiceKind.
  - `tauri_localhost_0` (the installed .deb) has only keyboard, profilesMigrated and theme.

  So settings made in one build never reach the other. WebKit also flushes localStorage to disk lazily, which can lose a change made right before quitting. The SQLite store already has a `settings` table (`get_setting`/`set_setting` in crates/plexustyping-store/src/lib.rs), and both builds share it.
- **L3 (evidence, 2026-11-19):** T1 done.
  - Backend: commands `ui_settings_get`, `ui_settings_set` and `ui_settings_remove` in `src-tauri/src/ui_settings.rs` store values under `ui:plexustyping.*` in the `settings` table. Keys are limited to 128 bytes and values to 256 KiB. In temporary mode `get` refuses, so the app keeps using localStorage. New store methods: `settings_with_prefix` and `delete_setting`.
  - Frontend: `src/features/settings/persist.ts` holds an in-memory snapshot with serialized writes. `initSettingsStorage()` is awaited in `main.ts` before the mount. On the first start it migrates `plexustyping.*` keys once, behind the marker `plexustyping.settingsInStore`. Every former default storage now goes through `settingsStorage()`.
  - RED: E0599/E0425, a missing module, and 2 routing failures. GREEN: cargo, clippy, vitest 494/494, check 0, build ok.
  - Known limits:
    - Only the origin of the build that starts first is migrated.
    - A write still queued in the page when the window is killed can be lost; each write is durable once handled.
- **L4 (review follow-up, 2026-11-19):** review-9a6ea80b3804ab14 advisories R3/R4 (partial migration) are fixed.
  - Migration is now gated on the `plexustyping.settingsInStore` marker, not on an empty store, and it copies only the keys the store lacks; the store's own values win.
  - The marker is written only after every copy landed, counted with the new `StoreStorage.failures`. An interrupted or failed copy is therefore retried at the next start.
  - RED: 2 new tests. One existing test now waits for the marker with a second flush.
  - Checks: vitest 505/505, check 0.
