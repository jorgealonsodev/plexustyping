//! AI wiring through the plain functions: in-memory store, in-memory keychain
//! and a loopback mock provider. No real provider or keychain is touched.

use std::sync::{Arc, Mutex};

use plexustyping_ai::{AiClient, Endpoints, ProviderError, ProviderId};
use plexustyping_core::{ErrorMode, InputEvent};
use plexustyping_store::{StatsSnapshot, Store, UnitStatRow};
use serde_json::json;

use super::mock::{Mock, mock, mock_with};
use super::*;
use crate::code::{DEFAULT_MAX_LINES, PracticeMode};
use crate::commands::{STATS_MODE, StorageStatus, score_session};
use crate::secrets::{MemoryStore, NoStore, SecretStore};

const KEY: &str = "sk-test-SECRET-123";
const DAY: &str = "2026-11-16";
const OPENAI: &str = "/v1/chat/completions";
const GEMINI: &str = "/v1beta/models/gemini-3.5-flash-lite:generateContent";
/// 54 chars, 5 × ñ: valid for target ñ at 3× over 50..61 chars.
const AI_TEXT: &str = "La niña y el niño comen ñame con ñoquis en la montaña.";

fn state(secrets: SharedSecrets, ai: AiClient) -> AppState {
    let store = Store::open_in_memory().unwrap();
    let profile = store.default_profile().unwrap();
    AppState {
        store: Mutex::new(store),
        profile: Mutex::new(profile),
        status: StorageStatus {
            persistent: false,
            error: None,
        },
        ai,
        secrets,
    }
}

fn offline() -> AiClient {
    AiClient::new(Endpoints::loopback("http://127.0.0.1:9"))
}

/// The model practises only ñ.
fn weak_in_n(state: &AppState) {
    let snapshot = StatsSnapshot {
        units: vec![UnitStatRow {
            unit: "ñ".into(),
            ema_error: 0.9,
            ema_slowness: 0.0,
            samples: 5,
            slowness_samples: 0,
        }],
        confusions: vec![],
        mean_interval_ms: Some(150.0),
    };
    let mut store = lock_store(&state.store);
    store
        .save_stats(state.active_profile().id, STATS_MODE, &snapshot)
        .unwrap();
}

fn enable(state: &AppState, chain: &[(ProviderId, &str)]) {
    let settings = AiSettings {
        enabled: true,
        chain: chain
            .iter()
            .map(|&(provider, model)| ChainEntry {
                provider,
                model: model.into(),
            })
            .collect(),
        density: 3.0,
        topic: Some("comida".into()),
    };
    save_settings(&lock_store(&state.store), settings).unwrap();
}

async fn with_key(secrets: &Arc<MemoryStore>, id: ProviderId) {
    let shared: SharedSecrets = secrets.clone();
    key_save(&shared, id, KEY.into()).await.unwrap();
}

/// One char event per typed char, 100 ms apart.
fn typing(typed: &str) -> Vec<InputEvent> {
    let events = typed
        .chars()
        .enumerate()
        .map(|(i, ch)| json!({ "kind": "char", "ch": ch.to_string(), "tMs": i as f64 * 100.0 }));
    serde_json::from_value(events.collect()).unwrap()
}

fn params(seed: u64) -> TextParams {
    TextParams {
        language: "es".into(),
        layout: "es-qwerty".into(),
        seed,
        target_count: 3,
        length: 50,
        fallback: vec![],
        day: DAY.into(),
        mode: PracticeMode::Prose,
        max_lines: DEFAULT_MAX_LINES,
    }
}

/// The corpus text the same request gets with the AI off.
fn corpus_expected(state: &AppState, seed: u64) -> NextText {
    let store = lock_store(&state.store);
    let model = commands::load_model(&store, &state.active_profile()).unwrap();
    let p = params(seed);
    let req = NextTextRequest {
        language: &p.language,
        layout: &p.layout,
        seed,
        target_count: p.target_count,
        length: p.length,
        fallback: &p.fallback,
    };
    commands::next_text(&model, &req).unwrap()
}

fn chat_reply(text: &str) -> String {
    json!({
        "choices": [{"message": {"role": "assistant", "content": json!({"text": text}).to_string()}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 80, "completion_tokens": 30}
    })
    .to_string()
}

fn gemini_reply(text: &str) -> String {
    json!({
        "candidates": [{"content": {"parts": [{"text": json!({"text": text}).to_string()}]}, "finishReason": "STOP"}],
        "usageMetadata": {"promptTokenCount": 11, "candidatesTokenCount": 7}
    })
    .to_string()
}

async fn server(routes: Vec<(&str, u16, String)>) -> Mock {
    mock(routes).await
}

fn usage_of(state: &AppState) -> Vec<UsageRow> {
    usage(&lock_store(&state.store), DAY).unwrap()
}

// ---- settings ----

#[test]
fn settings_default_to_disabled_with_one_claude_entry() {
    let store = Store::open_in_memory().unwrap();
    let s = load_settings(&store);
    assert_eq!(s, AiSettings::default());
    assert!(!s.enabled);
    assert_eq!(s.density, 3.0);
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({"enabled": false, "chain": [{"provider": "claude", "model": "claude-haiku-5-5"}], "density": 3.0, "topic": null})
    );
}

#[test]
fn save_settings_normalizes_and_round_trips() {
    let store = Store::open_in_memory().unwrap();
    let input: AiSettings = serde_json::from_value(json!({
        "enabled": true,
        "chain": [{"provider": "opencode-go", "model": "  "}, {"provider": "gemini", "model": " gemini-x "}],
        "density": 2.0,
        "topic": "  viajes "
    }))
    .unwrap();
    let saved = save_settings(&store, input).unwrap();
    assert_eq!(saved.chain[0].model, "");
    assert_eq!(saved.chain[1].model, "gemini-x");
    assert_eq!(saved.topic.as_deref(), Some("viajes"));
    assert_eq!(load_settings(&store), saved);
    let blank = AiSettings {
        topic: Some("   ".into()),
        ..AiSettings::default()
    };
    assert_eq!(save_settings(&store, blank).unwrap().topic, None);
}

#[test]
fn invalid_settings_are_rejected_and_keep_the_stored_ones() {
    let store = Store::open_in_memory().unwrap();
    let good = save_settings(
        &store,
        AiSettings {
            enabled: true,
            ..AiSettings::default()
        },
    )
    .unwrap();
    let entry = |provider| ChainEntry {
        provider,
        model: "m".into(),
    };
    let base = AiSettings::default;
    let cases = [
        (
            AiSettings {
                chain: vec![],
                ..base()
            },
            "La cadena de respaldo necesita de 1 a 5 proveedores",
        ),
        (
            AiSettings {
                chain: [ProviderId::ALL.to_vec(), vec![ProviderId::Claude]]
                    .concat()
                    .into_iter()
                    .map(entry)
                    .collect(),
                ..base()
            },
            "La cadena de respaldo necesita de 1 a 5 proveedores",
        ),
        (
            AiSettings {
                chain: vec![entry(ProviderId::Gemini), entry(ProviderId::Gemini)],
                ..base()
            },
            "Gemini aparece dos veces en la cadena de respaldo",
        ),
        (
            AiSettings {
                density: 1.9,
                ..base()
            },
            "La densidad debe estar entre 2× y 5×",
        ),
        (
            AiSettings {
                density: 5.1,
                ..base()
            },
            "La densidad debe estar entre 2× y 5×",
        ),
        (
            AiSettings {
                density: f32::NAN,
                ..base()
            },
            "La densidad debe estar entre 2× y 5×",
        ),
        (
            AiSettings {
                topic: Some("a".repeat(61)),
                ..base()
            },
            "El tema admite como máximo 60 caracteres",
        ),
        (
            AiSettings {
                topic: Some("mar\nmontaña".into()),
                ..base()
            },
            "El tema no puede tener saltos de línea",
        ),
        (
            AiSettings {
                chain: vec![ChainEntry {
                    provider: ProviderId::Claude,
                    model: "m".repeat(201),
                }],
                ..base()
            },
            "El nombre del modelo es demasiado largo",
        ),
    ];
    for (bad, message) in cases {
        let err = save_settings(&store, bad.clone()).unwrap_err();
        assert_eq!(err, AiFailure::settings(message), "{bad:?}");
        assert_eq!(load_settings(&store), good);
    }
    // Limits themselves are valid.
    let edge = AiSettings {
        chain: ProviderId::ALL.into_iter().map(entry).collect(),
        density: 5.0,
        topic: Some("ñ".repeat(60)),
        ..base()
    };
    assert_eq!(save_settings(&store, edge.clone()).unwrap(), edge);
}

#[test]
fn unreadable_stored_settings_fall_back_to_the_defaults() {
    let store = Store::open_in_memory().unwrap();
    for stored in [
        "not json",
        r#"{"enabled":true,"chain":[],"density":3,"topic":null}"#,
    ] {
        store.set_setting(SETTINGS_KEY, stored).unwrap();
        assert_eq!(load_settings(&store), AiSettings::default(), "{stored}");
    }
}

