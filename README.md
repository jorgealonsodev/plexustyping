<p align="center">
  <img src="assets/logo-wordmark.svg" alt="PlexusTyping — Adaptive typing" width="420">
</p>

# PlexusTyping

An adaptive typing tutor built with Rust and Tauri 2.

Status: phase 2 in progress (code mode). See `PRD-PlexusTyping.v0.1.md` for the product plan.
To install a release, see [Install](#install).

Phase 0 is a working practice screen (Spanish, QWERTY ES, fully offline):

- Starts with the diagnostic text until the first session is saved, then with an
  adaptive text; the clock starts on the first key, `Esc` restarts.
- Each character turns correct or wrong as you type; optional live bar with WPM,
  keystrokes per minute, accuracy and time.
- Results show gross and net WPM, accuracy, time, errors and the failed characters.
  "Siguiente texto" (`Enter`) generates a new adaptive text (see below). The
  round-arrows button in the header (or `F5`) drops the current text and loads a
  new one at any time; its arrows turn while the text loads ("Cargando texto…"
  at start, "Generando texto…" afterwards). The "IA" button right before it is the
  same setting as "Usar IA para generar textos" in Ajustes → IA: lit, new texts are
  generated with the AI; off, new texts are saved AI texts of the same language and
  mode, rotating so they do not repeat, or the local corpus when none is saved, and
  no AI request is made.
- Ghost session: when you type a text again ("Repetir", the diagnostic text, or any
  text that comes back), a translucent second cursor races you at the pace of your
  best earlier session with exactly that text: same profile, same mode and, for code,
  the same language, with the highest net WPM (ties: the earliest). It waits at
  the start until your first key and stays at the end once it gets there. Next to
  the live metrics, a delta shows how far behind ("+1,2 s") or ahead ("−0,8 s")
  you are at the last position you reached. The results then show "Récord anterior:
  52 PPM" and, when you beat it, "¡Nuevo récord para este texto!". A record set in
  this attempt races the next one. The timeline comes from the stored keystrokes
  (`ghost_for_text`), so it works in every error mode and, in temporary mode, with
  the sessions of this run. The time spent deleting with Backspace is not stored per key,
  so it is shared among the keys typed after each deletion. Turn it on or off with
  the ghost button in the header (lit while on) or in Ajustes → Práctica →
  "Sesión fantasma".
- Accents and dead keys work: typing goes through a hidden text field, so the
  system input method composes `´` + `a` into one `á`.
- On-screen keyboard at the bottom (ISO, or ANSI for US), labelled from the chosen layout (normal,
  Shift, AltGr; dead keys underlined): it highlights the next key, plus Shift or
  AltGr, coloured by finger, and flashes typed keys green or red. `F2` or a click on the logo key in the header
  shows or hides it; the bar under its P is lit green while the keyboard shows.
- Reduced motion: with the system's `prefers-reduced-motion: reduce`, no CSS
  transition or animation runs anywhere in the app; a typed key is underlined
  in the flash colour instead of flashing, and the metronome dot still lights
  on every beat, without transition.
- Settings: the gear button in the header (or `Ctrl+,`) opens the "Ajustes"
  window, with one section per topic, in this order: Práctica, Texto, Teclado,
  Metrónomo, Código, Apariencia, Accesibilidad, IA and Perfiles. The search box
  at the top (focused on open, `Ctrl+F` from anywhere in the window) shows only
  the settings whose name, description or options hold every typed word,
  ignoring case and accents, grouped by section with a match count; `Enter`
  goes to the first one and `Escape` clears the search (a second `Escape`
  closes the window). "Práctica": language (see below), error mode (continue, stop
  until corrected, or fix with Backspace; changing it restarts the text),
  targets per text (2 or 3), text length ("Longitud": 100, 200, 300, 450, 600,
  800, 1000, 1500 or 2000 characters, default 300, or "Ilimitado", see
  [Unlimited texts](#unlimited-texts); also in the header, left of the
  Prose / Words / Code switch, where in code mode it picks the maximum lines:
  5, 8, 12, 16 or 20), live stats,
  the ghost session (on by default) and progressive unlock.
  "Accesibilidad": "Anunciar con lector de pantalla"
  (off by default): a polite `aria-live` region then announces the next
  character after each keystroke («siguiente: a», with spoken names such as
  "espacio", "salto de línea", "tabulador" or "coma"), each wrong key
  («error: esperaba e, pulsaste r»), the start of a new text and the result
  line; while off it is not rendered and announces nothing. The typing field
  is described by the practice text region; and "Modo
  daltónico" (off by default: the keyboard flashes use the Okabe–Ito blue `#0072B2` and
  orange `#E69F00`, the same in every theme, and a wrong key's label also gets
  a bold underline).
  "Teclado": layout (see below), on-screen keyboard (`F2`), flashes and
  heatmap. "Apariencia": 59
  colour themes, dark and light, switched live (PlexusTerm's 57 plus "Alto
  contraste oscuro" and "Alto contraste claro", whose colours reach 7:1 for
  text and 4.5:1 for UI; every other theme meets WCAG AA), plus themes imported from
  VS Code (see below). "Texto", everything about how the practice text looks:
  "Cursor" (how the next char is marked: "Barra", the default,
  a thin caret line between chars like Monkeytype's; "Resaltado", a tint with an underline; "Bloque", "Subrayado" or
  "Contorno", in the error colour on a wrong char) and "Cursor parpadeante"
  (on by default: blinks while waiting and stays solid while typing; never with the system's reduced motion); "Atenuar texto pendiente" (on by default: the text
  still to type, prose and code, is dimmed to about 2.5:1 on its background,
  below AA, so the typed text stands out; off restores the AA colours; the
  high-contrast themes never dim); and the typography of the practice text, prose and
  code, applied live: "Fuente" (the typeface; default "Predeterminada", the
  font PlexusTyping always used; typing staples Roboto Mono, JetBrains Mono,
  Fira Code without ligatures, Source Code Pro, IBM Plex Mono and Inconsolata;
  for legibility Atkinson Hyperlegible Mono, Intel One Mono, Lexend Deca and
  OpenDyslexic; all bundled, none loaded from the network; each option is
  shown in its own font), "Tamaño de letra" (80–200 %, default 100),
  "Espaciado entre letras" (0–0.30 em, default 0) and "Interlineado"
  (1.2–2.4, default 1.75); a value outside its range is clamped. Everything is remembered
  locally; targets and length apply from the next text. `Escape`, `×` or a
  click outside closes the window and returns to the text. "IA" and
  "Metrónomo": see below.

### Adaptive loop

Type → measure → recalculate weaknesses → generate a text that attacks them.
Every completed session is replayed in Rust and folded into a weakness model:
each character and bigram («qu», «rr») keeps an exponential moving average of
errors (weight 0.7) and slowness relative to your own pace (weight 0.3). The
next text picks the 2 or 3 worst units, with some randomness, and builds sentences
from words that contain them ("Practicando: ñ · qu · z" above the text). Results
list your top weaknesses, and the optional "Mapa de calor" tints each key of
the on-screen keyboard by its score and marks it with its level, 1 to 5, so it
does not rely on colour alone; the legend under the keyboard names the levels
("Errores por tecla: 1 muy pocos, 2 pocos, 3 algunos, 4 bastantes, 5 muchos").

### Unlimited texts

"Longitud" → "Ilimitado" (prose and words; code ignores it and keeps its line
limit) gives a text that keeps growing while you type. It starts with a
600-character text from the usual source (AI, cache or corpus). When the cursor
reaches half of the text, or at most 400 characters remain ahead of it, a
continuation of about 600 characters is requested in the background, one at a
time, and appended after a single space without interrupting you; the next one
is asked at half of the new total (or 400 characters before its end), so there
is always text ahead. Prose continues with the AI from the last ~400 characters
(same language, only characters the layout and progressive unlock allow);
without the AI, or when it fails, a corpus text follows, with the reason as a
note above the text; a continuation without a note clears the previous
continuation's note. If a continuation cannot be had at all, a note says so
("No se pudo continuar el texto: …; reintentando…") and it is asked again: after
2.5 seconds near the end, else 50 characters later. Reaching the end of the
text never ends the session: it waits there with "Generando más texto…" and
goes on as soon as the continuation arrives. Words mode always continues with
more corpus words. Continuations are never cached, and only generated text is
sent, never what you type. `Esc` (from the text) or the "Terminar" button under
it ends the session: the text typed so far is scored as a completed session,
with the usual results, saving, statistics, daily goal and unlock progress, from
the keystrokes up to where the cursor first reached its final position (a wrong
key at the cursor or characters typed past it and erased are not counted);
ending before typing anything discards it and loads a new text. Unlimited texts race no ghost
session and the adaptive metronome does not change the tempo after them; the
metronome itself keeps playing. "Repetir" after ending repeats only the text you
typed, as a normal text that ends at its last character (and races the attempt
just saved as a ghost). The first diagnostic text is never unlimited.

### AI texts (optional)

In "Ajustes → IA", turn on "Usar IA para generar textos" and build the fallback
chain: up to five providers (Claude, OpenAI, Google Gemini, OpenCode Go and
ChatGPT (tu suscripción)), tried in order, each with its model (picked from the
provider's list, or typed by hand), its API key ("Guardar clave" / "Borrar
clave") and "Probar conexión". ChatGPT takes no key: it uses your signed-in
ChatGPT account (see below).
"Densidad" (2×–5×, default 3×) sets how much more often the targets appear in
code mode and
"Tema" an optional topic. Prose prompts only ask for a text in the language,
with its length range and topic: from 80 % to 125 % of "Longitud del texto"
(300 → 240–375 characters; local corpus texts keep their own range, the chosen
length plus a fifth). The text must fit the length, use only
characters the layout can type and contain no Markdown, but is not checked for
the targets. Code prompts also name the targets and allowed characters, and
each target must appear
`max(1, ceil(min(density × natural frequency, natural frequency + 5 %) × length))`
times, where the natural frequency is how often it occurs per character in the
bundled snippets, case-insensitively; the prompt states that count for the
longest allowed length. If a provider's text is rejected, its retry asks for
any text in the language, without targets; such a text practises no particular
unit. Every failed attempt (rejected text with its validation issues, timeout,
provider error) is logged to stderr, never the key. OpenCode Go reasoning
models are asked for their lowest reasoning effort so short practice texts
come back quickly. The usage table counts calls and tokens per provider,
today and in total, so you can watch your spending.

- **Keys** are stored only in the system keychain (Keychain on macOS,
  Credential Manager on Windows, Secret Service on Linux; service
  `com.plexustyping.app`, account `ai:<provider>`), never in plain text, never
  in the database, and never sent back to the interface. Without a working
  keychain no key can be saved.
- **Privacy:** only the language, the length and the topic are sent (in code
  mode also the target characters and the allowed characters). Nothing you type ever leaves
  this machine. Requests go from Rust to the provider hosts only.
- **Fallback:** each request has a 30 s timeout (60 s for the whole chain); a provider whose text fails
  validation gets one retry, then the next provider is tried. If all fail (or
  no key is set), the local corpus generates a new text, so a failure never
  repeats a text already shown (a cached AI text is only used if it was never
  shown; the texts the app shows are stored as shown). A short note above the
  text says why, and the
  source is always shown ("Generado con IA · Claude · …", "Texto del caché",
  "Corpus local").
- **Latency:** the next text is requested in the background as soon as you
  start typing the current one, so it is usually ready when you finish.

#### ChatGPT (tu suscripción)

This provider uses your ChatGPT plan instead of an API key, through OpenAI's
official "Sign in with ChatGPT" for open-source, locally run apps
(developers.openai.com/siwc), a preview programme of OpenAI's.

- **Sign in:** add "ChatGPT (tu suscripción)" to the chain in Ajustes → IA and
  press "Iniciar sesión con ChatGPT". Your browser opens OpenAI's sign-in page;
  the app shows "Esperando al navegador…" (with "Cancelar") until you finish
  there, then the account's email and whether plan use is enabled. Pick the
  model from the plan's list (shown by name). If you decline, cancel or take
  more than 5 minutes, nothing is saved and the reason is shown.
- **Usage counts against your ChatGPT plan**, under its own limits; "Gestionar
  uso" opens chatgpt.com/settings/usage. When the plan's limit is reached the
  notice reads "Has alcanzado el límite de tu plan de ChatGPT".
- **No silent switch to your keys:** a ChatGPT plan error (limit reached, plan
  use unavailable or not enabled, account not eligible, session rejected) stops
  the chain: the text comes from the cache or the local corpus with that
  notice, and your other providers' keys are never used instead. Only a network
  failure, a timeout or a malformed reply moves on to the next provider.
- **Sign out:** "Cerrar sesión" revokes the session at OpenAI and forgets the
  tokens on this machine; if OpenAI cannot confirm the revocation the app says
  "No se pudo confirmar la revocación" (the tokens are deleted anyway). After
  signing out you can sign in with the same or another ChatGPT account.
- **Privacy:** the sign-in tokens stay only in a private file on this machine
  (`chatgpt-auth.json` in the app config directory, owner-only, on Linux
  `~/.config/com.plexustyping.app/`), never in the database, the logs or the
  interface. The prompts (language, length, topic, and in code mode the
  targets and allowed characters) go to OpenAI, as with the other providers.

### Words mode

"Palabras" in the header switch practises loose words ("palabras sueltas"):
real words of the chosen language from the local word list, all lowercase,
separated by single spaces, with no punctuation and no capitals. At least half
of them contain your weak units (the same targets as prose), only characters
the chosen layout can type are used, and the length follows "Longitud del
texto". These texts always come from the local corpus, never from the AI, even
when it is enabled (no request, no cache). Words sessions feed the prose
statistics (same language and layout), so results, weaknesses and the heatmap
are the prose ones; the history shows them as "Palabras" (CSV `modo`
`palabras`). Changing the language reloads the text, as in prose.

### Code mode

The "Prosa / Palabras / Código" switch in the header (next to the gear) changes what you
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

### VS Code themes

"Ajustes → Apariencia → Importar tema de VS Code…" reads one VS Code colour
theme file (`.json` / `.jsonc`, comments and trailing commas allowed, up to
1 MB) in the WebView and adds it to the "Tema" selector under "Importados".
It is applied at once and kept in local storage; importing a theme with the
same name replaces the earlier one, and at most 30 are kept. "Eliminar tema
importado" removes the selected import and returns to the default theme.

- **Mapping:** `editor.background`, `editor.foreground` (both required),
  `editorCursor.foreground` and `editor.selectionBackground` give the base
  colours; `terminal.ansiBlack` … `terminal.ansiBrightWhite` give the 16 ANSI
  colours that colour the keyboard, the status colours and the code syntax.
  Each ANSI colour the file lacks comes from the built-in base scheme
  (PlexusTerm for dark themes, the first light scheme for light ones).
- **Dark or light:** from the file's `type`, else from the background.
- **Not read:** `tokenColors` and `include`; alpha in colours is dropped.

### Metronome

In "Ajustes → Metrónomo" (or with `F4` from anywhere), turn on the
metronome: 30–300 BPM (default 60; one click = one expected keystroke), sound
"Clic", "Madera" or "Bip", volume 0–100 % and an accent every N beats (0 = no
accent, up to 16). Changes apply live. It plays from Rust (`cpal`, the default
output device) only while a text is ready or being typed, and stops on the
results screen and when the window is hidden. A dot next to the live stats
flashes on each beat (accented beats in another colour).

- **Adaptive tempo:** with "Tempo adaptativo" on (off by default), each
  finished text that was typed entirely at one tempo retunes the "Tempo"
  setting for the next one: up one step when accuracy is at least 97 % and the
  mean absolute offset is at most 15 % of the beat interval, down one step when
  accuracy is under 90 % or the mean offset is over 20 % of the interval,
  unchanged otherwise. A step is 5 % of the tempo (at least 2 BPM), within
  30–300 BPM. Results show "Tempo: 60 → 63 BPM" or "Tempo: se mantiene en
  60 BPM". Changing the tempo by hand sets the new starting point.
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
"Historial" window. A filter "Todo / Prosa / Palabras / Código" picks the sessions shown,
and a switch "Sesiones / Por día / Por carácter" picks the view; every view
follows the filter. "Sesiones" (the default) shows:

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
  `modo` is `prose`, `palabras` or `code`, durations, WPM and `precision` (0–100) have one
  decimal, and `lenguaje_codigo` (prose) and `bpm` (no metronome) may be empty.
  With no sessions nothing is written and "No hay sesiones que exportar." is
  shown.

"Por día" groups the sessions by local day (the same local day as the daily
goal) and shows the last 30 days that have sessions: a chart of the day's mean
net WPM (line) and practised minutes (bars), oldest on the left, and a table
"Día · Sesiones · Minutos · PPM netas · Precisión", newest first. A day's WPM
and accuracy are means weighted by session duration; minutes are rounded down.

"Por carácter" uses the per-character statistics that drive the weak keys and
the heatmap ("Todo" merges prose and code by samples; "Palabras" shares the
prose statistics): a table "Carácter · Muestras · Error · Lentitud" of the
characters with at least 5 samples, worst first (by error, then slowness), and
a bar chart of the error of the 20 worst. Space shows as "espacio", Enter as
"Intro" and Tab as "Tab"; error is a percentage (`4,2 %`) and slowness the
stored relative value (0–1) with one decimal. Without enough keystrokes it
shows "Aún no hay bastantes pulsaciones por carácter.". The charts are plain
SVG with accessible titles; CSV export stays in "Sesiones".

Without saved sessions (or in temporary mode) every view shows "Aún no hay
sesiones guardadas.". `Escape`, `×` or a click outside closes it and returns
to the text.

### Interface language

The interface (menus, settings, notices, screen-reader announcements and the
backend's messages) is available in the same five languages as the practice
texts: Español, English, Português (Brasil), Français and Deutsch. On first
start it follows the computer's language (the first supported one among the
system's preferred languages, by base code: `pt-PT` → Português); when none is
supported it is English. "Ajustes → Apariencia → Idioma de la interfaz"
("Settings → Appearance → Interface language") changes it live: "Automático"
follows the computer again, any other choice is stored (`plexustyping.uiLanguage`)
and is the same for every profile. It is independent of the practice-text
language below: neither changes the other. Numbers, percentages and dates use
the interface language's format. Setting names quoted in this README are those
of the Spanish interface.

### Languages and keyboard layouts

"Ajustes → Práctica → Idioma" chooses the language of prose texts: Español
(default), English, Português (Brasil), Français or Deutsch. Each language has
its own offline corpus (5,000 frequent words, everyday sentences and a
diagnostic paragraph) and the AI is asked for texts in that language. The
sentences include up to 2,000 per language from Mozilla Common Voice (CC0),
filtered to plain text typeable on the language's layout. Changing
it loads a new text; each saved session records its language (history and the
CSV `idioma` column). Code practice is not affected.

"Ajustes → Teclado → Distribución" chooses one of eight built-in layouts:

| Layout | Id | Shape | Dead keys | Notes |
|---|---|---|---|---|
| Español (España) (default) | `es-qwerty` | ISO | `` ´ ¨ ^ ` `` | |
| Español (Latinoamérica) | `latam-qwerty` | ISO | `` ´ ¨ ^ ` `` | `@` on AltGr+Q, `\` on AltGr+`'`, `~` on AltGr+`+` |
| English (US) | `us-qwerty` | ANSI | none | no AltGr layer |
| Português (Brasil) | `pt-abnt2` | ABNT2 | `` ´ ` ^ ~ ¨ `` | `ç` key; extra `/ ?` key beside the right Shift |
| Français (France) | `fr-azerty` | ISO | `^ ¨` | digits on Shift; no `æ œ` or accented capitals |
| Deutsch (Deutschland) | `de-qwertz` | ISO | `` ^ ´ ` `` | `ß ä ö ü` keys |
| English (Dvorak) | `us-dvorak` | ANSI | none | US Dvorak; same characters as US |
| English (Colemak) | `us-colemak` | ANSI | none | US Colemak; same characters as US |

The layout drives the drawn keyboard (labels, next-key highlight, heatmap) and
the characters generated texts may contain: on the US layout Spanish texts
have no `ñ` or accents. A new profile starts with the diagnostic text of the
chosen language when the layout can type it. Changing the layout loads a new
text; each saved session records its layout (history and the CSV `layout`
column).

Limits: the keystrokes themselves come from the system keyboard layout, so the
layout chosen in the app must match the one set in the operating system. The
statistics of weak keys are one model per profile, shared by every language.
Corpus sources and licences: `THIRD_PARTY_NOTICES.md`.

### Profiles

Several people (or several goals) can share the app through local profiles.
The active profile's name is shown in the header; clicking it opens
"Ajustes → Perfiles", which lists every profile with the active one marked and
has "Usar" (switch), "Nuevo perfil", "Renombrar" and "Borrar". Names are 1–40
characters and unique ignoring case. A new profile starts with the active
profile's language and layout and does not become active until "Usar".

Per profile: the history, keystrokes and statistics (weak keys, heatmap), the
keyboard layout, the prose language and the theme. Everything else (error mode,
live stats, keyboard and heatmap toggles, text length, metronome, code options,
AI settings and imported VS Code themes) is shared by all profiles. Switching
profile applies its layout, language and theme, loads a new text and shows its
own weaknesses, heatmap and history; a profile without a theme keeps the
current one. On the first start with profiles, the layout, language and theme
chosen before are copied once to the active profile.

"Borrar" asks "Se borrarán el historial y las estadísticas de «nombre».
¿Continuar?" and then deletes that profile with all its sessions, keystrokes
and statistics; other profiles are untouched and it cannot be undone. The only
remaining profile cannot be deleted. Deleting the active profile switches to
the first remaining one.

### Daily goal and streak

Each profile can have its own daily goal, set in "Ajustes → Perfiles" under
"Meta diaria": "Desactivada" (the default), "Minutos al día" (5, 10, 15, 20,
30, 45, 60, 90 or 120) or "Textos al día" (1, 3, 5, 10, 15, 20 or 30). It
counts the active profile's saved sessions (prose, words and code) started on
the local day: minutes are the sum of their durations (shown rounded down;
met when the exact sum reaches the goal), texts are the number of sessions.
With a goal set, the header shows the progress next to the profile name, e.g.
"Hoy 12/20 min · racha 3" or "Hoy 2/5 textos", and "Meta cumplida · racha 4"
once met; the result screen of the session that meets it says "¡Meta diaria
cumplida!". The streak counts consecutive days on which the current goal was
met, ending today when today's goal is met and yesterday otherwise; a day
without sessions breaks it, and changing the goal applies the new one to past
days. Other profiles keep their own goal. In temporary mode the goal and the
progress last only until the app closes.

### Progressive unlock (beginner mode)

An optional beginner mode, off by default, set per profile in
"Ajustes → Práctica": "Desbloqueo progresivo" ("Empieza por la fila base y
añade una tecla cada vez que dominas las activas") and "Velocidad objetivo"
(20, 30, 40 or 50 PPM; 30 by default). It shapes prose texts only: words and
code modes ignore it. Progress is kept per profile, language and layout, and
survives turning the mode off and on.

Practice starts with the home row only: the letters the layout puts on the
A…L keys and the one after L (Spanish: a s d f g h j k l ñ); space is always
active and so is the uppercase of an active letter. A key counts as mastered
with at least 30 samples, an error rate (EMA) of 5 % or less and a median time
per keystroke within the target speed (12 000 / PPM ms: 400 ms at 30 PPM),
from the profile's prose statistics. After each saved prose session in this
mode one key is added when every active key is mastered, or after 3 sessions
in a row without a newly mastered key. The order is the home row (in Spanish
f j d k s l a ñ) and then the remaining letters by their frequency in the
language (Spanish: e o n r i t u c m p…); digits and punctuation unlock all
together once every letter is active.

With fewer than 15 letters there are no meaningful sentences, so texts are
lowercase words of active letters only, without AI: real words from the
language corpus first, completed with pronounceable pseudowords (alternating
consonant and vowel groups), with the new key and weak keys weighted up. From
15 letters on, texts come from the usual generation (AI when enabled, the
corpus otherwise) restricted to the active characters.

The on-screen keyboard dims the keys not unlocked yet, the line above the text
shows the progress ("Teclas: 9/27 · siguiente: e") and the result screen of
the session that adds a key says "Nueva tecla desbloqueada: e". In temporary
mode the progress lasts only until the app closes.

### Where data lives

Sessions, keystrokes and statistics are stored only on this machine, in
`plexustyping.sqlite3` under the app data directory (on Linux
`~/.local/share/com.plexustyping.app/`). If that file cannot be opened the app
still works with a temporary in-memory store and shows "Sin guardar: modo
temporal" in the header. The same database keeps the AI settings (without
keys), the cache of generated texts and the usage counters, and the profiles
(with their daily goals) and the active one. The ChatGPT sign-in lives apart,
in the private `chatgpt-auth.json` of the app config directory. The display and
practice settings (theme, keyboard, toggles, metronome, typography…) are kept
there too and written as soon as they change, so the development build and the
installed app share them and closing the app never loses the last change. On
the first start with this version they are copied once from the webview's local
storage, which is left untouched; in temporary mode they stay in local storage.

## Install

Each [GitHub release](https://github.com/jorgealonsodev/plexustyping/releases)
ships three 64-bit artefacts and a `SHA256SUMS` file. Check your download with
`sha256sum -c SHA256SUMS --ignore-missing` in the download folder.

### Linux: `.deb` (Debian, Ubuntu 22.04+ and derivatives)

```sh
sudo apt install ./plexustyping_<versión>_amd64.deb
```

`apt` also installs the system dependencies it declares (`libwebkit2gtk-4.1-0`,
`libgtk-3-0`), the menu entry and the icons. Update by installing the newer
`.deb` the same way; remove with `sudo apt remove plexus-typing` (the Debian package name Tauri derives from the product name).

### Linux: portable `.tar.gz` (any x86_64 distribution)

`plexustyping-<versión>-linux-x86_64.tar.gz` does not include WebKitGTK 4.1
or GTK 3; install them first:

| Distribution | Command |
| --- | --- |
| Debian / Ubuntu | `sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0` |
| Fedora | `sudo dnf install webkit2gtk4.1 gtk3` |
| Arch Linux | `sudo pacman -S webkit2gtk-4.1 gtk3` |

```sh
tar -xzf plexustyping-<versión>-linux-x86_64.tar.gz
cd plexustyping-<versión>
./plexustyping           # run it from here, or:
./install.sh             # install into ~/.local (no root)
./install.sh --uninstall # remove exactly what install.sh installed
```

`plexustyping` is a launcher: if WebKitGTK 4.1 is missing it prints the
package to install for Debian/Ubuntu, Fedora and Arch and exits with status 1;
otherwise it starts the real binary, `plexustyping-bin`. `install.sh` copies
both to `~/.local/bin`, the menu entry to `~/.local/share/applications` and the
icons to `~/.local/share/icons/hicolor`, and writes nothing outside
`~/.local`. Uninstalling keeps your data in
`~/.local/share/com.plexustyping.app/`. The `.tar.gz` does not update itself:
download the new one and run `./install.sh` again. The archive's `README`
(in Spanish) has the same instructions.

### Windows: `PlexusTyping_<versión>_x64-setup.exe`

Run the installer. It installs for the current user only, without
administrator rights, and downloads the WebView2 runtime if Windows does not
have it. The installer is not signed yet, so SmartScreen warns about an
unknown publisher: click "Más información" ("More info") → "Ejecutar de todas
formas" ("Run anyway"). Uninstall it from Settings → Apps.

## Layout

| Path | Contents |
| --- | --- |
| `crates/plexustyping-core` | Typing session engine: input replay, error modes, stats |
| `crates/plexustyping-layouts` | Keyboard layouts (TOML) and dead-key composition |
| `crates/plexustyping-corpus` | Local offline corpora (es, en, pt, fr, de) and AI-free text generator |
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
pnpm tauri build    # installers: .deb on Linux, NSIS .exe on Windows
scripts/package-linux-tarball.sh target/release/plexustyping 0.2.0 out  # .tar.gz
scripts/test-package-linux-tarball.sh  # test of the .tar.gz script
```

Pushing a `v*` tag (or running the Release workflow by hand) builds the
`.deb`, the `.tar.gz`, the `.exe` and `SHA256SUMS` in GitHub Actions
(`.github/workflows/release.yml`); on a tag they are attached to a draft
release, which a maintainer publishes.

## License

Code: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
Bundled data keeps its own license; see `THIRD_PARTY_NOTICES.md`.
