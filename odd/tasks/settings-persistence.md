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
| T1 | S1–S5 | Backend commands over the `settings` table (UI keys only), a synchronous `Storage` adapter preloaded before the app mounts, serialized writes, one-time migration, every `localStorage` use routed through it, tests (Rust and vitest) | worker | pending | |

## Log

- **L1 (user, verbatim, 2026-11-19):** "se debe guardar los ajustes y configuracion de los botones como lo dejamos al cerrar la app"
- **L2 (evidence, 2026-11-19):** `~/.local/share/com.plexustyping.app/localstorage/` holds two separate origins.
  - `http_localhost_1420` (the dev app) has heatmap, keyboard, language, layout and practiceKind.
  - `tauri_localhost_0` (the installed .deb) has only keyboard, profilesMigrated and theme.

  So settings made in one build never reach the other. WebKit also flushes localStorage to disk lazily, which can lose a change made right before quitting. The SQLite store already has a `settings` table (`get_setting`/`set_setting` in crates/plexustyping-store/src/lib.rs), and both builds share it.