#[test]
fn providers_list_ids_names_and_default_models() {
    let list = providers();
    assert_eq!(list.len(), 5);
    assert_eq!(
        serde_json::to_value(&list[3]).unwrap(),
        json!({"id": "opencode-go", "label": "OpenCode Go", "name": "OpenCode Go", "defaultModel": ""})
    );
    assert_eq!(
        serde_json::to_value(&list[4]).unwrap(),
        json!({"id": "chatgpt", "label": "ChatGPT (tu suscripción)", "name": "ChatGPT", "defaultModel": ""})
    );
    for p in list {
        assert_eq!(json!(slug(p.id)), serde_json::to_value(p.id).unwrap());
        assert_eq!(account(p.id), format!("ai:{}", slug(p.id)));
    }
}

#[test]
fn provider_errors_map_to_spanish_reasons_in_one_place() {
    let m = || "detail with sk-1".to_owned();
    let cases = [
        (ProviderError::NoKey, "Falta la clave de Gemini"),
        (
            ProviderError::Auth { message: m() },
            "Gemini rechazó la clave",
        ),
        (
            ProviderError::RateLimit { message: m() },
            "Gemini está limitando las peticiones",
        ),
        (
            ProviderError::Network { message: m() },
            "Sin conexión con la IA",
        ),
        (ProviderError::Timeout, "Gemini no respondió a tiempo"),
        (
            ProviderError::Refused { message: m() },
            "Gemini se negó a generar el texto",
        ),
        (
            ProviderError::BadResponse { message: m() },
            "La IA no devolvió un texto válido",
        ),
        (
            ProviderError::Invalid { issues: vec![] },
            "La IA no devolvió un texto válido",
        ),
        (
            ProviderError::Config { message: m() },
            "Gemini rechazó la petición (revisa el modelo)",
        ),
    ];
    for (err, reason) in cases {
        assert_eq!(provider_reason(ProviderId::Gemini, &err), reason);
        assert_eq!(provider_failure(ProviderId::Gemini, &err).message, reason);
    }
}

#[test]
fn usage_day_keeps_a_local_day_and_falls_back_to_utc() {
    assert_eq!(usage_day("2026-11-16"), "2026-11-16");
    assert_eq!(civil_date(0), "1970-01-01");
    assert_eq!(civil_date(20_773), "2026-11-16");
    assert_eq!(civil_date(11_016), "2000-02-29");
    for bad in ["", "2026-1-16", "2026/11/16", "'; DROP--x", "2026-11-16 "] {
        let day = usage_day(bad);
        assert_ne!(day, bad);
        assert_eq!(usage_day(&day), day, "a UTC YYYY-MM-DD for {bad:?}");
    }
}

// ---- keys ----

#[tokio::test]
async fn keys_are_saved_trimmed_in_the_keychain_and_only_their_status_returns() {
    let memory = MemoryStore::shared();
    let secrets: SharedSecrets = memory.clone();
    let id = ProviderId::OpencodeGo;
    assert_eq!(key_status(&offline(), &secrets, id).await, Ok(false));
    key_save(&secrets, id, format!("  {KEY}\n")).await.unwrap();
    assert_eq!(memory.get("ai:opencode-go").unwrap().as_deref(), Some(KEY));
    assert_eq!(
        memory.label("ai:opencode-go").as_deref(),
        Some("PlexusTyping AI key (OpenCode Go)")
    );
    let status = key_status(&offline(), &secrets, id).await;
    assert_eq!(status, Ok(true));
    assert_eq!(serde_json::to_string(&status.unwrap()).unwrap(), "true");
    assert_eq!(
        key_status(&offline(), &secrets, ProviderId::Claude).await,
        Ok(false)
    );
    key_delete(&secrets, id).await.unwrap();
    key_delete(&secrets, id).await.unwrap();
    assert_eq!(key_status(&offline(), &secrets, id).await, Ok(false));
}

#[tokio::test]
async fn an_empty_key_is_rejected_and_nothing_is_stored() {
    let memory = MemoryStore::shared();
    let secrets: SharedSecrets = memory.clone();
    with_key(&memory, ProviderId::Claude).await;
    let err = key_save(&secrets, ProviderId::Claude, " \n ".into()).await;
    assert_eq!(err, Err(AiFailure::new("noKey", "La clave está vacía")));
    assert_eq!(memory.get("ai:claude").unwrap().as_deref(), Some(KEY));
}

#[tokio::test]
async fn an_unavailable_keychain_is_an_error_never_a_plain_text_fallback() {
    let secrets: SharedSecrets = Arc::new(NoStore);
    let unavailable = Err(AiFailure::new("keychain", KEYCHAIN_UNAVAILABLE));
    assert_eq!(
        key_status(&offline(), &secrets, ProviderId::Claude).await,
        unavailable
    );
    assert_eq!(
        key_save(&secrets, ProviderId::Claude, KEY.into()).await,
        unavailable.clone().map(|_| ())
    );
    assert_eq!(
        key_delete(&secrets, ProviderId::Claude).await,
        unavailable.map(|_| ())
    );
}

// ---- models and test connection ----

#[tokio::test]
async fn models_use_the_stored_key_and_opencode_go_needs_none() {
    let srv = server(vec![
        (
            "/inference/go/v1/models",
            200,
            json!({"data": [{"id": "kimi-k2"}]}).to_string(),
        ),
        (
            "/v1/models",
            200,
            json!({"data": [{"id": "gpt-5-mini"}]}).to_string(),
        ),
    ])
    .await;
    let memory = MemoryStore::shared();
    let secrets: SharedSecrets = memory.clone();
    let client = srv.client();
    let got = models(&client, &secrets, ProviderId::OpencodeGo).await;
    assert_eq!(got, Ok(vec!["kimi-k2".to_owned()]));
    let no_key = models(&client, &secrets, ProviderId::Openai).await;
    assert_eq!(
        no_key,
        Err(AiFailure::new("noKey", "Falta la clave de OpenAI"))
    );
    assert_eq!(srv.requests().len(), 1, "no request without a key");
    with_key(&memory, ProviderId::Openai).await;
    let got = models(&client, &secrets, ProviderId::Openai).await;
    assert_eq!(got, Ok(vec!["gpt-5-mini".to_owned()]));
}

#[tokio::test]
async fn test_connection_reports_ok_or_a_typed_failure_without_the_key() {
    let srv = server(vec![
        ("/v1/models", 200, json!({"data": []}).to_string()),
        (
            "/v1/models",
            401,
            json!({"error": {"message": format!("bad key {KEY}")}}).to_string(),
        ),
    ])
    .await;
    let memory = MemoryStore::shared();
    let secrets: SharedSecrets = memory.clone();
    let client = srv.client();
    assert_eq!(
        test_connection(&client, &secrets, ProviderId::Openai, "").await,
        Err(AiFailure::new("noKey", "Falta la clave de OpenAI"))
    );
    with_key(&memory, ProviderId::Openai).await;
    assert_eq!(
        test_connection(&client, &secrets, ProviderId::Openai, "gpt-5-mini").await,
        Ok(())
    );
    let failed = test_connection(&client, &secrets, ProviderId::Openai, "gpt-5-mini").await;
    let failed = failed.unwrap_err();
    assert_eq!(failed, AiFailure::new("auth", "OpenAI rechazó la clave"));
    assert!(!serde_json::to_string(&failed).unwrap().contains(KEY));
}

// ---- next_text ----

#[tokio::test]
async fn disabled_ai_gives_the_corpus_text_without_any_request() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got, corpus_expected(&state, 4));
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(got.notice, None);
    assert!(srv.requests().is_empty());
    assert_eq!(
        serde_json::to_value(&got).unwrap()["source"],
        json!("corpus")
    );
}

/// Caches an unused `text` of language es and `mode` with no targets, created at `at`.
fn cache_unused(state: &AppState, mode: &str, text: &str, at: i64) {
    lock_store(&state.store)
        .cache_text(&NewCachedText {
            provider: "claude",
            model: "claude-haiku-5-5",
            language: "es",
            mode,
            targets: &[],
            text,
            created_at_ms: at,
            used: false,
        })
        .unwrap();
}

#[tokio::test]
async fn disabled_ai_rotates_the_cached_texts_without_any_request() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    cache_unused(&state, "prose", "primer texto guardado", 1);
    cache_unused(&state, "prose", "segundo texto guardado", 2);
    cache_unused(&state, "code:rust", "let x = 1;", 3);
    let mut shown = Vec::new();
    for seed in 0..4 {
        let got = next_text(&state, &params(seed)).await.unwrap();
        assert_eq!(got.source, TextSource::Cache);
        assert_eq!(got.notice, None);
        assert_eq!(got.provider.as_deref(), Some("claude"));
        assert_eq!((got.syntax.clone(), got.tokens.clone()), (None, None));
        shown.push(got.text);
    }
    assert_eq!(
        shown,
        [
            "primer texto guardado",
            "segundo texto guardado",
            "primer texto guardado",
            "segundo texto guardado"
        ]
    );
    assert!(srv.requests().is_empty());
    assert!(usage_of(&state).is_empty());
}

