//! The ChatGPT provider (S3, S4) against a loopback mock of api.openai.com and
//! auth.openai.com: the streamed Responses request, its SSE events, the plan
//! errors, the 401 refresh-and-retry, the model list and the fallback chain.
//! The credential file lives in a scratch folder under `target/` that each test
//! removes.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::{Reply, chat_reply, config, mock, small_request, text_json};
use plexustyping_ai::chatgpt_auth::{CREDENTIALS_FILE, ChatgptAuth};
use plexustyping_ai::{ApiKey, ProviderConfig, ProviderError, ProviderId, Usage, build_prompt};
use serde_json::{Value, json};

const RESPONSES: &str = "/v1/responses";
const MODELS: &str = "/v1/models";
const TOKEN: &str = "/api/accounts/oauth/token";
const ACCESS: &str = "at-SECRET-chatgpt-111";
const ACCESS_2: &str = "at-SECRET-chatgpt-222";
const REFRESH: &str = "rt-SECRET-chatgpt-111";
const REFRESH_2: &str = "rt-SECRET-chatgpt-222";
const PLAN: &str = "chatgpt.tokens.use.direct email offline_access openid profile resource.invoke";
const MODEL: &str = "gpt-6.1-sol";
/// Fields the plan API rejects (developers.openai.com/siwc/token-sharing-open-source/preview-limitations).
const FORBIDDEN: [&str; 16] = [
    "background",
    "conversation",
    "max_output_tokens",
    "max_tool_calls",
    "metadata",
    "moderation",
    "multi_agent",
    "prompt",
    "prompt_cache_retention",
    "safety_identifier",
    "temperature",
    "top_logprobs",
    "top_p",
    "truncation",
    "user",
    "previous_response_id",
];

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// A scratch folder under `target/tmp`, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "chatgpt-provider-{}-{}",
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

/// A signed-in account with plan use whose access token is valid for an hour.
fn signed_in(dir: &Scratch) -> PathBuf {
    let path = dir.0.join(CREDENTIALS_FILE);
    let record = json!({
        "hostId": "urn:uuid:00000000-0000-4000-8000-000000000000",
        "clientId": "oaiapp_TEST-client-1",
        "subject": "user-sub-1",
        "email": "ana@example.com",
        "accessToken": ACCESS,
        "refreshToken": REFRESH,
        "scopes": PLAN.split(' ').collect::<Vec<_>>(),
        "expiresAt": now() + 3600
    });
    std::fs::write(&path, record.to_string()).unwrap();
    path
}

