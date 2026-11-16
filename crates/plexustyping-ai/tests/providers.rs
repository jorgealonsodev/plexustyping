//! `AiClient` against a loopback mock server (never a real endpoint).

mod common;

use std::time::{Duration, Instant};

use common::{KEY, Reply, chat_reply, config, key, mock, small_request, text_json};
use plexustyping_ai::{
    ProviderError, ProviderId, TextProvider, Usage, ValidationIssue, build_prompt,
};
use serde_json::json;

const TEXT: &str = "Zapatos  azules\u{2026}";
const NORMALIZED: &str = "Zapatos azules...";

fn claude_reply(content: &str) -> Reply {
    Reply::ok(json!({
        "content": [{"type": "text", "text": content}],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 120, "output_tokens": 45}
    }))
}

fn models_dev() -> Reply {
    Reply::ok(json!({"opencode-go": {"models": {
        "minimax-m3": {"provider": {"npm": "@ai-sdk/anthropic"}},
        "gpt-oss": {"provider": {"npm": "@ai-sdk/openai"}},
        "kimi-k2.6": {}
    }}}))
}

fn assert_no_key(err: &ProviderError) {
    let shown = format!("{err} {err:?} {}", serde_json::to_string(err).unwrap());
    assert!(!shown.contains(KEY), "{shown}");
}