#[tokio::test]
async fn disabled_ai_in_code_mode_rotates_its_own_language_cache_then_the_corpus() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_CODE))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_brace(&state);
    cache_unused(&state, "prose", "texto de prosa", 1);
    cache_unused(&state, "code:python", "print(1)", 2);
    // No rust text yet: the corpus snippet, without a notice.
    let got = next_text(&state, &code_params(4)).await.unwrap();
    assert_eq!((got.source, got.notice.clone()), (TextSource::Corpus, None));
    assert_eq!(got.syntax.as_deref(), Some("rust"));
    cache_unused(&state, "code:rust", "let x = 1;", 3);
    cache_unused(&state, "code:rust", "let y = 2;", 4);
    let mut shown = Vec::new();
    for seed in 0..3 {
        let got = next_text(&state, &code_params(seed)).await.unwrap();
        assert_eq!((got.source, got.notice.clone()), (TextSource::Cache, None));
        assert_eq!(got.syntax.as_deref(), Some("rust"));
        assert_contiguous(&got);
        shown.push(got.text);
    }
    assert_eq!(shown, ["let x = 1;", "let y = 2;", "let x = 1;"]);
    assert!(srv.requests().is_empty());
}

#[tokio::test]
async fn a_prose_ai_text_only_lists_the_targets_it_really_contains() {
    // Prose no longer asks for targets (S9), so "Practicando" must not claim them.
    const NO_N: &str = "La casa del lago tiene una puerta roja y dos ventanas.";
    let srv = server(vec![(OPENAI, 200, chat_reply(NO_N))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!((got.source, got.text.as_str()), (TextSource::Ai, NO_N));
    assert!(got.targets.is_empty(), "{:?}", got.targets);
}

#[tokio::test]
async fn a_prose_ai_text_lists_a_target_it_only_contains_in_uppercase() {
    const UPPER: &str = "ÑANDÚ Y ÑU corren por la llanura verde del sur hoy.";
    let srv = server(vec![(OPENAI, 200, chat_reply(UPPER))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(
        (got.source, got.targets),
        (TextSource::Ai, vec!["ñ".to_owned()])
    );
}

#[tokio::test]
async fn ai_success_is_returned_cached_as_used_and_counted() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(
        got,
        NextText {
            text: AI_TEXT.into(),
            targets: vec!["ñ".into()],
            source: TextSource::Ai,
            provider: Some("openai".into()),
            model: Some("gpt-5-mini".into()),
            notice: None,
            syntax: None,
            tokens: None,
        }
    );
    assert_eq!(
        serde_json::to_value(&got).unwrap(),
        json!({"text": AI_TEXT, "targets": ["ñ"], "source": "ai", "provider": "openai", "model": "gpt-5-mini"})
    );
    let usage = usage_of(&state);
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].provider, "openai");
    assert_eq!(
        (
            usage[0].calls_today,
            usage[0].input_tokens_today,
            usage[0].output_tokens_today
        ),
        (1, 80, 30)
    );
    // Cached as already used: an unused text wins even without shared targets,
    // then the AI text comes back as the least recently used one.
    let store = lock_store(&state.store);
    store
        .cache_text(&NewCachedText {
            provider: "gemini",
            model: "g",
            language: "es",
            mode: "prose",
            targets: &[],
            text: "otro texto",
            created_at_ms: 1,
            used: false,
        })
        .unwrap();
    let ñ = ["ñ".to_owned()];
    let take = || store.take_cached_text("es", "prose", &ñ).unwrap().unwrap();
    assert_eq!(take().text, "otro texto");
    let cached = take();
    assert_eq!(
        (cached.text.as_str(), cached.targets),
        (AI_TEXT, ñ.to_vec())
    );
    assert_eq!(
        (cached.provider.as_str(), cached.model.as_str()),
        ("openai", "gpt-5-mini")
    );
}

#[tokio::test]
async fn the_prose_request_only_asks_for_a_text_in_the_language_and_nothing_typed() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    {
        // The user typed "hqlz mxndo" for "hola mundo".
        let mut store = lock_store(&state.store);
        let events = typing("hqlz mxndo");
        let scored = score_session(
            &mut store,
            &state.active_profile(),
            "hola mundo",
            ErrorMode::AllowContinue,
            &events,
            1,
        );
        assert!(scored.saved);
    }
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    next_text(&state, &params(4)).await.unwrap();
    let requests = srv.requests();
    assert_eq!(requests.len(), 1);
    let body = &requests[0].body;
    let sent: serde_json::Value = serde_json::from_str(body).unwrap();
    let prompt = sent["messages"][0]["content"].as_str().unwrap();
    // "dile que genere solo un texto en mi idioma" (S9): language, length and topic only.
    assert!(
        prompt.starts_with("Escribe un texto en español de 40–62 caracteres.\nTema: comida.\n"),
        "{prompt}"
    );
    for absent in ["Caracteres objetivo", "Usa solo"] {
        assert!(!prompt.contains(absent), "{absent:?} in {prompt}");
    }
    for typed in ["hqlz", "mxndo", "hola mundo"] {
        assert!(!body.contains(typed), "typed text {typed:?} leaked: {body}");
    }
}

#[tokio::test]
async fn the_chain_falls_back_to_the_next_provider_in_order() {
    let srv = server(vec![
        (OPENAI, 500, "{}".into()),
        (GEMINI, 200, gemini_reply(AI_TEXT)),
    ])
    .await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    with_key(&memory, ProviderId::Gemini).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(
        &state,
        &[
            (ProviderId::Openai, "gpt-5-mini"),
            (ProviderId::Gemini, "gemini-3.5-flash-lite"),
        ],
    );
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got.source, TextSource::Ai);
    assert_eq!(got.provider.as_deref(), Some("gemini"));
    let paths: Vec<String> = srv.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, [OPENAI, GEMINI]);
    let usage = usage_of(&state);
    assert_eq!(usage.len(), 1, "only the successful call is counted");
    assert_eq!(
        (usage[0].provider.as_str(), usage[0].input_tokens_total),
        ("gemini", 11)
    );
}

#[tokio::test]
async fn an_ai_failure_uses_a_cached_text_then_the_corpus() {
    let srv = server(vec![
        (OPENAI, 500, "{}".into()),
        (OPENAI, 500, "{}".into()),
        (OPENAI, 500, "{}".into()),
    ])
    .await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let targets = vec!["z".to_owned()];
    lock_store(&state.store)
        .cache_text(&NewCachedText {
            provider: "claude",
            model: "claude-haiku-5-5",
            language: "es",
            mode: "prose",
            targets: &targets,
            text: "zumo de zanahoria",
            created_at_ms: 1,
            used: false,
        })
        .unwrap();
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(
        got,
        NextText {
            text: "zumo de zanahoria".into(),
            targets,
            source: TextSource::Cache,
            provider: Some("claude".into()),
            model: Some("claude-haiku-5-5".into()),
            notice: Some("Sin conexión con la IA: texto del caché".into()),
            syntax: None,
            tokens: None,
        }
    );
    // That text was shown: the next failure uses the corpus instead of repeating it
    // ("cuando sale error de ia siempre sale el mismo texto").
    let again = next_text(&state, &params(5)).await.unwrap();
    assert_eq!(again.source, TextSource::Corpus);
    assert_eq!(
        again.notice.as_deref(),
        Some("Sin conexión con la IA: texto del corpus local")
    );
    assert_ne!(again.text, "zumo de zanahoria");
    // An empty cache: the corpus comes next.
    let state = self::state(state.secrets.clone(), srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    let mut expected = corpus_expected(&state, 4);
    expected.notice = Some("Sin conexión con la IA: texto del corpus local".into());
    assert_eq!(got, expected);
    assert!(usage_of(&state).is_empty());
}

#[tokio::test]
async fn an_invalid_ai_text_is_retried_once_then_the_corpus_is_used() {
    let srv = server(vec![
        (OPENAI, 200, chat_reply("sin la letra")),
        (OPENAI, 200, chat_reply("sin la letra")),
    ])
    .await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(srv.requests().len(), 2);
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(
        got.notice.as_deref(),
        Some("La IA no devolvió un texto válido: texto del corpus local")
    );
    assert_eq!(got.text, corpus_expected(&state, 4).text);
}

#[tokio::test]
async fn an_invalid_prose_text_is_retried_as_any_text_and_practises_none() {
    // "si no hay letras suficientes o no hay una respuesta valida pide un texto cualquiera"
    const NO_N: &str = "La casa del lago tiene una puerta roja y dos ventanas.";
    // The layout cannot type the emoji, so the first text is invalid.
    let untypeable = format!("{} \u{1F642}", NO_N.trim_end_matches('.'));
    let srv = server(vec![
        (OPENAI, 200, chat_reply(&untypeable)),
        (OPENAI, 200, chat_reply(NO_N)),
    ])
    .await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &params(4)).await.unwrap();
    let requests = srv.requests();
    assert_eq!(requests.len(), 2);
    let prompt = |i: usize| {
        let sent: serde_json::Value = serde_json::from_str(&requests[i].body).unwrap();
        sent["messages"][0]["content"].as_str().unwrap().to_owned()
    };
    // Prose never states targets (S9), so both attempts ask for the same thing.
    assert_eq!(prompt(0), prompt(1));
    assert!(!prompt(0).contains("Caracteres objetivo"), "{}", prompt(0));
    assert_eq!(
        (got.source, got.text.as_str(), got.targets.len()),
        (TextSource::Ai, NO_N, 0)
    );
    assert_eq!(got.notice, None);
}