fn record(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn chatgpt(key: &str) -> ProviderConfig {
    let mut cfg = ProviderConfig::new(ProviderId::Chatgpt, ApiKey::new(key).unwrap());
    cfg.model = MODEL.into();
    cfg.timeout = Duration::from_secs(5);
    cfg
}

/// Server-sent events, one `event:`/`data:` block per JSON event.
fn sse(events: &[Value]) -> String {
    events
        .iter()
        .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
        .collect()
}

fn stream(events: &[Value]) -> Reply {
    Reply::status(200, &sse(events)).header("X-Accel-Buffering: no")
}

fn delta(text: &str) -> Value {
    json!({"type": "response.output_text.delta", "delta": text})
}

fn completed_response(text: &str) -> Value {
    json!({
        "status": "completed",
        "output": [{"type": "message", "role": "assistant",
                    "content": [{"type": "output_text", "text": text}]}],
        "usage": {"input_tokens": 77, "output_tokens": 21}
    })
}

/// A stream answering `text` in two deltas and a full `response.completed`.
fn answer(text: &str) -> Reply {
    let (a, b) = text.split_at(text.len() / 2);
    stream(&[
        json!({"type": "response.created", "response": {"status": "in_progress"}}),
        delta(a),
        delta(b),
        json!({"type": "response.completed", "response": completed_response(text)}),
    ])
}

fn failed(code: &str) -> Reply {
    stream(&[
        json!({"type": "response.created", "response": {"status": "in_progress"}}),
        delta("{\"te"),
        json!({"type": "response.failed", "response": {"status": "failed",
               "error": {"code": code, "message": format!("plan says no to {ACCESS}")}}}),
    ])
}

fn error_body(code: &str) -> String {
    json!({"error": {"code": code, "param": null, "message": format!("echo {ACCESS}")}}).to_string()
}

fn assert_clean(err: &ProviderError) -> String {
    let shown = format!("{err} {err:?} {}", serde_json::to_string(err).unwrap());
    for secret in [ACCESS, ACCESS_2, REFRESH, REFRESH_2] {
        assert!(!shown.contains(secret), "token leaked: {shown}");
    }
    shown
}

fn message(err: &ProviderError) -> String {
    err.to_string()
}

// ------------------------------------------------------------- chain

const OPENAI_CHAT: &str = "/v1/chat/completions";

/// "si ChatGPT falla por un problema de tu plan … no pasar a otro proveedor" (user
/// decision, following OpenAI: plan usage errors stop inference and are never
/// switched silently to another billing path).
#[tokio::test]
async fn a_plan_error_stops_the_chain_before_any_other_provider() {
    for code in [
        "subscription_sharing_usage_limit_exceeded",
        "subscription_sharing_usage_unavailable",
        "subscription_sharing_user_not_eligible",
        "subscription_sharing_invalid_user",
    ] {
        let server = mock(vec![
            (RESPONSES, failed(code)),
            (OPENAI_CHAT, chat_reply(&text_json("Zapatos azules"))),
        ])
        .await;
        let chain = [chatgpt(ACCESS), config(ProviderId::Openai, "gpt-5-mini")];
        let errors = server
            .client()
            .generate_with_fallback(&chain, &small_request())
            .await
            .unwrap_err();
        assert_eq!(errors.len(), 1, "{code}: {errors:?}");
        assert_eq!(errors[0].0, ProviderId::Chatgpt, "{code}");
        assert_eq!(
            server.hits(OPENAI_CHAT),
            0,
            "{code}: no other provider is billed"
        );
    }
}

#[tokio::test]
async fn a_chatgpt_network_failure_still_moves_on_to_the_next_provider() {
    let server = mock(vec![
        (RESPONSES, Reply::status(502, "{}")),
        (OPENAI_CHAT, chat_reply(&text_json("Zapatos azules"))),
    ])
    .await;
    let chain = [chatgpt(ACCESS), config(ProviderId::Openai, "gpt-5-mini")];
    let got = server
        .client()
        .generate_with_fallback(&chain, &small_request())
        .await
        .unwrap();
    assert_eq!(got.provider, ProviderId::Openai);
}

// ------------------------------------------------------------- request

#[tokio::test]
async fn the_request_is_a_streamed_unstored_responses_call_with_only_supported_fields() {
    let server = mock(vec![(RESPONSES, answer(&text_json("Zapatos azules")))]).await;
    let req = small_request();

    let got = server
        .client()
        .generate(&chatgpt(ACCESS), &req)
        .await
        .unwrap();

    let r = &server.requests()[0];
    assert_eq!((r.method.as_str(), r.path.as_str()), ("POST", RESPONSES));
    assert_eq!(
        r.header("authorization"),
        Some(&*format!("Bearer {ACCESS}"))
    );
    let body = r.json();
    let mut keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["input", "model", "store", "stream"], "{body}");
    assert_eq!(body["model"], MODEL);
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    assert_eq!(
        body["input"],
        json!([{"role": "user", "content": build_prompt(&req)}]),
        "input is an array with the prompt as the user message"
    );
    for field in FORBIDDEN {
        assert!(body.get(field).is_none(), "{field} must not be sent");
    }
    assert!(!r.body.contains("\"system\""), "no system-role items");
    assert_eq!(got.text, "Zapatos azules");
    assert_eq!(got.provider, ProviderId::Chatgpt);
    assert_eq!(got.model, MODEL);
    assert_eq!(
        got.usage,
        Usage {
            input_tokens: 77,
            output_tokens: 21
        }
    );
}

#[test]
fn chatgpt_metadata_and_debug_hide_the_token() {
    assert_eq!(ProviderId::Chatgpt.label(), "ChatGPT (tu suscripción)");
    assert_eq!(ProviderId::Chatgpt.default_model(), "");
    assert_eq!(
        serde_json::to_string(&ProviderId::Chatgpt).unwrap(),
        "\"chatgpt\""
    );
    assert!(ProviderId::ALL.contains(&ProviderId::Chatgpt));
    let cfg = chatgpt(ACCESS);
    let shown = format!("{cfg:?} {cfg:#?}");
    assert!(!shown.contains(ACCESS), "{shown}");
}