#[tokio::test]
async fn claude_generate_round_trip() {
    let server = mock(vec![("/v1/messages", claude_reply(&text_json(TEXT)))]).await;
    let req = small_request();

    let got = server
        .client()
        .generate(&config(ProviderId::Claude, "claude-haiku-5-5"), &req)
        .await
        .unwrap();

    let r = &server.requests()[0];
    assert_eq!(
        (r.method.as_str(), r.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(r.header("x-api-key"), Some(KEY));
    assert_eq!(r.header("anthropic-version"), Some("2023-06-01"));
    let body = r.json();
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": build_prompt(&req)}])
    );
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    assert_eq!(got.text, NORMALIZED);
    assert_eq!(got.provider, ProviderId::Claude);
    assert_eq!(got.model, "claude-haiku-5-5");
    assert_eq!(
        got.usage,
        Usage {
            input_tokens: 120,
            output_tokens: 45
        }
    );
}

#[tokio::test]
async fn openai_generate_round_trip() {
    let fenced = format!("```json\n{}\n```", text_json(TEXT));
    let server = mock(vec![("/v1/chat/completions", chat_reply(&fenced))]).await;
    let req = small_request();

    let got = server
        .client()
        .generate(&config(ProviderId::Openai, "gpt-5-mini"), &req)
        .await
        .unwrap();

    let r = &server.requests()[0];
    assert_eq!(r.path, "/v1/chat/completions");
    assert_eq!(
        r.header("authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    let body = r.json();
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": build_prompt(&req)}])
    );
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(got.text, NORMALIZED);
    assert_eq!(
        got.usage,
        Usage {
            input_tokens: 80,
            output_tokens: 30
        }
    );
}

#[tokio::test]
async fn gemini_generate_round_trip() {
    let path = "/v1beta/models/gemini-3.5-flash-lite:generateContent";
    let server = mock(vec![(
        path,
        Reply::ok(json!({
            "candidates": [{"content": {"parts": [{"text": text_json(TEXT)}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 70, "candidatesTokenCount": 25}
        })),
    )])
    .await;
    let req = small_request();

    let got = server
        .client()
        .generate(&config(ProviderId::Gemini, "gemini-3.5-flash-lite"), &req)
        .await
        .unwrap();

    let r = &server.requests()[0];
    assert_eq!(r.path, path);
    assert_eq!(r.header("x-goog-api-key"), Some(KEY));
    assert_eq!(r.header("authorization"), None);
    let body = r.json();
    assert_eq!(body["contents"][0]["parts"][0]["text"], build_prompt(&req));
    assert_eq!(
        body["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert_eq!(got.text, NORMALIZED);
    assert_eq!(got.provider, ProviderId::Gemini);
    assert_eq!(
        got.usage,
        Usage {
            input_tokens: 70,
            output_tokens: 25
        }
    );
}

#[tokio::test]
async fn opencode_go_picks_the_format_from_models_dev_once() {
    let responses = Reply::ok(json!({
        "status": "completed",
        "output": [{"type": "message", "content": [{"type": "output_text", "text": text_json(TEXT)}]}],
        "usage": {"input_tokens": 3, "output_tokens": 2}
    }));
    let server = mock(vec![
        ("/api.json", models_dev()),
        ("/inference/go/v1/messages", claude_reply(&text_json(TEXT))),
        ("/inference/go/v1/responses", responses),
        (
            "/inference/go/v1/chat/completions",
            chat_reply(&text_json(TEXT)),
        ),
    ])
    .await;
    let client = server.client();
    let req = small_request();

    for model in ["minimax-m3", "gpt-oss", "kimi-k2.6"] {
        let got = client
            .generate(&config(ProviderId::OpencodeGo, model), &req)
            .await
            .unwrap();
        assert_eq!(got.text, NORMALIZED, "{model}");
        assert_eq!(got.model, model);
    }

    assert_eq!(server.hits("/api.json"), 1, "catalogue cached in memory");
    let calls: Vec<_> = server
        .requests()
        .into_iter()
        .filter(|r| r.path.starts_with("/inference"))
        .collect();
    let paths: Vec<&str> = calls.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/inference/go/v1/messages",
            "/inference/go/v1/responses",
            "/inference/go/v1/chat/completions"
        ]
    );
    assert_eq!(calls[0].header("x-api-key"), Some(KEY));
    let session = calls[0].header("x-opencode-session").unwrap().to_string();
    assert!(!session.is_empty() && session.len() <= 128);
    for call in &calls {
        assert_eq!(
            call.header("authorization"),
            Some(format!("Bearer {KEY}").as_str())
        );
        assert_eq!(
            call.header("x-opencode-session"),
            Some(session.as_str()),
            "one session per client"
        );
    }
}

#[tokio::test]
async fn opencode_go_uses_given_session_and_chat_without_catalogue() {
    let server = mock(vec![
        ("/api.json", Reply::status(503, "{}")),
        (
            "/inference/go/v1/chat/completions",
            chat_reply(&text_json(TEXT)),
        ),
    ])
    .await;
    let mut cfg = config(ProviderId::OpencodeGo, "minimax-m3");
    cfg.session_id = Some("practice.42".into());

    server
        .client()
        .generate(&cfg, &small_request())
        .await
        .unwrap();

    let r = server
        .requests()
        .into_iter()
        .find(|r| r.path.starts_with("/inference"))
        .unwrap();
    assert_eq!(r.path, "/inference/go/v1/chat/completions");
    assert_eq!(r.header("x-opencode-session"), Some("practice.42"));
}

#[tokio::test]
async fn list_models_per_provider() {
    let server = mock(vec![
        (
            "/v1/models",
            Reply::ok(json!({"data": [{"id": "claude-haiku-5-5"}, {"id": "claude-opus-5-5"}]})),
        ),
        (
            "/v1/models",
            Reply::ok(json!({"data": [{"id": "gpt-5-mini"}, {"id": "whisper-1"}]})),
        ),
        (
            "/v1beta/models",
            Reply::ok(json!({"models": [
                {"name": "models/gemini-3.5-flash-lite", "supportedGenerationMethods": ["generateContent"]},
                {"name": "models/embedding-001", "supportedGenerationMethods": ["embedContent"]}
            ]})),
        ),
        (
            "/inference/go/v1/models",
            Reply::ok(json!({"data": [{"id": "kimi-k2.6"}]})),
        ),
    ])
    .await;
    let client = server.client();
    let k = key();

    assert_eq!(
        client
            .list_models(ProviderId::Claude, Some(&k))
            .await
            .unwrap(),
        ["claude-haiku-5-5", "claude-opus-5-5"]
    );
    assert_eq!(
        client
            .list_models(ProviderId::Openai, Some(&k))
            .await
            .unwrap(),
        ["gpt-5-mini"]
    );
    assert_eq!(
        client
            .list_models(ProviderId::Gemini, Some(&k))
            .await
            .unwrap(),
        ["gemini-3.5-flash-lite"]
    );
    assert_eq!(
        client
            .list_models(ProviderId::OpencodeGo, None)
            .await
            .unwrap(),
        ["kimi-k2.6"]
    );

    let r = server.requests();
    assert_eq!(r[0].path, "/v1/models?limit=1000");
    assert_eq!(r[0].method, "GET");
    assert_eq!(r[0].header("x-api-key"), Some(KEY));
    assert_eq!(r[0].header("anthropic-version"), Some("2023-06-01"));
    assert_eq!(r[1].path, "/v1/models");
    assert_eq!(
        r[1].header("authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(r[2].path, "/v1beta/models?pageSize=1000");
    assert_eq!(r[2].header("x-goog-api-key"), Some(KEY));
    assert_eq!(r[3].path, "/inference/go/v1/models");
    assert_eq!(r[3].header("authorization"), None, "public list, no key");
}

#[tokio::test]
async fn list_models_without_key_sends_nothing() {
    let server = mock(vec![]).await;
    for id in [ProviderId::Claude, ProviderId::Openai, ProviderId::Gemini] {
        assert_eq!(
            server.client().list_models(id, None).await,
            Err(ProviderError::NoKey)
        );
    }
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn slow_provider_times_out() {
    let server = mock(vec![(
        "/v1/chat/completions",
        chat_reply(&text_json(TEXT)).delayed(Duration::from_secs(3)),
    )])
    .await;
    let mut cfg = config(ProviderId::Openai, "gpt-5-mini");
    cfg.timeout = Duration::from_millis(200);

    let start = Instant::now();
    let err = server
        .client()
        .generate(&cfg, &small_request())
        .await
        .unwrap_err();

    assert_eq!(err, ProviderError::Timeout);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let server = mock(vec![]).await;
    let location = format!("Location: {}/elsewhere", server.base);
    let server2 = mock(vec![(
        "/v1/chat/completions",
        Reply::status(302, "").header(&location),
    )])
    .await;

    let err = server2
        .client()
        .generate(&config(ProviderId::Openai, "gpt-5-mini"), &small_request())
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::Config { .. }), "{err:?}");
    assert!(
        server.requests().is_empty(),
        "redirect target never contacted"
    );
    assert_eq!(server2.requests().len(), 1);
}

#[tokio::test]
async fn provider_errors_are_mapped_and_never_echo_the_key() {
    let echo =
        json!({"error": {"message": format!("Incorrect API key provided: {KEY}")}}).to_string();
    let server = mock(vec![
        ("/v1/chat/completions", Reply::status(400, &echo)),
        ("/v1/chat/completions", Reply::status(401, &echo)),
        ("/v1/chat/completions", Reply::status(429, "{}")),
        (
            "/v1/chat/completions",
            Reply::status(503, "<html>down</html>"),
        ),
    ])
    .await;
    let client = server.client();
    let cfg = config(ProviderId::Openai, "gpt-5-mini");
    let req = small_request();

    let config_err = client.generate(&cfg, &req).await.unwrap_err();
    match &config_err {
        ProviderError::Config { message } => assert!(message.contains("***"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
    let auth = client.generate(&cfg, &req).await.unwrap_err();
    assert!(matches!(auth, ProviderError::Auth { .. }), "{auth:?}");
    let limited = client.generate(&cfg, &req).await.unwrap_err();
    assert!(
        matches!(limited, ProviderError::RateLimit { .. }),
        "{limited:?}"
    );
    let down = client.generate(&cfg, &req).await.unwrap_err();
    assert!(matches!(down, ProviderError::Network { .. }), "{down:?}");
    for err in [config_err, auth, limited, down] {
        assert_no_key(&err);
    }
}

#[tokio::test]
async fn gemini_bad_key_is_auth() {
    let body = r#"{"error":{"code":400,"message":"API key not valid.","status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}"#;
    let server = mock(vec![("/v1beta/models", Reply::status(400, body))]).await;

    let err = server
        .client()
        .test_connection(&config(ProviderId::Gemini, "gemini-3.5-flash-lite"))
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::Auth { .. }), "{err:?}");
}

#[tokio::test]
async fn connection_refused_is_network_without_url_or_key() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let client = plexustyping_ai::AiClient::new(plexustyping_ai::Endpoints::loopback(&base));

    let err = client
        .generate(
            &config(ProviderId::Claude, "claude-haiku-5-5"),
            &small_request(),
        )
        .await
        .unwrap_err();

    match &err {
        ProviderError::Network { message } => {
            assert!(!message.contains(&base), "{message}");
        }
        other => panic!("expected Network, got {other:?}"),
    }
    assert_no_key(&err);
}

#[tokio::test]
async fn malformed_and_invalid_texts() {
    let server = mock(vec![
        (
            "/v1/chat/completions",
            chat_reply("Here is your text: hola"),
        ),
        (
            "/v1/chat/completions",
            Reply::ok(json!({"unexpected": true})),
        ),
        (
            "/v1/chat/completions",
            chat_reply(&text_json("**Zapatos** azules")),
        ),
    ])
    .await;
    let client = server.client();
    let cfg = config(ProviderId::Openai, "gpt-5-mini");
    let req = small_request();

    for _ in 0..2 {
        let err = client.generate(&cfg, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::BadResponse { .. }), "{err:?}");
    }
    let err = client.generate(&cfg, &req).await.unwrap_err();
    assert_eq!(
        err,
        ProviderError::Invalid {
            issues: vec![
                ValidationIssue::CharNotAllowed { ch: '*' },
                ValidationIssue::Markdown
            ]
        }
    );
}

#[tokio::test]
async fn test_connection_uses_the_authenticated_model_list() {
    let server = mock(vec![
        (
            "/v1/models",
            Reply::ok(json!({"data": [{"id": "claude-haiku-5-5"}]})),
        ),
        ("/v1/models", Reply::status(401, "{}")),
    ])
    .await;
    let client = server.client();
    let cfg = config(ProviderId::Claude, "claude-haiku-5-5");

    assert_eq!(client.test_connection(&cfg).await, Ok(()));
    let err = client.test_connection(&cfg).await.unwrap_err();
    assert!(matches!(err, ProviderError::Auth { .. }), "{err:?}");
    let r = server.requests();
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|r| r.header("x-api-key") == Some(KEY)));
}

#[tokio::test]
async fn opencode_test_connection_sends_an_authenticated_generation() {
    let server = mock(vec![
        ("/api.json", models_dev()),
        (
            "/inference/go/v1/chat/completions",
            chat_reply(&text_json("ok")),
        ),
        (
            "/inference/go/v1/chat/completions",
            Reply::status(401, "{}"),
        ),
    ])
    .await;
    let client = server.client();
    let cfg = config(ProviderId::OpencodeGo, "kimi-k2.6");

    assert_eq!(client.test_connection(&cfg).await, Ok(()));
    let err = client.test_connection(&cfg).await.unwrap_err();
    assert!(matches!(err, ProviderError::Auth { .. }), "{err:?}");
    assert_eq!(server.hits("/inference/go/v1/chat/completions"), 2);
    let empty = config(ProviderId::OpencodeGo, "");
    assert!(matches!(
        client.test_connection(&empty).await,
        Err(ProviderError::Config { .. })
    ));
}

#[tokio::test]
async fn text_provider_trait_delegates_to_the_client() {
    let server = mock(vec![("/v1/chat/completions", chat_reply(&text_json(TEXT)))]).await;
    let client = server.client();
    let provider: &dyn TextProvider = &client;

    let got = provider
        .generate(&config(ProviderId::Openai, "gpt-5-mini"), &small_request())
        .await
        .unwrap();

    assert_eq!(got.text, NORMALIZED);
}
