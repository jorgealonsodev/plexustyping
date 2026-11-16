# PlexusTyping

*Del cerebro a los dedos.* An adaptive typing tutor built with Rust and Tauri 2.

Status: phase 1A (adaptive engine and local storage). See `PRD-PlexusTyping.v0.1.md` for the product plan.

Phase 0 is a working practice screen (Spanish, QWERTY ES, fully offline):

- Starts with the diagnostic text until the first session is saved, then with an
  adaptive text; the clock starts on the first key, `Esc` restarts.
- Each character turns correct or wrong as you type; optional live bar with WPM,
  keystrokes per minute, accuracy and time.
- Results show gross and net WPM, accuracy, time, errors and the failed characters.
  "Siguiente texto" (`Enter`) generates a new adaptive text (see below).
- Accents and dead keys work: typing goes through a hidden text field, so the
  system input method composes `´` + `a` into one `á`.
- On-screen ISO keyboard at the bottom, labelled from the active layout (normal,
  Shift, AltGr; dead keys underlined): it highlights the next key, plus Shift or
  AltGr, coloured by finger, and flashes typed keys green or red. `F2` shows or hides it.
- Settings: the gear button in the header (or `Ctrl+,`) opens the "Ajustes"
  window, with one section per topic. "Práctica": error mode (continue, stop
  until corrected, or fix with Backspace; changing it restarts the text),
  targets per text (3–5), text length (100–600 characters) and live stats.
  "Teclado": on-screen keyboard (`F2`), flashes and heatmap. "Apariencia": 57
  colour themes, dark and light, switched live. Everything is remembered
  locally; targets and length apply from the next text. `Escape`, `×` or a
  click outside closes the window and returns to the text.

### Adaptive loop

Type → measure → recalculate weaknesses → generate a text that attacks them.
Every completed session is replayed in Rust and folded into a weakness model:
each character and bigram («qu», «rr») keeps an exponential moving average of
errors (weight 0.7) and slowness relative to your own pace (weight 0.3). The
next text picks the 3–5 worst units, with some randomness, and builds sentences
from words that contain them ("Practicando: ñ · qu · z" above the text). Results
list your top weaknesses, and the optional "Mapa de calor" tints each key of
the on-screen keyboard by its score.

### Where data lives

Sessions, keystrokes and statistics are stored only on this machine, in
`plexustyping.sqlite3` under the app data directory (on Linux
`~/.local/share/com.plexustyping.app/`). If that file cannot be opened the app
still works with a temporary in-memory store and shows "Sin guardar: modo
temporal" in the header. Display settings stay in the webview's local storage.

## Layout

| Path | Contents |
| --- | --- |
| `crates/plexustyping-core` | Typing session engine: input replay, error modes, stats |
| `crates/plexustyping-layouts` | Keyboard layouts (TOML) and dead-key composition |
| `crates/plexustyping-corpus` | Local offline corpus and AI-free text generator |
| `crates/plexustyping-store` | Local SQLite store: profiles, sessions, keystrokes, statistics |
| `src-tauri` | Tauri app (`plexustyping-app`, binary `plexustyping`) |
| `src` | Svelte 5 frontend |

## Development

Requirements: Rust 1.85+, Node 22+, pnpm, and the
[Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/).

```sh
pnpm install
pnpm tauri dev      # run the app
cargo test --workspace
pnpm check && pnpm test
pnpm tauri build    # installers
```

## License

Code: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
Bundled data keeps its own license; see `THIRD_PARTY_NOTICES.md`.