#[tokio::test]
async fn a_missing_key_or_model_or_keychain_falls_back_with_a_notice() {
    let srv = server(vec![]).await;
    let cases: [(SharedSecrets, (ProviderId, &str), &str); 3] = [
        (
            MemoryStore::shared(),
            (ProviderId::Gemini, "gemini-3.5-flash-lite"),
            "Falta la clave de Gemini: texto del corpus local",
        ),
        (
            MemoryStore::shared(),
            (ProviderId::OpencodeGo, ""),
            "Falta el modelo de OpenCode Go: texto del corpus local",
        ),
        (
            Arc::new(NoStore),
            (ProviderId::Claude, "claude-haiku-5-5"),
            "Llavero no disponible: texto del corpus local",
        ),
    ];
    for (secrets, entry, notice) in cases {
        let state = state(secrets, srv.client());
        weak_in_n(&state);
        enable(&state, &[entry]);
        let got = next_text(&state, &params(4)).await.unwrap();
        assert_eq!(got.source, TextSource::Corpus);
        assert_eq!(got.notice.as_deref(), Some(notice));
        assert_eq!(got.text, corpus_expected(&state, 4).text);
    }
    assert!(srv.requests().is_empty());
}

#[tokio::test]
async fn first_text_is_the_diagnostic_for_a_new_profile_then_like_next_text() {
    let state = state(MemoryStore::shared(), offline());
    let first = first_text(
        &state,
        "es",
        "es-qwerty",
        &PracticeMode::Prose,
        DEFAULT_MAX_LINES,
        1,
        DAY,
    )
    .await
    .unwrap();
    assert!(first.diagnostic);
    assert_eq!(
        first.text,
        NextText::corpus(commands::diagnostic_text("es").unwrap(), vec![])
    );
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        json!({"text": first.text.text, "targets": [], "source": "corpus", "diagnostic": true})
    );
    {
        let mut store = lock_store(&state.store);
        let events = typing("nanana");
        let profile = state.active_profile();
        let scored = score_session(
            &mut store,
            &profile,
            "ñañaña",
            ErrorMode::AllowContinue,
            &events,
            1,
        );
        assert!(scored.saved);
    }
    let next = first_text(
        &state,
        "es",
        "es-qwerty",
        &PracticeMode::Prose,
        DEFAULT_MAX_LINES,
        1,
        DAY,
    )
    .await
    .unwrap();
    assert!(!next.diagnostic);
    let p = TextParams {
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        ..params(1)
    };
    assert_eq!(next.text, next_text(&state, &p).await.unwrap());
    assert!(
        first_text(
            &state,
            "xx",
            "es-qwerty",
            &PracticeMode::Prose,
            DEFAULT_MAX_LINES,
            1,
            DAY
        )
        .await
        .is_err()
    );
}

// ---- words mode ----

fn words_params(seed: u64) -> TextParams {
    TextParams {
        mode: PracticeMode::Words,
        length: 300,
        ..params(seed)
    }
}

/// The words text the corpus gives for `p` with the prose model.
fn words_expected(state: &AppState, p: &TextParams) -> NextText {
    let store = lock_store(&state.store);
    let model = commands::load_model(&store, &state.active_profile()).unwrap();
    let req = NextTextRequest {
        language: &p.language,
        layout: &p.layout,
        seed: p.seed,
        target_count: p.target_count,
        length: p.length,
        fallback: &p.fallback,
    };
    commands::words_text(&commands::plan_text(&model, &req).unwrap(), p.seed).unwrap()
}

#[tokio::test]
async fn words_mode_never_asks_the_ai_nor_the_cache_even_when_enabled() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    cache_unused(&state, "prose", "texto de prosa guardado", 1);
    cache_unused(&state, "words", "palabras guardadas", 2);
    let p = words_params(4);
    let got = next_text(&state, &p).await.unwrap();
    assert_eq!(got, words_expected(&state, &p));
    assert_eq!((got.source, got.notice.clone()), (TextSource::Corpus, None));
    assert_eq!(got.targets, vec!["ñ".to_owned()], "prose model targets");
    assert!(
        got.text
            .chars()
            .all(|c| c == ' ' || (c.is_alphabetic() && !c.is_uppercase())),
        "{}",
        got.text
    );
    assert!(srv.requests().is_empty());
    assert!(usage_of(&state).is_empty());
    // Neither cached text was taken.
    let store = lock_store(&state.store);
    assert!(
        store
            .take_unused_cached_text("es", "prose", &[])
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .take_unused_cached_text("es", "words", &[])
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn first_text_in_words_mode_skips_the_diagnostic_and_the_ai() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let first = first_text(
        &state,
        "es",
        "es-qwerty",
        &PracticeMode::Words,
        DEFAULT_MAX_LINES,
        1,
        DAY,
    )
    .await
    .unwrap();
    assert!(!first.diagnostic);
    let p = TextParams {
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        ..words_params(1)
    };
    assert_eq!(first.text, words_expected(&state, &p));
    assert!(srv.requests().is_empty());
}

// ---- AI length margin (S5) ----

#[test]
fn a_300_char_setting_accepts_ai_texts_from_240_to_375_chars() {
    let req = NextTextRequest {
        language: "es",
        layout: "es-qwerty",
        seed: 1,
        target_count: 2,
        length: 300,
        fallback: &[],
    };
    let model = plexustyping_core::adaptive::WeaknessModel::new();
    let plan = Plan::Prose(commands::plan_text(&model, &req).unwrap());
    let request = plan.request("es", &AiSettings::default(), false).unwrap();
    for (len, ok) in [(239, false), (240, true), (375, true), (376, false)] {
        let text = "a".repeat(len);
        assert_eq!(
            plexustyping_ai::validate(&text, &request).is_ok(),
            ok,
            "{len} chars"
        );
    }
    let prompt = plexustyping_ai::build_prompt(&request);
    assert!(
        prompt.starts_with("Escribe un texto en español de 240–375 caracteres."),
        "{prompt}"
    );
}

// ---- code mode ----

/// 99 chars, 10 × `{`: valid for target `{` at 3× over 80..481 chars.
const AI_CODE: &str = "fn a() { b({ c }) }\nfn d() { e({ f }) }\nfn g() { h({ i }) }\nfn j() { k({ l }) }\nfn m() { n({ o }) }";

fn rust() -> PracticeMode {
    PracticeMode::code("rust").unwrap()
}

fn code_params(seed: u64) -> TextParams {
    TextParams {
        mode: rust(),
        ..params(seed)
    }
}

/// The code model practises only `{`; the prose model only ñ.
fn weak_in_brace(state: &AppState) {
    weak_in_n(state);
    let snapshot = StatsSnapshot {
        units: vec![UnitStatRow {
            unit: "{".into(),
            ema_error: 0.9,
            ema_slowness: 0.0,
            samples: 5,
            slowness_samples: 0,
        }],
        confusions: vec![],
        mean_interval_ms: Some(150.0),
    };
    let mut store = lock_store(&state.store);
    store
        .save_stats(state.active_profile().id, "code", &snapshot)
        .unwrap();
}

fn assert_contiguous(text: &NextText) {
    let tokens = text.tokens.as_deref().expect("code texts carry tokens");
    let mut at = 0;
    for t in tokens {
        assert_eq!(t.start, at, "{tokens:?}");
        at = t.end;
    }
    assert_eq!(at, text.text.chars().count());
}

#[tokio::test]
async fn code_mode_without_ai_gives_a_snippet_for_the_code_model_with_tokens() {
    let state = state(MemoryStore::shared(), offline());
    weak_in_brace(&state);
    let got = next_text(&state, &code_params(5)).await.unwrap();
    let snippets = plexustyping_corpus::code_snippets("rust").unwrap();
    assert!(snippets.iter().any(|s| s.code == got.text), "{}", got.text);
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(got.targets, vec!["{".to_owned()], "code model, not ñ");
    assert_eq!(got.syntax.as_deref(), Some("rust"));
    assert_contiguous(&got);
    let json = serde_json::to_value(&got).unwrap();
    assert_eq!(json["syntax"], "rust");
    assert_eq!(json["tokens"][0]["start"], 0);
    assert!(json["tokens"][0]["class"].is_string());
    // Prose texts stay as before: no syntax, no tokens.
    let prose = next_text(&state, &params(5)).await.unwrap();
    assert_eq!(prose, corpus_expected(&state, 5));
    let json = serde_json::to_value(&prose).unwrap();
    assert!(json.get("syntax").is_none() && json.get("tokens").is_none());
}

