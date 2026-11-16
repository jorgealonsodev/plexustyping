use std::time::Duration;

use serde_json::{Value, json};

use super::*;

const KEY: &str = "sk-test-SECRET-123";
const PROMPT: &str = "Eres un generador";

fn cfg(id: ProviderId, model: &str) -> ProviderConfig {
    ProviderConfig {
        id,
        api_key: ApiKey::new(KEY).unwrap(),
        model: model.into(),
        timeout: Duration::from_secs(10),
        session_id: Some("sess-1".into()),
    }
}

fn header<'a>(call: &'a Call, name: &str) -> Option<&'a str> {
    call.headers
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.as_str())
}

fn built(id: ProviderId, model: &str, wire: Wire) -> Call {
    build(&cfg(id, model), &Endpoints::production(), PROMPT, wire).unwrap()
}

fn text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"text": {"type": "string"}},
        "required": ["text"],
        "additionalProperties": false
    })
}

fn shape(provider: ProviderId, wire: Wire) -> Shape {
    Shape { provider, wire }
}

fn ok(provider: ProviderId, wire: Wire, body: Value) -> Result<(String, Usage), ProviderError> {
    parse(shape(provider, wire), 200, &body.to_string())
}

fn usage(input_tokens: u64, output_tokens: u64) -> Usage {
    Usage {
        input_tokens,
        output_tokens,
    }
}

// ---- build ----

#[test]
fn claude_build() {
    let call = built(ProviderId::Claude, "claude-haiku-5-5", Wire::Chat);
    assert_eq!(call.url, "https://api.anthropic.com/v1/messages");
    assert_eq!(header(&call, "x-api-key"), Some(KEY));
    assert_eq!(header(&call, "anthropic-version"), Some("2023-06-01"));
    assert_eq!(header(&call, "authorization"), None);
    assert_eq!(
        call.body,
        json!({
            "model": "claude-haiku-5-5",
            "max_tokens": MAX_TOKENS,
            "messages": [{"role": "user", "content": PROMPT}],
            "output_config": {"format": {"type": "json_schema", "schema": text_schema()}}
        })
    );
    assert_eq!(call.shape, shape(ProviderId::Claude, Wire::Messages));
}

#[test]
fn claude_legacy_models_skip_output_config() {
    for model in [
        "claude-3-5-haiku-latest",
        "claude-haiku-4-5",
        "claude-opus-4-1",
    ] {
        let call = built(ProviderId::Claude, model, Wire::Chat);
        assert!(call.body.get("output_config").is_none(), "{model}");
        assert_eq!(call.body["max_tokens"], MAX_TOKENS);
    }
}

#[test]
fn openai_build() {
    let call = built(ProviderId::Openai, "gpt-5-mini", Wire::Messages);
    assert_eq!(call.url, "https://api.openai.com/v1/chat/completions");
    assert_eq!(
        header(&call, "authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(
        call.body,
        json!({
            "model": "gpt-5-mini",
            "messages": [{"role": "user", "content": PROMPT}],
            "max_completion_tokens": MAX_TOKENS,
            "response_format": {
                "type": "json_schema",
                "json_schema": {"name": "practice_text", "strict": true, "schema": text_schema()}
            }
        })
    );
    assert_eq!(call.shape, shape(ProviderId::Openai, Wire::Chat));
}

#[test]
fn gemini_build() {
    let call = built(
        ProviderId::Gemini,
        "models/gemini-3.5-flash-lite",
        Wire::Chat,
    );
    assert_eq!(
        call.url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent"
    );
    assert_eq!(header(&call, "x-goog-api-key"), Some(KEY));
    assert!(!call.url.contains(KEY));
    assert_eq!(
        call.body,
        json!({
            "contents": [{"role": "user", "parts": [{"text": PROMPT}]}],
            "generationConfig": {
                "maxOutputTokens": MAX_TOKENS,
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {"text": {"type": "STRING"}},
                    "required": ["text"]
                }
            }
        })
    );
    assert_eq!(call.shape, shape(ProviderId::Gemini, Wire::Gemini));
    let bad = build(
        &cfg(ProviderId::Gemini, "../x?key=1"),
        &Endpoints::production(),
        PROMPT,
        Wire::Gemini,
    );
    assert!(matches!(bad, Err(ProviderError::Config { .. })), "{bad:?}");
}

