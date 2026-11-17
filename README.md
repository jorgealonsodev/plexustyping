<p align="center">
  <img src="assets/logo-wordmark.svg" alt="PlexusTyping — Adaptive typing" width="420">
</p>

# PlexusTyping

An adaptive typing tutor built with Rust and Tauri 2.

Status: phase 2 in progress (code mode). See `PRD-PlexusTyping.v0.1.md` for the product plan.

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
  click outside closes the window and returns to the text. "IA" and
  "Metrónomo": see below.

### Adaptive loop

Type → measure → recalculate weaknesses → generate a text that attacks them.
Every completed session is replayed in Rust and folded into a weakness model:
each character and bigram («qu», «rr») keeps an exponential moving average of
errors (weight 0.7) and slowness relative to your own pace (weight 0.3). The
next text picks the 3–5 worst units, with some randomness, and builds sentences
from words that contain them ("Practicando: ñ · qu · z" above the text). Results
list your top weaknesses, and the optional "Mapa de calor" tints each key of
the on-screen keyboard by its score.

### AI texts (optional)

In "Ajustes → IA", turn on "Usar IA para generar textos" and build the fallback
chain: up to four providers (Claude, OpenAI, Google Gemini, OpenCode Go), tried
in order, each with its model (picked from the provider's list, or typed by
hand), its API key ("Guardar clave" / "Borrar clave") and "Probar conexión".
"Densidad" (2×–5×, default 3×) sets how much more often the targets appear and
"Tema" an optional topic. The usage table counts calls and tokens per provider,
today and in total, so you can watch your spending.

- **Keys** are stored only in the system keychain (Keychain on macOS,
  Credential Manager on Windows, Secret Service on Linux; service
  `com.plexustyping.app`, account `ai:<provider>`), never in plain text, never
  in the database, and never sent back to the interface. Without a working
  keychain no key can be saved.
- **Privacy:** only the target characters, the language, the length, the
  allowed characters and the topic are sent. Nothing you type ever leaves
  this machine. Requests go from Rust to the provider hosts only.
- **Fallback:** each request has a 10 s timeout; a provider whose text fails
  validation gets one retry, then the next provider is tried. If all fail (or
  no key is set), a previously generated text from the local cache is used,
  and then the local corpus. A short note above the text says why, and the
  source is always shown ("Generado con IA · Claude · …", "Texto del caché",
  "Corpus local").
- **Latency:** the next text is requested in the background as soon as you
  start typing the current one, so it is usually ready when you finish.

### Code mode

The "Prosa / Código" switch in the header (next to the gear) changes what you
practise; in code mode a select next to it picks the language: Python,
JavaScript, TypeScript, Java, C#, C++, C, Go, Rust, PHP, SQL, Bash, HTML/CSS,
Kotlin or Swift. Switching loads a new text at once.

- **Snippets:** 5–20 lines (Ajustes → Código → "Líneas máximas", default 12)
  from the bundled original snippets, chosen by how often they use your weak
  symbols (`{} [] () <> ; : = -> => && || " ' _ # $`…). With AI on, the
  fallback chain is asked for a snippet in that language first (printable
  ASCII only, newlines and indentation kept), then the cache of earlier AI
  snippets of the same language, then the bundled ones.
- **Highlighting:** the text keeps its lines and indentation and is coloured by
  syntax (`syntect` grammars, tokenized once per text in Rust) with the active
  theme's syntax colours; characters still to type are dimmed, typed ones take
  their full colour, errors show as in prose. A `↵` marks a newline at the
  cursor or typed wrong; the block scrolls to keep the cursor line in view.
- **Keys:** `Enter` types the newline and `Tab` types a tab or skips the
  spaces up to the next tab stop; both are measured keystrokes. "Sangría":
  "Automática al pulsar Enter" (default: after a correct newline the next
  line's indentation is filled in and not measured) or "Manual" (every space
  is typed). Changing it restarts the text.
- **Separate statistics:** code sessions are stored with their language and
  feed their own weakness model (mode `code`, shared by every language), so
  symbols never touch the prose metrics. Results, weaknesses and the keyboard
  heatmap in code mode come from that model.