#[tokio::test]
async fn code_mode_asks_the_ai_for_code_and_caches_it_per_language() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_CODE))]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_brace(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let got = next_text(&state, &code_params(4)).await.unwrap();
    assert_eq!(
        (got.text.as_str(), got.source, got.targets.clone()),
        (AI_CODE, TextSource::Ai, vec!["{".to_owned()])
    );
    assert_eq!(got.syntax.as_deref(), Some("rust"));
    assert_contiguous(&got);
    let requests = srv.requests();
    assert_eq!(requests.len(), 1);
    let sent: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    let prompt = sent["messages"][0]["content"].as_str().unwrap();
    assert!(prompt.contains("fragmento de código en Rust"), "{prompt}");
    // {: ceil(3 × rust code frequency × 480).
    assert!(
        prompt.contains("Caracteres objetivo: { (al menos 19 veces)."),
        "{prompt}"
    );
    assert!(prompt.contains("Longitud: 80–480 caracteres."), "{prompt}");
    let (_, listed) = prompt.split_once("Usa solo estos caracteres: ").unwrap();
    for ch in ['{', '}', '~', '|', '&', '$', '#', '"', '\''] {
        assert!(listed.contains(ch), "allowed {ch:?} missing: {prompt}");
    }
    assert!(!listed.contains('ñ'), "only ASCII in code: {prompt}");
    let store = lock_store(&state.store);
    assert_eq!(store.take_cached_text("es", "prose", &[]).unwrap(), None);
    let cached = store
        .take_cached_text("es", "code:rust", &[])
        .unwrap()
        .unwrap();
    assert_eq!(cached.text, AI_CODE);
}

#[tokio::test]
async fn code_mode_falls_back_to_its_own_language_cache_then_the_corpus() {
    let srv = server(vec![(OPENAI, 500, "{}".into()), (OPENAI, 500, "{}".into())]).await;
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, srv.client());
    weak_in_brace(&state);
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    let cache = |mode: &str, text: &str| {
        lock_store(&state.store)
            .cache_text(&NewCachedText {
                provider: "claude",
                model: "claude-haiku-5-5",
                language: "es",
                mode,
                targets: &[],
                text,
                created_at_ms: 1,
                used: false,
            })
            .unwrap();
    };
    cache("prose", "texto de prosa");
    cache("code:python", "print(1)");
    let got = next_text(&state, &code_params(4)).await.unwrap();
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(
        got.notice.as_deref(),
        Some("Sin conexión con la IA: texto del corpus local")
    );
    assert_eq!(got.syntax.as_deref(), Some("rust"));
    cache("code:rust", "let x = 1;");
    let got = next_text(&state, &code_params(4)).await.unwrap();
    assert_eq!(
        (got.text.as_str(), got.source),
        ("let x = 1;", TextSource::Cache)
    );
    assert_eq!(got.syntax.as_deref(), Some("rust"));
    assert_contiguous(&got);
}

#[tokio::test]
async fn html_css_ai_texts_are_highlighted_as_html() {
    let state = state(MemoryStore::shared(), offline());
    enable(&state, &[(ProviderId::Openai, "gpt-5-mini")]);
    lock_store(&state.store)
        .cache_text(&NewCachedText {
            provider: "claude",
            model: "m",
            language: "es",
            mode: "code:html-css",
            targets: &[],
            text: "<p class=\"a\">hola</p>",
            created_at_ms: 1,
            used: false,
        })
        .unwrap();
    let p = TextParams {
        mode: PracticeMode::code("html-css").unwrap(),
        ..params(1)
    };
    let got = next_text(&state, &p).await.unwrap();
    assert_eq!(got.source, TextSource::Cache);
    assert_eq!(got.syntax.as_deref(), Some("html"));
    assert_contiguous(&got);
}

#[tokio::test]
async fn first_text_skips_the_diagnostic_a_layout_cannot_type() {
    let state = state(MemoryStore::shared(), offline());
    let first = first_text(
        &state,
        "es",
        "us-qwerty",
        &PracticeMode::Prose,
        DEFAULT_MAX_LINES,
        1,
        DAY,
    )
    .await
    .unwrap();
    assert!(!first.diagnostic);
    let us = commands::get_layout("us-qwerty").unwrap().allowed_chars();
    assert!(!first.text.text.is_empty());
    assert!(first.text.text.chars().all(|ch| us.contains(&ch)));
    let p = TextParams {
        layout: "us-qwerty".into(),
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        ..params(1)
    };
    assert_eq!(first.text, next_text(&state, &p).await.unwrap());
}

#[tokio::test]
async fn first_text_in_code_mode_skips_the_prose_diagnostic() {
    let state = state(MemoryStore::shared(), offline());
    let first = first_text(&state, "es", "es-qwerty", &rust(), 5, 1, DAY)
        .await
        .unwrap();
    assert!(!first.diagnostic);
    assert_eq!(first.text.syntax.as_deref(), Some("rust"));
    let p = TextParams {
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        max_lines: 5,
        ..code_params(1)
    };
    assert_eq!(first.text, next_text(&state, &p).await.unwrap());
}

/// Tauri needs `Send` command futures; a `std::sync::MutexGuard` held across an
/// `.await` would make these futures `!Send` and fail to compile.
#[test]
fn generation_futures_hold_no_store_lock_across_await() {
    fn assert_send<F: std::future::Future + Send>(_: F) {}
    let state = state(MemoryStore::shared(), offline());
    let p = params(1);
    assert_send(next_text(&state, &p));
    assert_send(first_text(
        &state,
        "es",
        "es-qwerty",
        &PracticeMode::Prose,
        DEFAULT_MAX_LINES,
        1,
        DAY,
    ));
    assert_send(models(&state.ai, &state.secrets, ProviderId::Claude));
    assert_send(test_connection(
        &state.ai,
        &state.secrets,
        ProviderId::Claude,
        "",
    ));
}

/// L2: a natural 340-char Spanish text with ntr, ú and ti.
const NATURAL_340: &str = "Entré en la tienda del centro a las nueve. Dentro había poca gente y un señor muy simpático me atendió enseguida. Le pedí un café y un pan tostado, y me senté junto a la ventana para ver la calle. Fuera, la tarde estaba tranquila. Pensé en mi último viaje al norte, en las montañas y en los amigos que dejé allí. Luego pagué y volví a casa.";

fn prose_plan(targets: &[&str]) -> Plan {
    Plan::Prose(commands::TextPlan {
        corpus: plexustyping_corpus::corpus("es").unwrap(),
        targets: targets.iter().map(|t| (*t).to_owned()).collect(),
        length: 300..381,
        allowed: NATURAL_340.chars().chain(['a']).collect(),
    })
}

#[test]
fn a_prose_request_carries_the_corpus_frequency_of_each_target() {
    let es = plexustyping_corpus::corpus("es").unwrap();
    let req = prose_plan(&["ntr", "ú", "ti", "a"])
        .request("es", &AiSettings::default(), false)
        .unwrap();
    assert_eq!(
        req.frequencies,
        ["ntr", "ú", "ti", "a"].map(|t| es.frequency(t)).to_vec()
    );
    assert_eq!(req.density, 3.0);
    // Rare targets need a few; a frequent letter needs more than its natural share
    // but at most 5 points over it, so the text stays writable.
    let counts = req.min_counts(340);
    assert_eq!(&counts[..3], &[("ntr", 1), ("ú", 1), ("ti", 4)]);
    let natural = es.frequency("a") * 340.0;
    assert!(counts[3].1 as f64 > natural, "{counts:?}");
    assert!(
        counts[3].1 as f64 <= natural + 0.05 * 340.0 + 1.0,
        "{counts:?}"
    );
}

#[test]
fn the_log_example_now_passes_validation() {
    assert_eq!(NATURAL_340.chars().count(), 340);
    let req = prose_plan(&["ntr", "ú", "ti"])
        .request("es", &AiSettings::default(), false)
        .unwrap();
    assert_eq!(
        plexustyping_ai::validate(NATURAL_340, &req).as_deref(),
        Ok(NATURAL_340)
    );
}

#[test]
fn only_a_restricted_prose_request_names_its_characters() {
    let prose = prose_plan(&["a"]);
    assert!(
        !prose
            .request("es", &AiSettings::default(), false)
            .unwrap()
            .restrict_chars
    );
    assert!(
        prose
            .request("es", &AiSettings::default(), true)
            .unwrap()
            .restrict_chars
    );
    let code = Plan::Code(code::CodePlan {
        lang: "rust".into(),
        targets: vec!["{".into()],
        max_lines: 20,
    });
    assert!(
        !code
            .request("es", &AiSettings::default(), true)
            .unwrap()
            .restrict_chars
    );
}

#[test]
fn a_code_request_uses_the_code_corpus_frequency() {
    let plan = Plan::Code(code::CodePlan {
        lang: "rust".into(),
        targets: vec!["{".into(), "ñ".into()],
        max_lines: 20,
    });
    let req = plan.request("es", &AiSettings::default(), false).unwrap();
    let brace = plexustyping_corpus::code_frequency("rust", "{").unwrap();
    assert!(brace > 0.0);
    assert_eq!(req.frequencies, vec![brace, 0.0]);
    // A target absent from the code corpus needs one occurrence.
    assert_eq!(req.min_counts(100)[1], ("ñ", 1));
}

