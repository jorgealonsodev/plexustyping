//! AI wiring through the plain functions: in-memory store, in-memory keychain
//! and a loopback mock provider. No real provider or keychain is touched.

use std::sync::{Arc, Mutex};

use plexustyping_ai::{AiClient, Endpoints, ProviderError, ProviderId};
use plexustyping_core::{ErrorMode, InputEvent};
use plexustyping_store::{StatsSnapshot, Store, UnitStatRow};
use serde_json::json;

use super::mock::{Mock, mock};
use super::*;
use crate::commands::{StorageStatus, score_session};
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
        profile,
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
        .save_stats(state.profile.id, STATS_MODE, &snapshot)
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
    }
}

/// The corpus text the same request gets with the AI off.
fn corpus_expected(state: &AppState, seed: u64) -> NextText {
    let store = lock_store(&state.store);
    let model = commands::load_model(&store, &state.profile).unwrap();
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
            "La cadena de respaldo necesita de 1 a 4 proveedores",
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
            "La cadena de respaldo necesita de 1 a 4 proveedores",
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
    assert_eq!(list.len(), 4);
    assert_eq!(
        serde_json::to_value(&list[3]).unwrap(),
        json!({"id": "opencode-go", "label": "OpenCode Go", "name": "OpenCode Go", "defaultModel": ""})
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
    assert_eq!(key_status(&secrets, id).await, Ok(false));
    key_save(&secrets, id, format!("  {KEY}\n")).await.unwrap();
    assert_eq!(memory.get("ai:opencode-go").unwrap().as_deref(), Some(KEY));
    assert_eq!(
        memory.label("ai:opencode-go").as_deref(),
        Some("PlexusTyping AI key (OpenCode Go)")
    );
    let status = key_status(&secrets, id).await;
    assert_eq!(status, Ok(true));
    assert_eq!(serde_json::to_string(&status.unwrap()).unwrap(), "true");
    assert_eq!(key_status(&secrets, ProviderId::Claude).await, Ok(false));
    key_delete(&secrets, id).await.unwrap();
    key_delete(&secrets, id).await.unwrap();
    assert_eq!(key_status(&secrets, id).await, Ok(false));
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
    assert_eq!(key_status(&secrets, ProviderId::Claude).await, unavailable);
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
async fn the_request_carries_targets_allowed_chars_and_topic_but_nothing_typed() {
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
            &state.profile,
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
    assert!(prompt.contains("Caracteres objetivo: ñ."), "{prompt}");
    assert!(prompt.contains("Tema: comida."), "{prompt}");
    assert!(prompt.contains("Longitud: 50–60 caracteres."), "{prompt}");
    let (_, listed) = prompt.split_once("Usa solo estos caracteres: ").unwrap();
    let layout = commands::get_layout("es-qwerty").unwrap();
    for ch in layout
        .allowed_chars()
        .into_iter()
        .filter(|c| *c != '\n' && *c != ' ')
    {
        assert!(listed.contains(ch), "allowed {ch:?} missing: {prompt}");
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
    let srv = server(vec![(OPENAI, 500, "{}".into()), (OPENAI, 500, "{}".into())]).await;
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
        }
    );
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
    let first = first_text(&state, "es", 1, DAY).await.unwrap();
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
        let profile = state.profile.clone();
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
    let next = first_text(&state, "es", 1, DAY).await.unwrap();
    assert!(!next.diagnostic);
    let p = TextParams {
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        ..params(1)
    };
    assert_eq!(next.text, next_text(&state, &p).await.unwrap());
    assert!(first_text(&state, "xx", 1, DAY).await.is_err());
}

/// Tauri needs `Send` command futures; a `std::sync::MutexGuard` held across an
/// `.await` would make these futures `!Send` and fail to compile.
#[test]
fn generation_futures_hold_no_store_lock_across_await() {
    fn assert_send<F: std::future::Future + Send>(_: F) {}
    let state = state(MemoryStore::shared(), offline());
    let p = params(1);
    assert_send(next_text(&state, &p));
    assert_send(first_text(&state, "es", 1, DAY));
    assert_send(models(&state.ai, &state.secrets, ProviderId::Claude));
    assert_send(test_connection(
        &state.ai,
        &state.secrets,
        ProviderId::Claude,
        "",
    ));
}
