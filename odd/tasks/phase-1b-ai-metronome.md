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

## Tasks

| ID | Specs | Route | Status | Commit |
| --- | --- | --- | --- | --- |
| T1 | S7 | New crate `plexustyping-audio`: synthesized sounds, fixed-BPM scheduler on cpal, accent, volume — writer A (parallel) | pending | — |
| T2 | S1, S3, S4, S5 | New crate `plexustyping-ai`: `TextProvider` + 4 providers, prompt, JSON parse, validation, fallback chain, usage — writer B (parallel) | pending | — |
| T3 | S2, S3, S6 | Integration: keychain, provider settings UI + test connection, schema v2 `text_cache` + usage, background prefetch, fallback to corpus — writer C | pending | — |
| T4 | S7 | Integration: metronome commands, UI controls, visual beat, per-keystroke offset stored — writer D | pending | — |
| T5 | all | Close: suite, real-app check, verify (credentials, network), Trello | pending | — |

## Log

- **L1 (user, verbatim):** "ya puedes usar sudo" → installed `libasound2-dev` 1.2.11 with `sudo apt-get install`. API keys question unanswered → providers built and tested against local mock HTTP servers; real-provider test pending a user key.
- **L2 (env):** `dbus-1` 1.14.10 dev present (keyring Secret Service). MSRV 1.85: rodio 0.23 needs 1.87 → use `cpal` (0.18, MSRV 1.85) directly; reqwest 0.13 (1.85), keyring 3.6 (1.75), async-trait.
- **L3 (decisions):** network calls only from Rust (`reqwest` + rustls), never the WebView; host allowlist = api.anthropic.com, api.openai.com, generativelanguage.googleapis.com, opencode.ai. Default models are editable suggestions, overridden by `list_models` where available. Schema v1 already exists on the user's machine → `text_cache` and usage counters come as a proper v2 migration.