#[test]
fn rejections_are_one_line_per_invalid_attempt_without_the_key() {
    use plexustyping_ai::ValidationIssue;
    let errors = vec![
        (
            ProviderId::Openai,
            ProviderError::Invalid {
                issues: vec![
                    ValidationIssue::Length { len: 412 },
                    ValidationIssue::CharNotAllowed { ch: '€' },
                    ValidationIssue::Markdown,
                    ValidationIssue::LowDensity {
                        target: "ntr".into(),
                        found: 1,
                        needed: 2,
                    },
                ],
            },
        ),
        (ProviderId::Gemini, ProviderError::Timeout),
        (
            ProviderId::Claude,
            ProviderError::Invalid {
                issues: vec![ValidationIssue::Empty],
            },
        ),
    ];
    let lines = rejection_lines(&errors);
    assert_eq!(
        lines,
        vec![
            "PlexusTyping: AI text from OpenAI rejected: length 412; char not allowed '€'; \
             markdown; low density \"ntr\" found 1 needed 2"
                .to_owned(),
            "PlexusTyping: AI attempt with Gemini failed: the provider did not answer in time"
                .to_owned(),
            "PlexusTyping: AI text from Claude rejected: empty".to_owned(),
        ]
    );
    assert!(lines.iter().all(|l| !l.contains(KEY)));
    // A malformed reply is logged too: it shows the same notice as a rejected text.
    let malformed = ProviderError::BadResponse {
        message: "The answer was not a {\"text\": \"...\"} JSON object.".into(),
    };
    assert_eq!(
        rejection_lines(&[(ProviderId::OpencodeGo, malformed)]),
        [
            "PlexusTyping: AI attempt with OpenCode Go failed: The answer was not a {\"text\": \"...\"} JSON object."
        ]
    );
}

// ---- ChatGPT (tu suscripción) ----

const RESPONSES: &str = "/v1/responses";
const CHATGPT_MODEL: &str = "gpt-6.1-sol";
const CHATGPT_TOKEN: &str = "at-SECRET-chatgpt-1";
const PLAN_SCOPES: [&str; 6] = [
    "chatgpt.tokens.use.direct",
    "email",
    "offline_access",
    "openid",
    "profile",
    "resource.invoke",
];

/// A scratch folder under the workspace `target/tmp`, removed when dropped.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/tmp")
            .join(format!(
                "app-chatgpt-{}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// How the ChatGPT account is when the test starts.
#[derive(Clone, Copy)]
enum Account {
    SignedIn,
    PlanOff,
    SignedOut,
}

/// The mock's client with a ChatGPT sign-in whose record (if any) is `account`.
fn chatgpt_client(srv: &Mock, dir: &Scratch, account: Account) -> AiClient {
    use plexustyping_ai::chatgpt_auth::{CREDENTIALS_FILE, ChatgptAuth};
    let path = dir.0.join(CREDENTIALS_FILE);
    let scopes: Vec<&str> = match account {
        Account::SignedIn => PLAN_SCOPES.to_vec(),
        _ => PLAN_SCOPES[1..].to_vec(),
    };
    if !matches!(account, Account::SignedOut) {
        let expires = std::time::UNIX_EPOCH.elapsed().unwrap().as_secs() + 3600;
        let record = json!({
            "hostId": "urn:uuid:00000000-0000-4000-8000-000000000000",
            "clientId": "oaiapp_TEST-client-1",
            "subject": "user-sub-1",
            "email": "ana@example.com",
            "accessToken": CHATGPT_TOKEN,
            "refreshToken": "rt-SECRET-chatgpt-1",
            "scopes": scopes,
            "expiresAt": expires
        });
        std::fs::write(&path, record.to_string()).unwrap();
    }
    let auth = ChatgptAuth::new(&srv.endpoints(), Some(path));
    srv.client().with_chatgpt(Arc::new(auth))
}

/// A streamed Responses reply answering `{"text": text}`.
fn chatgpt_reply(text: &str) -> String {
    let answer = json!({"text": text}).to_string();
    let events = [
        json!({"type": "response.created", "response": {"status": "in_progress"}}),
        json!({"type": "response.output_text.delta", "delta": answer}),
        json!({"type": "response.completed", "response": {
            "status": "completed",
            "output": [{"type": "message", "content": [{"type": "output_text", "text": answer}]}],
            "usage": {"input_tokens": 50, "output_tokens": 20}
        }}),
    ];
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
        .collect()
}

fn chatgpt_limit() -> String {
    let event = json!({"type": "response.failed", "response": {"status": "failed",
        "error": {"code": "subscription_sharing_usage_limit_exceeded", "message": "limit"}}});
    format!("event: response.failed\ndata: {event}\n\n")
}

#[tokio::test]
async fn a_signed_in_chatgpt_writes_the_text_with_its_token_and_counts_usage() {
    let srv = server(vec![(RESPONSES, 200, chatgpt_reply(AI_TEXT))]).await;
    let dir = Scratch::new();
    let memory = MemoryStore::shared();
    let state = state(
        memory.clone(),
        chatgpt_client(&srv, &dir, Account::SignedIn),
    );
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Chatgpt, CHATGPT_MODEL)]);

    let got = next_text(&state, &params(4)).await.unwrap();

    assert_eq!(
        (got.source, got.text.as_str(), got.notice.as_deref()),
        (TextSource::Ai, AI_TEXT, None)
    );
    assert_eq!(got.provider.as_deref(), Some("chatgpt"));
    assert_eq!(got.model.as_deref(), Some(CHATGPT_MODEL));
    let requests = srv.requests();
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(
        (body["store"].clone(), body["stream"].clone()),
        (json!(false), json!(true))
    );
    assert_eq!(body["model"], CHATGPT_MODEL);
    let usage = usage_of(&state);
    assert_eq!(usage.len(), 1);
    assert_eq!(
        (
            usage[0].provider.as_str(),
            usage[0].calls_today,
            usage[0].input_tokens_today,
            usage[0].output_tokens_today
        ),
        ("chatgpt", 1, 50, 20)
    );
    assert_eq!(
        memory.get("ai:chatgpt").unwrap(),
        None,
        "no key in the keychain"
    );
}

#[tokio::test]
async fn chatgpt_without_a_usable_sign_in_is_skipped_with_a_notice() {
    let srv = server(vec![(RESPONSES, 200, chatgpt_reply(AI_TEXT))]).await;
    let cases = [
        (
            Some(Account::SignedOut),
            "Inicia sesión en ChatGPT: texto del corpus local",
        ),
        (None, "Inicia sesión en ChatGPT: texto del corpus local"),
        (
            Some(Account::PlanOff),
            "Tu cuenta de ChatGPT no tiene activado el uso del plan: texto del corpus local",
        ),
    ];
    for (account, notice) in cases {
        let dir = Scratch::new();
        let client = match account {
            Some(account) => chatgpt_client(&srv, &dir, account),
            None => srv.client(),
        };
        let state = state(MemoryStore::shared(), client);
        weak_in_n(&state);
        enable(&state, &[(ProviderId::Chatgpt, CHATGPT_MODEL)]);
        let got = next_text(&state, &params(4)).await.unwrap();
        assert_eq!(got.source, TextSource::Corpus);
        assert_eq!(got.notice.as_deref(), Some(notice));
        assert_eq!(got.text, corpus_expected(&state, 4).text);
    }
    assert!(
        srv.requests().is_empty(),
        "nothing is sent without a sign-in"
    );
}

#[tokio::test]
async fn a_skipped_chatgpt_leaves_the_rest_of_the_chain_working() {
    let srv = server(vec![(OPENAI, 200, chat_reply(AI_TEXT))]).await;
    let dir = Scratch::new();
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, chatgpt_client(&srv, &dir, Account::SignedOut));
    weak_in_n(&state);
    enable(
        &state,
        &[
            (ProviderId::Chatgpt, CHATGPT_MODEL),
            (ProviderId::Openai, "gpt-5-mini"),
        ],
    );
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got.provider.as_deref(), Some("openai"));
    let paths: Vec<String> = srv.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, [OPENAI]);
}

#[tokio::test]
async fn a_chatgpt_plan_error_stops_the_chain_without_billing_the_next_provider() {
    let srv = server(vec![
        (RESPONSES, 200, chatgpt_limit()),
        (OPENAI, 200, chat_reply(AI_TEXT)),
    ])
    .await;
    let dir = Scratch::new();
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, chatgpt_client(&srv, &dir, Account::SignedIn));
    weak_in_n(&state);
    enable(
        &state,
        &[
            (ProviderId::Chatgpt, CHATGPT_MODEL),
            (ProviderId::Openai, "gpt-5-mini"),
        ],
    );
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(
        got.notice.as_deref(),
        Some("Has alcanzado el límite de tu plan de ChatGPT: texto del corpus local")
    );
    assert_eq!(got.text, corpus_expected(&state, 4).text);
    let paths: Vec<String> = srv.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, [RESPONSES], "the next provider gets no request");
    assert!(usage_of(&state).is_empty());
}