// ------------------------------------------------------------- stream

#[tokio::test]
async fn the_text_comes_from_the_deltas_when_the_completed_event_has_no_output() {
    let text = text_json("Zorro azul");
    let (a, b) = text.split_at(5);
    let server = mock(vec![(
        RESPONSES,
        stream(&[
            delta(a),
            delta(b),
            json!({"type": "response.completed",
                   "response": {"status": "completed", "output": [], "usage": {"input_tokens": 3, "output_tokens": 4}}}),
        ]),
    )])
    .await;

    let got = server
        .client()
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap();

    assert_eq!(got.text, "Zorro azul");
    assert_eq!(
        got.usage,
        Usage {
            input_tokens: 3,
            output_tokens: 4
        }
    );
}

#[tokio::test]
async fn crlf_streams_and_multi_line_data_are_read() {
    let completed =
        json!({"type": "response.completed", "response": completed_response(&text_json("Zapato"))});
    let body = format!(
        ": keep-alive\r\n\r\nevent: response.output_text.delta\r\ndata: {}\r\n\r\nevent: response.completed\r\ndata: {}\r\n\r\n",
        delta("x"),
        completed
    );
    let server = mock(vec![(RESPONSES, Reply::status(200, &body))]).await;

    let got = server
        .client()
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap();

    assert_eq!(got.text, "Zapato");
}

#[tokio::test]
async fn an_incomplete_response_is_a_cut_answer() {
    let server = mock(vec![(
        RESPONSES,
        stream(&[
            delta("{\"text\": \"Zapa"),
            json!({"type": "response.incomplete", "response": {"status": "incomplete",
                   "incomplete_details": {"reason": "max_output_tokens"}, "output": []}}),
        ]),
    )])
    .await;

    let err = server
        .client()
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::BadResponse { .. }), "{err:?}");
    assert_eq!(message(&err), "The answer from ChatGPT was cut off.");
}

#[tokio::test]
async fn a_stream_that_stops_before_a_terminal_event_is_not_a_success() {
    let server = mock(vec![(
        RESPONSES,
        stream(&[delta(&text_json("Zapatos azules"))]),
    )])
    .await;

    let err = server
        .client()
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ProviderError::Network {
            message: "La respuesta de ChatGPT se cortó antes de terminar; inténtalo de nuevo."
                .into()
        }
    );
}

#[tokio::test]
async fn plan_errors_in_the_stream_and_in_error_bodies_get_spanish_messages() {
    let limit = ProviderError::RateLimit {
        message: "Has alcanzado el límite de tu plan de ChatGPT".into(),
    };
    let unavailable = ProviderError::RateLimit {
        message: "ChatGPT no pudo comprobar el uso de tu plan ahora; inténtalo más tarde".into(),
    };
    let not_eligible = ProviderError::Config {
        message: "Tu plan de ChatGPT no se puede usar en PlexusTyping (cuenta, espacio de trabajo o política)".into(),
    };
    let not_authorized = ProviderError::Config {
        message: "ChatGPT no autorizó esta petición de PlexusTyping".into(),
    };
    let invalid_user = ProviderError::Auth {
        message: "ChatGPT no pudo validar tu sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT".into(),
    };
    let cases: Vec<(&str, u16, ProviderError)> = vec![
        ("subscription_sharing_usage_limit_exceeded", 429, limit),
        (
            "subscription_sharing_usage_unavailable",
            503,
            unavailable.clone(),
        ),
        ("subscription_sharing_user_unavailable", 503, unavailable),
        ("subscription_sharing_user_not_eligible", 403, not_eligible),
        (
            "subscription_sharing_route_not_supported",
            403,
            not_authorized.clone(),
        ),
        (
            "chatpass_v2_scope_not_authorized",
            403,
            not_authorized.clone(),
        ),
        (
            "chatpass_v2_invalid_authorization_context",
            403,
            not_authorized,
        ),
        ("subscription_sharing_invalid_user", 401, invalid_user),
    ];
    for (code, status, expected) in cases {
        // In the stream, after it has begun.
        let server = mock(vec![(RESPONSES, failed(code))]).await;
        let err = server
            .client()
            .generate(&chatgpt(ACCESS), &small_request())
            .await
            .unwrap_err();
        assert_eq!(err, expected, "stream {code}");
        assert_clean(&err);
        // As the error body of a request that never streamed (no auth to refresh with).
        let server = mock(vec![(RESPONSES, Reply::status(status, &error_body(code)))]).await;
        let err = server
            .client()
            .generate(&chatgpt(ACCESS), &small_request())
            .await
            .unwrap_err();
        assert_eq!(err, expected, "HTTP {status} {code}");
        assert_clean(&err);
    }
}

