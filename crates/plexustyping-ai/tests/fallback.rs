//! `generate_with_fallback` order and retry rules (S3).

mod common;

use common::{Reply, chat_reply, config, mock, small_request, text_json};
use plexustyping_ai::{ProviderError, ProviderId, ValidationIssue};
use serde_json::json;
use std::time::{Duration, Instant};

const OPENAI: &str = "/v1/chat/completions";
const GEMINI: &str = "/v1beta/models/gemini-3.5-flash-lite:generateContent";

fn gemini_reply(text: &str) -> Reply {
    Reply::ok(json!({
        "candidates": [{"content": {"parts": [{"text": text_json(text)}]}, "finishReason": "STOP"}],
        "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
    }))
}

fn chain() -> Vec<plexustyping_ai::ProviderConfig> {
    vec![
        config(ProviderId::Openai, "gpt-5-mini"),
        config(ProviderId::Gemini, "gemini-3.5-flash-lite"),
    ]
}

fn invalid() -> Reply {
    chat_reply(&text_json("sin la letra"))
}

fn low_density() -> ProviderError {
    ProviderError::Invalid {
        issues: vec![ValidationIssue::LowDensity {
            target: "z".into(),
            found: 0,
            needed: 1,
        }],
    }
}

#[tokio::test]
async fn first_valid_provider_wins() {
    let server = mock(vec![
        (OPENAI, chat_reply(&text_json("Zapatos azules"))),
        (GEMINI, gemini_reply("Zorro")),
    ])
    .await;

    let got = server
        .client()
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap();

    assert_eq!(got.provider, ProviderId::Openai);
    assert_eq!(server.hits(GEMINI), 0);
    assert!(got.failed_attempts.is_empty());
}

#[tokio::test]
async fn invalid_text_is_retried_once_then_next_provider() {
    let server = mock(vec![
        (OPENAI, invalid()),
        (OPENAI, invalid()),
        (OPENAI, chat_reply(&text_json("Zapatos azules"))),
        (GEMINI, gemini_reply("Zorro veloz")),
    ])
    .await;

    let got = server
        .client()
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap();

    assert_eq!(got.provider, ProviderId::Gemini);
    assert_eq!(got.text, "Zorro veloz");
    assert_eq!(server.hits(OPENAI), 2, "one retry only");
    // The rejected attempts that came before are kept, so the app can log why.
    assert_eq!(
        got.failed_attempts,
        vec![
            (ProviderId::Openai, low_density()),
            (ProviderId::Openai, low_density())
        ]
    );
}

#[tokio::test]
async fn bad_response_retry_can_succeed() {
    let server = mock(vec![
        (OPENAI, chat_reply("not json")),
        (OPENAI, chat_reply(&text_json("Zapatos azules"))),
    ])
    .await;

    let got = server
        .client()
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap();

    assert_eq!(got.provider, ProviderId::Openai);
    assert_eq!(got.text, "Zapatos azules");
    assert_eq!(server.hits(OPENAI), 2);
}

#[tokio::test]
async fn other_errors_move_on_without_retry() {
    let server = mock(vec![
        (OPENAI, Reply::status(401, "{}")),
        (OPENAI, chat_reply(&text_json("Zapatos azules"))),
        (GEMINI, gemini_reply("Zorro")),
    ])
    .await;

    let got = server
        .client()
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap();

    assert_eq!(got.provider, ProviderId::Gemini);
    assert_eq!(server.hits(OPENAI), 1);
}

#[tokio::test]
async fn every_error_is_returned_in_order_when_all_fail() {
    let server = mock(vec![
        (OPENAI, invalid()),
        (OPENAI, invalid()),
        (GEMINI, Reply::status(429, "{}")),
    ])
    .await;

    let errors = server
        .client()
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap_err();

    assert_eq!(errors.len(), 3, "{errors:?}");
    assert_eq!(errors[0], (ProviderId::Openai, low_density()));
    assert_eq!(errors[1], (ProviderId::Openai, low_density()));
    assert_eq!(errors[2].0, ProviderId::Gemini);
    assert!(matches!(errors[2].1, ProviderError::RateLimit { .. }));
}

#[tokio::test]
async fn empty_chain_fails_without_requests() {
    let server = mock(vec![]).await;

    let errors = server
        .client()
        .generate_with_fallback(&[], &small_request())
        .await
        .unwrap_err();

    assert!(errors.is_empty());
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn the_deadline_skips_the_remaining_providers_with_timeout() {
    let server = mock(vec![
        (
            OPENAI,
            chat_reply(&text_json("Zapatos azules")).delayed(Duration::from_secs(3)),
        ),
        (GEMINI, gemini_reply("Zorro")),
    ])
    .await;
    let deadline = Duration::from_millis(300);
    let client = server.client().with_deadline(deadline);

    let started = Instant::now();
    let errors = client
        .generate_with_fallback(&chain(), &small_request())
        .await
        .unwrap_err();

    assert!(
        started.elapsed() < deadline + Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(
        errors,
        vec![
            (ProviderId::Openai, ProviderError::Timeout),
            (ProviderId::Gemini, ProviderError::Timeout),
        ]
    );
    assert_eq!(server.hits(OPENAI), 1);
    assert_eq!(server.hits(GEMINI), 0, "later providers are not contacted");
}