#[tokio::test]
async fn a_chatgpt_network_failure_still_reaches_the_next_provider() {
    let srv = server(vec![
        (RESPONSES, 503, "{}".to_owned()),
        (OPENAI, 200, chat_reply(AI_TEXT)),
    ])
    .await;
    let dir = Scratch::new();
    let memory = MemoryStore::shared();
    with_key(&memory, ProviderId::Openai).await;
    let state = state(memory, chatgpt_client(&srv, &dir, Account::SignedIn));
    weak_in_n(&state);
    enable(
        &state,
        &[
            (ProviderId::Chatgpt, CHATGPT_MODEL),
            (ProviderId::Openai, "gpt-5-mini"),
        ],
    );
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got.source, TextSource::Ai);
    assert_eq!(got.provider.as_deref(), Some("openai"));
    let paths: Vec<String> = srv.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, [RESPONSES, OPENAI]);
    let usage = usage_of(&state);
    assert_eq!(usage.len(), 1, "only the successful call is counted");
    assert_eq!(usage[0].provider, "openai");
}

#[tokio::test]
async fn a_chatgpt_plan_limit_gives_its_own_notice() {
    let srv = server(vec![(RESPONSES, 200, chatgpt_limit())]).await;
    let dir = Scratch::new();
    let state = state(
        MemoryStore::shared(),
        chatgpt_client(&srv, &dir, Account::SignedIn),
    );
    weak_in_n(&state);
    enable(&state, &[(ProviderId::Chatgpt, CHATGPT_MODEL)]);
    let got = next_text(&state, &params(4)).await.unwrap();
    assert_eq!(got.source, TextSource::Corpus);
    assert_eq!(
        got.notice.as_deref(),
        Some("Has alcanzado el límite de tu plan de ChatGPT: texto del corpus local")
    );
    assert_eq!(srv.requests().len(), 1, "a plan limit is not retried");
    assert!(usage_of(&state).is_empty());
}

#[test]
fn chatgpt_errors_show_their_own_spanish_message() {
    let id = ProviderId::Chatgpt;
    let message = |m: &str| m.to_owned();
    let cases = [
        (
            ProviderError::RateLimit {
                message: message("Has alcanzado el límite de tu plan de ChatGPT"),
            },
            "Has alcanzado el límite de tu plan de ChatGPT",
        ),
        (
            ProviderError::Auth {
                message: message("Inicia sesión en ChatGPT"),
            },
            "Inicia sesión en ChatGPT",
        ),
        (
            ProviderError::Config {
                message: message("ChatGPT no autorizó esta petición de PlexusTyping"),
            },
            "ChatGPT no autorizó esta petición de PlexusTyping",
        ),
        (
            ProviderError::Network {
                message: message("No se pudo conectar con ChatGPT: x."),
            },
            "Sin conexión con la IA",
        ),
        (ProviderError::Timeout, "ChatGPT no respondió a tiempo"),
        (
            ProviderError::BadResponse {
                message: message("cut"),
            },
            "La IA no devolvió un texto válido",
        ),
    ];
    for (err, reason) in cases {
        assert_eq!(provider_reason(id, &err), reason, "{err:?}");
    }
    assert_eq!(short_name(id), "ChatGPT");
    assert_eq!(slug(id), "chatgpt");
}

#[tokio::test]
async fn chatgpt_takes_no_key_and_its_status_is_the_sign_in() {
    let srv = server(vec![]).await;
    let memory = MemoryStore::shared();
    let secrets: SharedSecrets = memory.clone();
    let id = ProviderId::Chatgpt;
    let refused = key_save(&secrets, id, KEY.into()).await;
    assert_eq!(
        refused,
        Err(AiFailure::new(
            "config",
            "ChatGPT no usa clave: inicia sesión con ChatGPT en Ajustes → IA"
        ))
    );
    assert_eq!(memory.get("ai:chatgpt").unwrap(), None, "nothing stored");
    for (account, ready) in [
        (Account::SignedIn, true),
        (Account::PlanOff, false),
        (Account::SignedOut, false),
    ] {
        let dir = Scratch::new();
        let client = chatgpt_client(&srv, &dir, account);
        assert_eq!(key_status(&client, &secrets, id).await, Ok(ready));
        // Deleting does not touch the keychain nor sign out: sign-out is its own action.
        assert_eq!(key_delete(&secrets, id).await, Ok(()));
        assert_eq!(key_status(&client, &secrets, id).await, Ok(ready));
    }
    assert_eq!(key_status(&srv.client(), &secrets, id).await, Ok(false));
    // A keychain that cannot be used does not matter to ChatGPT.
    let none: SharedSecrets = Arc::new(NoStore);
    let dir = Scratch::new();
    let client = chatgpt_client(&srv, &dir, Account::SignedIn);
    assert_eq!(key_status(&client, &none, id).await, Ok(true));
    assert!(srv.requests().is_empty());
}

#[tokio::test]
async fn chatgpt_models_come_from_the_signed_in_account() {
    let list = json!({"models": [
        {"slug": "gpt-6.2-sol", "display_name": "GPT-6.2 Sol", "visibility": "list"},
        {"slug": "hidden", "visibility": "hide"},
        {"slug": "gpt-6.1-sol", "display_name": "GPT-6.1 Sol", "visibility": "list"}
    ]});
    let srv = server(vec![("/v1/models", 200, list.to_string())]).await;
    let secrets: SharedSecrets = MemoryStore::shared();
    let dir = Scratch::new();
    let signed_out = chatgpt_client(&srv, &dir, Account::SignedOut);
    assert_eq!(
        models(&signed_out, &secrets, ProviderId::Chatgpt).await,
        Err(AiFailure::new("auth", "Inicia sesión en ChatGPT"))
    );
    assert!(srv.requests().is_empty());
    let dir = Scratch::new();
    let client = chatgpt_client(&srv, &dir, Account::SignedIn);
    assert_eq!(
        models(&client, &secrets, ProviderId::Chatgpt).await,
        Ok(vec!["gpt-6.2-sol".to_owned(), "gpt-6.1-sol".to_owned()])
    );
    assert_eq!(srv.requests()[0].path, "/v1/models");
}

#[tokio::test]
async fn the_chatgpt_connection_test_is_the_sign_in_without_any_inference() {
    let srv = server(vec![(RESPONSES, 200, chatgpt_reply(AI_TEXT))]).await;
    let secrets: SharedSecrets = MemoryStore::shared();
    let id = ProviderId::Chatgpt;
    let cases = [
        (Account::SignedIn, Ok(())),
        (
            Account::SignedOut,
            Err(AiFailure::new("auth", "Inicia sesión en ChatGPT")),
        ),
        (
            Account::PlanOff,
            Err(AiFailure::new(
                "auth",
                "Tu cuenta de ChatGPT no tiene activado el uso del plan",
            )),
        ),
    ];
    for (account, expected) in cases {
        let dir = Scratch::new();
        let client = chatgpt_client(&srv, &dir, account);
        assert_eq!(
            test_connection(&client, &secrets, id, CHATGPT_MODEL).await,
            expected
        );
    }
    assert_eq!(
        test_connection(&srv.client(), &secrets, id, "").await,
        Err(AiFailure::new("auth", "Inicia sesión en ChatGPT"))
    );
    assert!(srv.requests().is_empty(), "no inference request");
}

// ---- Sign in with ChatGPT commands (S5) ----

/// A signed-out sign-in on any free callback port that waits at most `wait`.
fn sign_in_client(srv: &Mock, dir: &Scratch, wait: std::time::Duration) -> AiClient {
    use plexustyping_ai::chatgpt_auth::{CREDENTIALS_FILE, ChatgptAuth};
    let auth = ChatgptAuth::new(&srv.endpoints(), Some(dir.0.join(CREDENTIALS_FILE)))
        .with_callback_port(0)
        .with_sign_in_timeout(wait);
    srv.client().with_chatgpt(Arc::new(auth))
}

/// Answers the browser's callback for `url` with `query` (plus the attempt's `state`).
fn answer_callback(url: &str, query: &'static str) {
    use plexustyping_ai::chatgpt_auth::parse_query;
    let params = parse_query(url.split_once('?').unwrap().1);
    let redirect = params["redirect_uri"].clone();
    let state = params["state"].clone();
    let host = redirect
        .strip_prefix("http://")
        .unwrap()
        .split('/')
        .next()
        .unwrap()
        .to_owned();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        let mut sock = std::net::TcpStream::connect(&host).unwrap();
        let request = format!(
            "GET /auth/callback?{query}&state={state} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        );
        sock.write_all(request.as_bytes()).unwrap();
        let mut reply = Vec::new();
        let _ = sock.read_to_end(&mut reply);
    });
}

#[tokio::test]
async fn chatgpt_status_shows_the_account_and_plan_but_never_a_token() {
    use plexustyping_ai::chatgpt_auth::Status;
    let srv = server(vec![]).await;
    let signed = |signed_in, email: Option<&str>, plan_enabled| Status {
        signed_in,
        email: email.map(str::to_owned),
        plan_enabled,
    };
    let cases = [
        (
            Account::SignedIn,
            signed(true, Some("ana@example.com"), true),
        ),
        (
            Account::PlanOff,
            signed(true, Some("ana@example.com"), false),
        ),
        (Account::SignedOut, signed(false, None, false)),
    ];
    for (account, expected) in cases {
        let dir = Scratch::new();
        let client = chatgpt_client(&srv, &dir, account);
        let status = chatgpt_status(&client).await.unwrap();
        assert_eq!(status, expected);
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(
            json.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["email", "planEnabled", "signedIn"]
        );
        assert!(!json.to_string().contains("SECRET"));
    }
    // No sign-in configured (no config dir): simply signed out.
    assert_eq!(
        chatgpt_status(&srv.client()).await,
        Ok(signed(false, None, false))
    );
    assert!(srv.requests().is_empty(), "the status needs no network");
}