#[test]
fn opencode_chat_build() {
    let call = built(ProviderId::OpencodeGo, "kimi-k2.6", Wire::Chat);
    assert_eq!(
        call.url,
        "https://opencode.ai/inference/go/v1/chat/completions"
    );
    assert_eq!(header(&call, "x-opencode-session"), Some("sess-1"));
    assert_eq!(
        header(&call, "authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(header(&call, "x-api-key"), None);
    assert_eq!(
        call.body,
        json!({
            "model": "kimi-k2.6",
            "messages": [{"role": "user", "content": PROMPT}],
            "max_tokens": MAX_TOKENS
        })
    );
    assert_eq!(call.shape, shape(ProviderId::OpencodeGo, Wire::Chat));
}

#[test]
fn opencode_messages_build() {
    let call = built(ProviderId::OpencodeGo, "minimax-m3", Wire::Messages);
    assert_eq!(call.url, "https://opencode.ai/inference/go/v1/messages");
    assert_eq!(header(&call, "x-opencode-session"), Some("sess-1"));
    assert_eq!(
        header(&call, "authorization"),
        Some(format!("Bearer {KEY}").as_str())
    );
    assert_eq!(header(&call, "x-api-key"), Some(KEY));
    assert_eq!(header(&call, "anthropic-version"), Some("2023-06-01"));
    assert_eq!(
        call.body,
        json!({
            "model": "minimax-m3",
            "max_tokens": MAX_TOKENS,
            "messages": [{"role": "user", "content": PROMPT}]
        })
    );
    assert_eq!(call.shape, shape(ProviderId::OpencodeGo, Wire::Messages));
}

#[test]
fn opencode_responses_build() {
    let call = built(ProviderId::OpencodeGo, "gpt-oss", Wire::Responses);
    assert_eq!(call.url, "https://opencode.ai/inference/go/v1/responses");
    assert_eq!(header(&call, "x-opencode-session"), Some("sess-1"));
    assert_eq!(
        call.body,
        json!({
            "model": "gpt-oss",
            "input": PROMPT,
            "max_output_tokens": MAX_TOKENS,
            "store": false
        })
    );
    assert_eq!(call.shape, shape(ProviderId::OpencodeGo, Wire::Responses));
}

#[test]
fn opencode_session_is_generated_when_missing_and_validated_when_given() {
    let mut c = cfg(ProviderId::OpencodeGo, "m");
    c.session_id = None;
    let call = build(&c, &Endpoints::production(), PROMPT, Wire::Chat).unwrap();
    let id = header(&call, "x-opencode-session").unwrap();
    assert!(valid_session(id), "{id}");
    for bad in ["", "has space", "ñ", &"a".repeat(129)] {
        c.session_id = Some(bad.to_string());
        let got = build(&c, &Endpoints::production(), PROMPT, Wire::Chat);
        assert!(matches!(got, Err(ProviderError::Config { .. })), "{bad:?}");
    }
    let got = build(
        &cfg(ProviderId::OpencodeGo, "m"),
        &Endpoints::production(),
        PROMPT,
        Wire::Gemini,
    );
    assert!(matches!(got, Err(ProviderError::Config { .. })), "{got:?}");
}

#[test]
fn empty_model_is_config_for_every_provider() {
    for id in ProviderId::ALL {
        let got = build(&cfg(id, "  "), &Endpoints::production(), PROMPT, Wire::Chat);
        assert!(matches!(got, Err(ProviderError::Config { .. })), "{id:?}");
    }
}

// ---- parse ----

#[test]
fn claude_parse() {
    let body = json!({
        "content": [
            {"type": "thinking", "thinking": "x"},
            {"type": "text", "text": "{\"text\": "},
            {"type": "text", "text": "\"hola\"}"}
        ],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 120, "output_tokens": 45}
    });
    assert_eq!(
        ok(ProviderId::Claude, Wire::Messages, body).unwrap(),
        ("{\"text\": \"hola\"}".to_string(), usage(120, 45))
    );
    let refused = json!({"content": [], "stop_reason": "refusal"});
    assert!(matches!(
        ok(ProviderId::Claude, Wire::Messages, refused),
        Err(ProviderError::Refused { .. })
    ));
    let cut = json!({"content": [{"type": "text", "text": "{"}], "stop_reason": "max_tokens"});
    assert!(matches!(
        ok(ProviderId::Claude, Wire::Messages, cut),
        Err(ProviderError::BadResponse { .. })
    ));
    assert!(matches!(
        ok(ProviderId::Claude, Wire::Messages, json!({"x": 1})),
        Err(ProviderError::BadResponse { .. })
    ));
}

#[test]
fn chat_parse() {
    let body = json!({
        "choices": [{"message": {"role": "assistant", "content": "{\"text\":\"a\"}"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 80, "completion_tokens": 30, "total_tokens": 110}
    });
    assert_eq!(
        ok(ProviderId::Openai, Wire::Chat, body).unwrap(),
        ("{\"text\":\"a\"}".to_string(), usage(80, 30))
    );
    let no_usage = json!({"choices": [{"message": {"content": "t"}}]});
    assert_eq!(
        ok(ProviderId::OpencodeGo, Wire::Chat, no_usage).unwrap(),
        ("t".to_string(), Usage::default())
    );
    for refused in [
        json!({"choices": [{"message": {"content": null, "refusal": "no"}, "finish_reason": "stop"}]}),
        json!({"choices": [{"message": {"content": ""}, "finish_reason": "content_filter"}]}),
    ] {
        assert!(matches!(
            ok(ProviderId::Openai, Wire::Chat, refused),
            Err(ProviderError::Refused { .. })
        ));
    }
    let cut = json!({"choices": [{"message": {"content": "{"}, "finish_reason": "length"}]});
    assert!(matches!(
        ok(ProviderId::Openai, Wire::Chat, cut),
        Err(ProviderError::BadResponse { .. })
    ));
    assert!(matches!(
        ok(ProviderId::Openai, Wire::Chat, json!({"choices": []})),
        Err(ProviderError::BadResponse { .. })
    ));
}

#[test]
fn responses_parse() {
    let body = json!({
        "status": "completed",
        "output": [
            {"type": "reasoning", "summary": []},
            {"type": "message", "content": [{"type": "output_text", "text": "{\"text\":\"b\"}"}]}
        ],
        "usage": {"input_tokens": 9, "output_tokens": 4}
    });
    assert_eq!(
        ok(ProviderId::OpencodeGo, Wire::Responses, body).unwrap(),
        ("{\"text\":\"b\"}".to_string(), usage(9, 4))
    );
    let refused = json!({"status": "completed", "output": [
        {"type": "message", "content": [{"type": "refusal", "refusal": "no"}]}
    ]});
    assert!(matches!(
        ok(ProviderId::OpencodeGo, Wire::Responses, refused),
        Err(ProviderError::Refused { .. })
    ));
    let filtered =
        json!({"status": "incomplete", "incomplete_details": {"reason": "content_filter"}});
    assert!(matches!(
        ok(ProviderId::OpencodeGo, Wire::Responses, filtered),
        Err(ProviderError::Refused { .. })
    ));
    let cut =
        json!({"status": "incomplete", "incomplete_details": {"reason": "max_output_tokens"}});
    assert!(matches!(
        ok(ProviderId::OpencodeGo, Wire::Responses, cut),
        Err(ProviderError::BadResponse { .. })
    ));
}

#[test]
fn gemini_parse() {
    let body = json!({
        "candidates": [{"content": {"parts": [
            {"text": "thinking", "thought": true},
            {"text": "{\"text\": \"Zapatos "},
            {"text": "azules.\"}"}
        ], "role": "model"}, "finishReason": "STOP"}],
        "usageMetadata": {"promptTokenCount": 70, "candidatesTokenCount": 25, "totalTokenCount": 95}
    });
    assert_eq!(
        ok(ProviderId::Gemini, Wire::Gemini, body).unwrap(),
        ("{\"text\": \"Zapatos azules.\"}".to_string(), usage(70, 25))
    );
    for refused in [
        json!({"promptFeedback": {"blockReason": "SAFETY"}}),
        json!({"candidates": [{"finishReason": "SAFETY"}]}),
        json!({"candidates": [{"finishReason": "PROHIBITED_CONTENT"}]}),
    ] {
        assert!(
            matches!(
                ok(ProviderId::Gemini, Wire::Gemini, refused.clone()),
                Err(ProviderError::Refused { .. })
            ),
            "{refused}"
        );
    }
    let cut = json!({"candidates": [{"content": {"parts": [{"text": "{"}]}, "finishReason": "MAX_TOKENS"}]});
    assert!(matches!(
        ok(ProviderId::Gemini, Wire::Gemini, cut),
        Err(ProviderError::BadResponse { .. })
    ));
}

#[test]
fn non_json_success_body_is_bad_response() {
    let got = parse(shape(ProviderId::Openai, Wire::Chat), 200, "<html>");
    assert!(
        matches!(got, Err(ProviderError::BadResponse { .. })),
        "{got:?}"
    );
}

// ---- status mapping ----

fn status(provider: ProviderId, code: u16, body: &str) -> ProviderError {
    let mapped = status_error(provider, code, body);
    let via_parse = parse(shape(provider, Wire::Chat), code, body).unwrap_err();
    assert_eq!(mapped, via_parse, "parse maps statuses the same way");
    mapped
}

#[test]
fn statuses_map_to_error_kinds() {
    let p = ProviderId::Claude;
    assert!(matches!(status(p, 401, "{}"), ProviderError::Auth { .. }));
    assert!(matches!(status(p, 403, "{}"), ProviderError::Auth { .. }));
    assert!(matches!(
        status(p, 429, "{}"),
        ProviderError::RateLimit { .. }
    ));
    assert!(matches!(
        status(p, 500, "{}"),
        ProviderError::Network { .. }
    ));
    assert!(matches!(
        status(p, 529, "{}"),
        ProviderError::Network { .. }
    ));
    assert!(matches!(status(p, 302, ""), ProviderError::Config { .. }));
    assert!(matches!(
        status(p, 418, "{}"),
        ProviderError::BadResponse { .. }
    ));
    for code in [400, 404, 422] {
        let body =
            r#"{"type":"error","error":{"type":"not_found_error","message":"model: claude-x"}}"#;
        match status(p, code, body) {
            ProviderError::Config { message } => {
                assert!(message.contains("model: claude-x"), "{message}");
                assert!(message.contains(&code.to_string()), "{message}");
            }
            other => panic!("{code}: {other:?}"),
        }
    }
}

#[test]
fn gemini_invalid_key_is_auth() {
    let body = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}"#;
    assert!(matches!(
        status(ProviderId::Gemini, 400, body),
        ProviderError::Auth { .. }
    ));
    assert!(matches!(
        status(ProviderId::Openai, 400, body),
        ProviderError::Config { .. }
    ));
}

#[test]
fn error_detail_strips_controls_and_is_capped() {
    let long =
        json!({"error": {"message": format!("a\u{1b}[31mb{}", "x".repeat(500))}}).to_string();
    let ProviderError::Config { message } = status(ProviderId::Openai, 400, &long) else {
        panic!("expected Config");
    };
    assert!(!message.contains('\u{1b}'), "{message}");
    assert!(message.contains("a[31mb"), "{message}");
    assert!(message.chars().count() < 300, "{}", message.len());
}

#[test]
fn key_echoed_by_the_provider_is_masked_by_the_caller() {
    let body =
        json!({"error": {"message": format!("Incorrect API key provided: {KEY}")}}).to_string();
    let err = status(ProviderId::Openai, 400, &body).masked(KEY);
    let shown = format!("{err} {err:?}");
    assert!(!shown.contains(KEY), "{shown}");
    assert!(shown.contains("***"), "{shown}");
}

// ---- model lists ----

#[test]
fn list_requests_per_provider() {
    let e = Endpoints::production();
    let key = ApiKey::new(KEY).unwrap();
    let (url, headers) = list_request(ProviderId::Claude, &e, Some(&key)).unwrap();
    assert_eq!(url, "https://api.anthropic.com/v1/models?limit=1000");
    assert_eq!(
        headers,
        [
            ("x-api-key", KEY.to_string()),
            ("anthropic-version", "2023-06-01".to_string())
        ]
    );
    let (url, headers) = list_request(ProviderId::Openai, &e, Some(&key)).unwrap();
    assert_eq!(url, "https://api.openai.com/v1/models");
    assert_eq!(headers, [("authorization", format!("Bearer {KEY}"))]);
    let (url, headers) = list_request(ProviderId::Gemini, &e, Some(&key)).unwrap();
    assert_eq!(
        url,
        "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000"
    );
    assert_eq!(headers, [("x-goog-api-key", KEY.to_string())]);
    let (url, headers) = list_request(ProviderId::OpencodeGo, &e, None).unwrap();
    assert_eq!(url, "https://opencode.ai/inference/go/v1/models");
    assert!(headers.is_empty());
    for id in [ProviderId::Claude, ProviderId::Openai, ProviderId::Gemini] {
        assert_eq!(
            list_request(id, &e, None).unwrap_err(),
            ProviderError::NoKey
        );
    }
}

#[test]
fn model_lists_are_parsed_and_filtered() {
    let claude = json!({"data": [{"id": "claude-haiku-5-5", "type": "model"}, {"id": "claude-opus-5-5"}, {"id": "claude-opus-5-5"}]});
    assert_eq!(
        parse_models(ProviderId::Claude, 200, &claude.to_string()).unwrap(),
        ["claude-haiku-5-5", "claude-opus-5-5"]
    );
    let openai = json!({"object": "list", "data": [
        {"id": "gpt-5-mini"}, {"id": "text-embedding-3-small"}, {"id": "gpt-5"},
        {"id": "whisper-1"}, {"id": "gpt-image-1"}, {"id": "tts-1"}, {"id": "bad id"}
    ]});
    assert_eq!(
        parse_models(ProviderId::Openai, 200, &openai.to_string()).unwrap(),
        ["gpt-5", "gpt-5-mini"]
    );
    let gemini = json!({"models": [
        {"name": "models/gemini-3.5-flash-lite", "supportedGenerationMethods": ["generateContent", "countTokens"]},
        {"name": "models/text-embedding-004", "supportedGenerationMethods": ["embedContent"]},
        {"name": "models/gemini-legacy"}
    ]});
    assert_eq!(
        parse_models(ProviderId::Gemini, 200, &gemini.to_string()).unwrap(),
        ["gemini-3.5-flash-lite"]
    );
    let go = json!({"object": "list", "data": [{"id": "kimi-k2.6"}, {"id": "glm-5"}]});
    assert_eq!(
        parse_models(ProviderId::OpencodeGo, 200, &go.to_string()).unwrap(),
        ["kimi-k2.6", "glm-5"]
    );
    assert!(matches!(
        parse_models(ProviderId::OpencodeGo, 200, "{}"),
        Err(ProviderError::BadResponse { .. })
    ));
    assert!(matches!(
        parse_models(ProviderId::Openai, 401, "{}"),
        Err(ProviderError::Auth { .. })
    ));
}