#[tokio::test]
async fn other_failures_name_chatgpt_in_spanish_and_never_echo_the_token() {
    let unsupported = json!({"error": {"code": "subscription_sharing_unsupported_capability",
                                        "param": "tools", "message": ACCESS}})
    .to_string();
    let cases: Vec<(Reply, ProviderError)> = vec![
        (
            Reply::status(400, &unsupported),
            ProviderError::Config {
                message: "ChatGPT no admite \"tools\" en esta petición. Prueba con otro modelo"
                    .into(),
            },
        ),
        (
            Reply::status(
                403,
                &json!({"detail": format!("region {ACCESS}")}).to_string(),
            ),
            ProviderError::Auth {
                message: "ChatGPT no permitió esta petición (región, permisos o política)".into(),
            },
        ),
        (
            Reply::status(429, "{}"),
            ProviderError::RateLimit {
                message: "ChatGPT está limitando las peticiones; inténtalo en un momento".into(),
            },
        ),
        (
            Reply::status(503, &json!({"detail": "direct routing off"}).to_string()),
            ProviderError::Network {
                message: "ChatGPT no está disponible ahora (HTTP 503)".into(),
            },
        ),
        (
            Reply::status(404, &error_body("model_not_found")),
            ProviderError::Config {
                message:
                    "ChatGPT rechazó la petición (HTTP 404, model_not_found). Revisa el modelo"
                        .into(),
            },
        ),
        (
            Reply::status(302, "").header("Location: https://chatgpt.com/backend-api"),
            ProviderError::Config {
                message: "ChatGPT redirigió la petición (HTTP 302); no se siguen redirecciones"
                    .into(),
            },
        ),
        (
            stream(&[json!({"type": "error", "code": "server_error", "message": ACCESS})]),
            ProviderError::BadResponse {
                message: "ChatGPT no pudo responder (server_error)".into(),
            },
        ),
    ];
    for (reply, expected) in cases {
        let server = mock(vec![(RESPONSES, reply)]).await;
        let err = server
            .client()
            .generate(&chatgpt(ACCESS), &small_request())
            .await
            .unwrap_err();
        assert_eq!(err, expected);
        assert_clean(&err);
    }
}

#[tokio::test]
async fn an_unreachable_api_is_a_spanish_network_error_without_url_or_token() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = plexustyping_ai::AiClient::new(plexustyping_ai::Endpoints::loopback(&base));

    let err = client
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap_err();

    let ProviderError::Network { message } = &err else {
        panic!("{err:?}");
    };
    assert!(
        message.starts_with("No se pudo conectar con ChatGPT"),
        "{message}"
    );
    assert!(!message.contains("127.0.0.1"), "{message}");
    assert_clean(&err);
}

// ------------------------------------------------------- refresh on 401

#[tokio::test]
async fn a_rejected_token_is_refreshed_once_and_the_request_retried_once() {
    let dir = Scratch::new();
    let path = signed_in(&dir);
    let server = mock(vec![
        (
            RESPONSES,
            Reply::status(401, &json!({"detail": "expired"}).to_string()),
        ),
        (
            TOKEN,
            Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                             "token_type": "Bearer", "expires_in": 3600, "scope": PLAN})),
        ),
        (RESPONSES, answer(&text_json("Zapatos azules"))),
    ])
    .await;
    let auth = Arc::new(ChatgptAuth::new(&server.endpoints(), Some(path.clone())));
    let client = server.client().with_chatgpt(auth);

    let got = client
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap();

    assert_eq!(got.text, "Zapatos azules");
    let calls: Vec<(String, Option<String>)> = server
        .requests()
        .iter()
        .map(|r| {
            (
                r.path.clone(),
                r.header("authorization").map(str::to_string),
            )
        })
        .collect();
    assert_eq!(
        calls,
        [
            (RESPONSES.to_string(), Some(format!("Bearer {ACCESS}"))),
            (TOKEN.to_string(), None),
            (RESPONSES.to_string(), Some(format!("Bearer {ACCESS_2}"))),
        ]
    );
    let saved = record(&path);
    assert_eq!(saved["accessToken"], ACCESS_2, "the rotated pair is saved");
    assert_eq!(saved["refreshToken"], REFRESH_2);
}

