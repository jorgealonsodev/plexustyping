# Feature: PlexusTyping — Phase 1B (AI providers, keychain, fixed metronome)

Source: `PRD-PlexusTyping.v0.1.md` phase 1 (MVP), second block. Branch: `feat/phase-1b-ai-metronome` (stacked on `feat/phase-1-adaptive`).
Delivery: `chain_strategy=feature-branch-chain`. Forecast: ~2,000 authored lines. Closes the PRD phase 1 MVP.

## Specs

- **S1 — Providers.** PRD RF2 table: "Claude (Anthropic) | Messages API | Modelo rápido por defecto (gama Haiku)"; "OpenAI | Chat Completions / Responses | Modelo mini por defecto"; "Google Gemini | Gemini API (`generateContent`) | Modelo Flash por defecto"; "OpenCode Go | Compatible con OpenAI, base `https://opencode.ai/zen/go/v1` | Suscripción con modelos abiertos; lista de modelos vía `/v1/models`". "Un trait `TextProvider` común; el usuario elige proveedor y modelo, y puede probar la conexión."
- **S2 — Keys.** RF2: "Claves guardadas en el llavero del sistema (Keychain, Credential Manager, Secret Service), nunca en texto plano." §6: "Claves en el llavero del sistema; CSP estricta en Tauri; sin acceso a red salvo los 4 proveedores".
- **S3 — Output, timeout, fallback, cache, usage.** RF2: "Salida en JSON estructurado `{ \"text\": \"...\" }`; timeout de 10 s y cadena de respaldo configurable (p. ej. OpenCode Go → Gemini → corpus local)." / "Caché local de textos generados para reutilizarlos sin conexión." / "Contador de llamadas y tokens por proveedor para que el usuario controle su gasto."
- **S4 — Prompt and validation.** PRD §4.5: "Se pide a la IA un texto donde esos caracteres tengan alta densidad (por defecto 3× su frecuencia normal), con tema, longitud e idioma elegidos." §4.6: "Rust comprueba que el texto cumple la densidad, solo usa caracteres del layout y no trae formato raro. Si falla, reintenta una vez o cae al corpus local." Prompt example (§4): "Eres un generador de textos para practicar mecanografía. / Idioma: español. Longitud: 250–300 caracteres. Tema: viajes. / Caracteres objetivo: ñ, z, \"ci\". Cada uno debe aparecer al menos 3 veces más / de lo habitual. Usa solo estos caracteres: [lista del layout]. / Sin listas, sin markdown, sin comillas tipográficas. / Responde solo con JSON: {\"text\": \"...\"}". RF1: "densidad (2×–5×)".
- **S5 — Privacy.** §6: "a la IA solo se envían caracteres objetivo, idioma y tema, nunca lo que escribe el usuario".
- **S6 — Latency.** §2: "Latencia hasta mostrar el siguiente texto | < 2 s (precarga en segundo plano)". §4: "El siguiente texto se pide en segundo plano mientras el usuario escribe el actual, para no esperar."
- **S7 — Fixed metronome.** RF6: "**Fijo:** de 30 a 300 BPM, un clic = una pulsación esperada." / "Sonidos a elegir (clic, madera, bip), volumen y acento cada N pulsaciones." / "Indicador visual de ritmo y métrica de desvío: cuántos ms antes o después del clic cae cada tecla." / "Audio con baja latencia desde Rust (`rodio` o `cpal`), no desde la web." Data model: `keystrokes.desvío_metrónomo_ms`, `sessions.BPM`. (Adaptive metronome is phase 3.)
- **S8 — Settings modal.** User: "hay que crear un icono de ajustes que abra una modal de ajustes".

## Tasks