### Metronome

In "Ajustes → Metrónomo" (or with `F4` from anywhere), turn on the fixed
metronome: 30–300 BPM (default 60; one click = one expected keystroke), sound
"Clic", "Madera" or "Bip", volume 0–100 % and an accent every N beats (0 = no
accent, up to 16). Changes apply live. It plays from Rust (`cpal`, the default
output device) only while a text is ready or being typed, and stops on the
results screen and when the window is hidden. A dot next to the live stats
flashes on each beat (accented beats in another colour).

- **Offset metric:** each typed character records how many ms before
  (negative, "adelantada") or after (positive, "atrasada") the nearest click
  it fell. Results show "Ritmo: desvío medio ±X ms" (mean absolute offset) and
  the share of early and late keys. Offsets are stored per keystroke, and the
  session's BPM when one tempo covered the whole text. Changing the tempo
  restarts the beat.
- **Latency caveat:** beat 0 is timed when the audio stream starts; the
  device's own output latency (a few ms) is not included: the click is heard
  that much later, so keys in time with it read a few ms late.

### History and CSV export

The clock button in the header (next to the gear), or `Ctrl+H`, opens the
"Historial" window. A filter "Todo / Prosa / Código" picks the sessions shown;
everything below follows it:

- **Summary:** number of sessions, mean and best net WPM, and mean accuracy.
- **Chart:** net WPM and accuracy of the last 50 sessions, oldest on the left,
  drawn in plain SVG.
- **Table:** every session, newest first: date, mode and language, net WPM,
  accuracy, time and BPM (empty without a metronome). It scrolls inside the
  window.
- **"Exportar CSV"** writes all sessions of the active filter straight to your
  Downloads directory as `plexustyping-sesiones-AAAAMMDD.csv` (local date; an
  existing file is never overwritten, `-2`, `-3`… is appended instead) and
  shows "Exportado: <ruta> (N sesiones)". The file is standard CSV: UTF-8
  without BOM, `,` separator, `.` decimal point, RFC 4180 quoting, `\n` line
  ends, newest first, with the header
  `fecha,modo,lenguaje_codigo,idioma,layout,duracion_s,ppm_brutas,ppm_netas,precision,pulsaciones,bpm`.
  `fecha` is local ISO 8601 time with offset (`2026-11-17T09:30:05+01:00`),
  `modo` is `prose` or `code`, durations, WPM and `precision` (0–100) have one
  decimal, and `lenguaje_codigo` (prose) and `bpm` (no metronome) may be empty.
  With no sessions nothing is written and "No hay sesiones que exportar." is
  shown.

Without saved sessions (or in temporary mode) the window shows "Aún no hay
sesiones guardadas.". `Escape`, `×` or a click outside closes it and returns
to the text.

### Where data lives

Sessions, keystrokes and statistics are stored only on this machine, in
`plexustyping.sqlite3` under the app data directory (on Linux
`~/.local/share/com.plexustyping.app/`). If that file cannot be opened the app
still works with a temporary in-memory store and shows "Sin guardar: modo
temporal" in the header. The same database keeps the AI settings (without
keys), the cache of generated texts and the usage counters. Display settings
stay in the webview's local storage.

## Layout

| Path | Contents |
| --- | --- |
| `crates/plexustyping-core` | Typing session engine: input replay, error modes, stats |
| `crates/plexustyping-layouts` | Keyboard layouts (TOML) and dead-key composition |
| `crates/plexustyping-corpus` | Local offline corpus and AI-free text generator |
| `crates/plexustyping-store` | Local SQLite store: profiles, sessions, keystrokes, statistics, AI text cache and usage |
| `crates/plexustyping-ai` | AI providers, prompt, validation and fallback chain |
| `crates/plexustyping-audio` | Metronome audio |
| `crates/plexustyping-highlight` | Syntax highlighting of code snippets (semantic token classes) |
| `src-tauri` | Tauri app (`plexustyping-app`, binary `plexustyping`) |
| `src` | Svelte 5 frontend |

## Development

Requirements: Rust 1.88+, Node 22+, pnpm, and the
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