#[tokio::test]
async fn a_second_401_ends_the_attempt_without_another_refresh() {
    let dir = Scratch::new();
    let path = signed_in(&dir);
    let server = mock(vec![
        (
            RESPONSES,
            Reply::status(401, &error_body("invalid_api_key")),
        ),
        (
            TOKEN,
            Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                             "expires_in": 3600, "scope": PLAN})),
        ),
        (
            RESPONSES,
            Reply::status(401, &error_body("invalid_api_key")),
        ),
        (
            TOKEN,
            Reply::ok(json!({"access_token": "never", "expires_in": 3600})),
        ),
        (RESPONSES, answer(&text_json("never"))),
    ])
    .await;
    let auth = Arc::new(ChatgptAuth::new(&server.endpoints(), Some(path)));
    let client = server.client().with_chatgpt(auth);

    let err = client
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ProviderError::Auth {
            message:
                "ChatGPT no aceptó la sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT"
                    .into()
        }
    );
    assert_clean(&err);
    assert_eq!(server.hits(RESPONSES), 2, "one retry only");
    assert_eq!(server.hits(TOKEN), 1, "one refresh only");
}

#[tokio::test]
async fn a_refresh_that_fails_after_a_401_is_reported_without_a_retry() {
    let dir = Scratch::new();
    let path = signed_in(&dir);
    let server = mock(vec![
        (RESPONSES, Reply::status(401, "{}")),
        (
            TOKEN,
            Reply::status(400, &json!({"error": "invalid_grant"}).to_string()),
        ),
    ])
    .await;
    let auth = Arc::new(ChatgptAuth::new(&server.endpoints(), Some(path.clone())));
    let client = server.client().with_chatgpt(auth);

    let err = client
        .generate(&chatgpt(ACCESS), &small_request())
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ProviderError::Auth {
            message: "Tu sesión de ChatGPT ha caducado. Vuelve a iniciar sesión".into()
        }
    );
    assert_eq!(server.hits(RESPONSES), 1);
    assert_eq!(
        record(&path)["accessToken"],
        Value::Null,
        "terminal: forgotten"
    );
}

#[tokio::test]
async fn a_token_another_request_already_replaced_is_not_refreshed_again() {
    let dir = Scratch::new();
    let path = signed_in(&dir);
    let server = mock(vec![
        (RESPONSES, Reply::status(401, "{}")),
        (RESPONSES, answer(&text_json("Zapatos azules"))),
    ])
    .await;
    let auth = Arc::new(ChatgptAuth::new(&server.endpoints(), Some(path)));
    let client = server.client().with_chatgpt(auth);

    // The chain was built with an older token; the saved one is already newer.
    let got = client
        .generate(&chatgpt("at-SECRET-older-000"), &small_request())
        .await
        .unwrap();

    assert_eq!(got.text, "Zapatos azules");
    assert_eq!(server.hits(TOKEN), 0);
    let last = server.requests().pop().unwrap();
    assert_eq!(
        last.header("authorization"),
        Some(&*format!("Bearer {ACCESS}"))
    );
}

// ------------------------------------------------------------- models

fn model_list() -> Reply {
    Reply::ok(json!({"models": [
        {"slug": "gpt-6.2-sol", "display_name": "GPT-6.2 Sol", "visibility": "list"},
        {"slug": "internal-x", "display_name": "Hidden", "visibility": "hide"},
        {"slug": "gpt-6.1-sol", "display_name": "GPT-6.1 Sol", "visibility": "list"},
        {"display_name": "No slug", "visibility": "list"},
        {"slug": "has space", "visibility": "list"},
        {"slug": "gpt-6.2-sol", "visibility": "list"},
        {"slug": "gpt-5-luna", "visibility": "list"}
    ]}))
}