| ID | Specs | Route | Status | Commit |
| --- | --- | --- | --- | --- |
| T1 | S7 | New crate `plexustyping-audio`: synthesized sounds, fixed-BPM scheduler on cpal, accent, volume — writer A (parallel) | done | 7968f14 |
| T2 | S1, S3, S4, S5 | New crate `plexustyping-ai`: `TextProvider` + 4 providers, prompt, JSON parse, validation, fallback chain, usage — writer B (parallel) | done | 7957ea2 |
| T3 | S2, S3, S6 | Integration: keychain, provider settings UI + test connection, schema v2 `text_cache` + usage, background prefetch, fallback to corpus — writer C | pending | — |
| T4 | S7 | Integration: metronome commands, UI controls, visual beat, per-keystroke offset stored — writer D | pending | — |
| T6 | S8 | Settings modal: gear icon in the header opens a modal with sectioned settings (Práctica, Teclado, Apariencia; IA and Metrónomo sections added by T3/T4) — writer E (parallel with T2) | done | 720cedd |
| T5 | all | Close: suite, real-app check, verify (credentials, network), Trello | pending | — |

## Log

- **L1 (user, verbatim):** "ya puedes usar sudo" → installed `libasound2-dev` 1.2.11 with `sudo apt-get install`. API keys question unanswered → providers built and tested against local mock HTTP servers; real-provider test pending a user key.
- **L2 (env):** `dbus-1` 1.14.10 dev present (keyring Secret Service). MSRV 1.85: rodio 0.23 needs 1.87 → use `cpal` (0.18, MSRV 1.85) directly; reqwest 0.13 (1.85), keyring 3.6 (1.75), async-trait.
- **L3 (decisions):** network calls only from Rust (`reqwest` + rustls), never the WebView; host allowlist = api.anthropic.com, api.openai.com, generativelanguage.googleapis.com, opencode.ai. Default models are editable suggestions, overridden by `list_models` where available. Schema v1 already exists on the user's machine → `text_cache` and usage counters come as a proper v2 migration.
- **L4 (T1 evidence):** `plexustyping-audio` (cpal 0.18 on its own thread, atomics config, allocation-free callback, sample-accurate `BeatClock`); 12 tests + 1 ignored device test (passed on this host). `started_at` excludes device output latency (a few ms).
- **L5 (user, verbatim):** "mira como implementa la ia /mnt/1tb/Repos/Personal/20261019_plexusterm/20261019_plexusterm_git/PlexusTerm igual hay cosas que te sirven" (first T2 writer cancelled by the user; then "continua, le di esc sin querer"). Explorer findings adopted: request = plain `Call {url, headers, body, shape}` + `parse(shape)` instead of per-provider HTTP code; error kinds NoKey/Auth/RateLimit/Network/Timeout/Refused/BadResponse/Config with key masking; redirects disabled; redacting `ApiKey` Debug; native JSON-schema output per provider (Anthropic `output_config.format`, OpenAI strict `json_schema` + `max_completion_tokens`, Gemini `responseSchema`); Gemini bad key = 400 `API_KEY_INVALID` → Auth; keychain behind a `SecretStore` trait called via `spawn_blocking`; hand-written loopback mock server. **OpenCode Go inference base is `https://opencode.ai/inference/go/v1`** (PlexusTerm working code + its user guide), not the PRD's `/zen/go/v1`; per-model format (chat / messages / responses) from models.dev `["opencode-go"].models[id].provider.npm`, every request with `x-opencode-session`. MSRV 1.85: no let-chains.
- **L6 (user, verbatim):** "hay que crear un icono de ajustes que abra una modal de ajustes" → S8/T6; replaces the header selects and the "Ajustes" popover from phase 1A.
- **L7 (T2/T6 evidence):** T2 `7957ea2`: `plexustyping-ai` 65 tests (43 unit + 6 fallback + 16 providers, loopback mock), also passes on Rust 1.85; contacts models.dev for the OpenCode Go format catalogue (allowlist = 4 providers + models.dev); shapes unverified against live endpoints (no key yet); BASELINE_PER_CHAR 0.02 may be strict. T6 `720cedd`: gear icon + native `<dialog>` modal, sections registry (Práctica, Teclado, Apariencia), Ctrl+,; 89 vitest. Real-app check at 720×480: modal fits, opens from the gear, Escape closes it without restarting the text.
- **L8 (decision):** raise workspace `rust-version` 1.85 → 1.88 so T3 can use `keyring` 4.2 + `keyring-core` 1.0 exactly as PlexusTerm (keyring 4.2 needs 1.88; toolchain is 1.98; project unpublished).