#[tokio::test]
async fn a_chatgpt_sign_in_can_be_cancelled_while_the_browser_is_open() {
    let srv = server(vec![]).await;
    let dir = Scratch::new();
    let client = Arc::new(sign_in_client(
        &srv,
        &dir,
        std::time::Duration::from_secs(60),
    ));
    let opened = Arc::new(Mutex::new(Vec::<String>::new()));
    let task = {
        let (client, opened) = (client.clone(), opened.clone());
        tokio::spawn(async move {
            chatgpt_sign_in(&client, |url: &str| {
                opened.lock().unwrap().push(url.to_owned());
                Ok(())
            })
            .await
        })
    };
    while !client.chatgpt().unwrap().sign_in_pending() {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    chatgpt_cancel(&client);
    assert_eq!(
        task.await.unwrap(),
        Err(AiFailure::new(
            "cancelled",
            "Se canceló el inicio de sesión en ChatGPT"
        ))
    );
    let opened = opened.lock().unwrap().clone();
    assert_eq!(opened.len(), 1, "the browser opened once");
    assert!(opened[0].starts_with(&format!("{}/api/accounts/authorize?", srv.base())));
    assert!(!chatgpt_status(&client).await.unwrap().signed_in);
    // Cancelling with nothing pending, or without a sign-in at all, is harmless.
    chatgpt_cancel(&client);
    chatgpt_cancel(&srv.client());
}

#[tokio::test]
async fn chatgpt_sign_in_failures_are_spanish_and_sign_nobody_in() {
    let srv = server(vec![]).await;
    let short = std::time::Duration::from_millis(100);

    let dir = Scratch::new();
    let client = sign_in_client(&srv, &dir, std::time::Duration::from_secs(60));
    let denied = chatgpt_sign_in(&client, |url: &str| {
        answer_callback(url, "error=access_denied");
        Ok(())
    })
    .await;
    assert_eq!(
        denied,
        Err(AiFailure::new(
            "denied",
            "Has rechazado el permiso, así que PlexusTyping no puede usar tu plan de ChatGPT"
        ))
    );
    assert!(!chatgpt_status(&client).await.unwrap().signed_in);

    let dir = Scratch::new();
    let client = sign_in_client(&srv, &dir, short);
    assert_eq!(
        chatgpt_sign_in(&client, |_: &str| Ok(())).await,
        Err(AiFailure::new(
            "timeout",
            "El inicio de sesión en ChatGPT ha caducado. Inténtalo de nuevo"
        ))
    );

    let dir = Scratch::new();
    let client = sign_in_client(&srv, &dir, short);
    assert_eq!(
        chatgpt_sign_in(&client, |_: &str| Err("no browser".to_owned())).await,
        Err(AiFailure::new("config", "No se pudo abrir el navegador"))
    );

    assert_eq!(
        chatgpt_sign_in(&srv.client(), |_: &str| Ok(())).await,
        Err(AiFailure::new(
            "config",
            "PlexusTyping no tiene una carpeta donde guardar la sesión de ChatGPT"
        ))
    );
    assert!(srv.requests().is_empty(), "nothing reached OpenAI");
}

#[test]
fn chatgpt_sign_in_errors_keep_their_spanish_message_with_a_kind() {
    use plexustyping_ai::chatgpt_auth::AuthError;
    let cases = [
        (
            AuthError::KeysUnavailable,
            "network",
            "No se pudieron obtener las claves de OpenAI para verificar el inicio de sesión. Comprueba la conexión e inténtalo de nuevo",
        ),
        (
            AuthError::Unavailable { status: 503 },
            "network",
            "El servicio de inicio de sesión de ChatGPT no está disponible ahora (HTTP 503)",
        ),
        (
            AuthError::Cancelled,
            "cancelled",
            "Se canceló el inicio de sesión en ChatGPT",
        ),
        (
            AuthError::AccessDenied,
            "denied",
            "Has rechazado el permiso, así que PlexusTyping no puede usar tu plan de ChatGPT",
        ),
        (
            AuthError::Timeout,
            "timeout",
            "El inicio de sesión en ChatGPT ha caducado. Inténtalo de nuevo",
        ),
        (
            AuthError::IdentityMismatch,
            "config",
            "Ha iniciado sesión otra cuenta de ChatGPT distinta de la registrada; no se ha usado",
        ),
    ];
    for (err, kind, message) in cases {
        assert_eq!(auth_failure(err), AiFailure::new(kind, message));
    }
}

#[tokio::test]
async fn chatgpt_sign_out_revokes_and_forgets_the_tokens() {
    let srv = mock_with(|base| {
        let discovery = json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/api/accounts/authorize"),
            "token_endpoint": format!("{base}/api/accounts/oauth/token"),
            "jwks_uri": format!("{base}/jwks"),
            "revocation_endpoint": format!("{base}/revoke"),
            "id_token_signing_alg_values_supported": ["RS256"]
        });
        vec![
            (
                "/.well-known/openid-configuration".to_owned(),
                200,
                discovery.to_string(),
            ),
            ("/revoke".to_owned(), 200, "{}".to_owned()),
        ]
    })
    .await;
    let dir = Scratch::new();
    let client = chatgpt_client(&srv, &dir, Account::SignedIn);
    let out = chatgpt_sign_out(&client).await.unwrap();
    assert_eq!(
        serde_json::to_value(&out).unwrap(),
        json!({"revoked": true})
    );
    let revoked = srv.requests();
    let revoke = revoked.iter().find(|r| r.path == "/revoke").unwrap();
    assert!(revoke.body.contains("token=rt-SECRET-chatgpt-1"));
    assert!(!chatgpt_status(&client).await.unwrap().signed_in);
    let file = std::fs::read_to_string(dir.0.join("chatgpt-auth.json")).unwrap();
    assert!(!file.contains("SECRET"), "no token left in the file");
    assert!(file.contains("oaiapp_TEST-client-1"), "the client id stays");
    let secrets: SharedSecrets = MemoryStore::shared();
    assert_eq!(
        models(&client, &secrets, ProviderId::Chatgpt).await,
        Err(AiFailure::new("auth", "Inicia sesión en ChatGPT"))
    );
    assert_eq!(
        key_status(&client, &secrets, ProviderId::Chatgpt).await,
        Ok(false)
    );
}

#[tokio::test]
async fn an_unconfirmed_chatgpt_sign_out_still_forgets_the_tokens() {
    // No discovery: the revocation cannot be confirmed, the tokens go anyway.
    let srv = server(vec![]).await;
    let dir = Scratch::new();
    let client = chatgpt_client(&srv, &dir, Account::SignedIn);
    assert_eq!(
        chatgpt_sign_out(&client).await.map(|s| s.revoked),
        Ok(false)
    );
    assert!(!chatgpt_status(&client).await.unwrap().signed_in);
    let file = std::fs::read_to_string(dir.0.join("chatgpt-auth.json")).unwrap();
    assert!(!file.contains("SECRET"), "no token left in the file");
    assert_eq!(
        chatgpt_sign_out(&srv.client()).await,
        Err(AiFailure::new(
            "config",
            "PlexusTyping no tiene una carpeta donde guardar la sesión de ChatGPT"
        ))
    );
}

#[tokio::test]
async fn chatgpt_models_carry_their_display_names_and_store_the_slug() {
    let list = json!({"models": [
        {"slug": "gpt-6.2-sol", "display_name": "GPT-6.2 Sol", "visibility": "list"},
        {"slug": "hidden", "display_name": "Hidden", "visibility": "hide"},
        {"slug": "gpt-6.1-sol", "display_name": "GPT-6.1 Sol", "visibility": "list"},
        {"slug": "gpt-unnamed", "visibility": "list"}
    ]});
    let srv = server(vec![("/v1/models", 200, list.to_string())]).await;
    let dir = Scratch::new();
    let signed_out = chatgpt_client(&srv, &dir, Account::SignedOut);
    assert_eq!(
        chatgpt_models(&signed_out).await,
        Err(AiFailure::new("auth", "Inicia sesión en ChatGPT"))
    );
    assert!(srv.requests().is_empty());
    let dir = Scratch::new();
    let client = chatgpt_client(&srv, &dir, Account::SignedIn);
    let got = chatgpt_models(&client).await.unwrap();
    assert_eq!(
        serde_json::to_value(&got).unwrap(),
        json!([
            {"id": "gpt-6.2-sol", "name": "GPT-6.2 Sol"},
            {"id": "gpt-6.1-sol", "name": "GPT-6.1 Sol"},
            {"id": "gpt-unnamed", "name": "gpt-unnamed"}
        ])
    );
    assert_eq!(srv.requests().len(), 1);
}