#[tokio::test]
async fn the_model_list_keeps_listed_slugs_in_server_order() {
    let server = mock(vec![(MODELS, model_list())]).await;

    let got = server
        .client()
        .list_models(ProviderId::Chatgpt, Some(&ApiKey::new(ACCESS).unwrap()))
        .await
        .unwrap();

    assert_eq!(got, ["gpt-6.2-sol", "gpt-6.1-sol", "gpt-5-luna"]);
    let r = &server.requests()[0];
    assert_eq!((r.method.as_str(), r.path.as_str()), ("GET", MODELS));
    assert_eq!(
        r.header("authorization"),
        Some(&*format!("Bearer {ACCESS}"))
    );
}

#[tokio::test]
async fn the_model_list_uses_the_signed_in_account_and_refreshes_once_on_401() {
    let dir = Scratch::new();
    let path = signed_in(&dir);
    let server = mock(vec![
        (MODELS, Reply::status(401, "{}")),
        (
            TOKEN,
            Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                             "expires_in": 3600, "scope": PLAN})),
        ),
        (MODELS, model_list()),
    ])
    .await;
    let auth = Arc::new(ChatgptAuth::new(&server.endpoints(), Some(path)));
    let client = server.client().with_chatgpt(auth);

    let got = client.list_models(ProviderId::Chatgpt, None).await.unwrap();

    assert_eq!(got, ["gpt-6.2-sol", "gpt-6.1-sol", "gpt-5-luna"]);
    let auths: Vec<Option<String>> = server
        .requests()
        .iter()
        .filter(|r| r.path == MODELS)
        .map(|r| r.header("authorization").map(str::to_string))
        .collect();
    assert_eq!(
        auths,
        [
            Some(format!("Bearer {ACCESS}")),
            Some(format!("Bearer {ACCESS_2}"))
        ]
    );
}

#[tokio::test]
async fn without_a_token_or_a_sign_in_no_model_list_is_requested() {
    let server = mock(vec![(MODELS, model_list())]).await;

    let err = server
        .client()
        .list_models(ProviderId::Chatgpt, None)
        .await
        .unwrap_err();

    assert_eq!(
        err,
        ProviderError::Auth {
            message: "Inicia sesión en ChatGPT".into()
        }
    );
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn a_model_list_that_is_not_a_plan_catalogue_is_unreadable() {
    let server = mock(vec![(
        MODELS,
        Reply::ok(json!({"data": [{"id": "gpt-5"}]})),
    )])
    .await;

    let err = server
        .client()
        .list_models(ProviderId::Chatgpt, Some(&ApiKey::new(ACCESS).unwrap()))
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::BadResponse { .. }), "{err:?}");
}

// ------------------------------------------------------------- chain

#[tokio::test]
async fn an_invalid_chatgpt_text_gets_the_free_retry_like_any_provider() {
    let server = mock(vec![
        (RESPONSES, answer(&text_json("**sin la letra**"))),
        (RESPONSES, answer(&text_json("Zapatos azules"))),
    ])
    .await;

    let got = server
        .client()
        .generate_with_fallback(&[chatgpt(ACCESS)], &small_request())
        .await
        .unwrap();

    assert_eq!(got.provider, ProviderId::Chatgpt);
    assert!(got.free_text, "the second attempt asks for any text");
    assert_eq!(got.failed_attempts.len(), 1);
    assert!(matches!(
        got.failed_attempts[0],
        (ProviderId::Chatgpt, ProviderError::Invalid { .. })
    ));
    let second = server.requests()[1].json();
    let prompt = second["input"][0]["content"].as_str().unwrap();
    assert!(!prompt.is_empty());
}

#[tokio::test]
async fn a_slow_chatgpt_stream_times_out() {
    let server = mock(vec![(
        RESPONSES,
        answer(&text_json("Zapatos azules")).delayed(Duration::from_secs(3)),
    )])
    .await;
    let mut cfg = chatgpt(ACCESS);
    cfg.timeout = Duration::from_millis(300);

    let err = server
        .client()
        .generate(&cfg, &small_request())
        .await
        .unwrap_err();

    assert_eq!(err, ProviderError::Timeout);
}
