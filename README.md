# PlexusTyping

*Del cerebro a los dedos.* An adaptive typing tutor built with Rust and Tauri 2.

Status: phase 0 (prototype). See `PRD-PlexusTyping.v0.1.md` for the product plan.

Phase 0 is a working practice screen (Spanish, QWERTY ES, fully offline):

- Starts with the diagnostic text; the clock starts on the first key, `Esc` restarts.
- Each character turns correct or wrong as you type; optional live bar with WPM,
  keystrokes per minute, accuracy and time.
- Results show gross and net WPM, accuracy, time, errors and the failed characters.
  "Siguiente texto" (`Enter`) generates a text that practises those characters.
- Accents and dead keys work: typing goes through a hidden text field, so the
  system input method composes `´` + `a` into one `á`.
- On-screen ISO keyboard at the bottom, labelled from the active layout (normal,
  Shift, AltGr; dead keys underlined): it highlights the next key, plus Shift or
  AltGr, coloured by finger, and flashes typed keys green or red. `F2` shows or hides it.
- Settings: error mode (continue, stop until corrected, or fix with Backspace),
  57 colour themes switched live, live stats, keyboard and flashes toggles;
  all remembered locally.

## Layout

| Path | Contents |
| --- | --- |
| `crates/plexustyping-core` | Typing session engine: input replay, error modes, stats |
| `crates/plexustyping-layouts` | Keyboard layouts (TOML) and dead-key composition |
| `crates/plexustyping-corpus` | Local offline corpus and AI-free text generator |
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
