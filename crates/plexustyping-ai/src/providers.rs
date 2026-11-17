//! Request shapes and reply parsing for each provider: pure functions, no I/O.

use serde_json::{Value, json};

use crate::{ApiKey, Endpoints, ProviderConfig, ProviderError, ProviderId, Usage};

/// The wire format of a request and its reply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Wire {
    /// Anthropic Messages (`/messages`).
    Messages,
    /// Gemini `generateContent`.
    Gemini,
    /// OpenAI Chat Completions (`/chat/completions`).
    #[default]
    Chat,
    /// OpenAI Responses (`/responses`).
    Responses,
}

/// How to read a reply: the provider (for messages) and its wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shape {
    pub provider: ProviderId,
    pub wire: Wire,
}

/// Request headers (name, value).
pub(crate) type Headers = Vec<(&'static str, String)>;

/// One HTTP POST ready to send.
#[derive(Debug)]
pub(crate) struct Call {
    pub url: String,
    pub headers: Headers,
    pub body: Value,
    pub shape: Shape,
}

/// Output token budget of every generation (thinking counts toward it).
pub(crate) const MAX_TOKENS: u32 = 4096;
/// Anthropic Messages API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// OpenCode's per-conversation header.
const SESSION_HEADER: &str = "x-opencode-session";
/// Longest provider error message kept in a [`ProviderError`].
const MAX_DETAIL: usize = 200;
/// Longest model id accepted from a model list.
const MAX_ID: usize = 200;
/// Gemini finish/block reasons that mean the model declined.
const GEMINI_BLOCKED: [&str; 6] = [
    "SAFETY",
    "RECITATION",
    "BLOCKLIST",
    "PROHIBITED_CONTENT",
    "SPII",
    "IMAGE_SAFETY",
];
/// OpenAI model families that do not answer chat requests.
const OPENAI_NOT_CHAT: [&str; 10] = [
    "embedding",
    "tts",
    "whisper",
    "dall-e",
    "moderation",
    "transcribe",
    "realtime",
    "audio",
    "image",
    "davinci",
];

/// A session id of 1–128 characters of `A-Z a-z 0-9 . _ -`.
pub(crate) fn valid_session(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// A fresh valid session id (`plexustyping-<128 random bits in hex>`), revealing nothing
/// about the user or when the app started.
pub(crate) fn new_session_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    // Each `RandomState` is seeded from OS randomness; two of them give 128 bits.
    let random = || {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(0);
        hasher.finish()
    };
    format!("plexustyping-{:016x}{:016x}", random(), random())
}

/// `{text: string}`, the only answer accepted (Anthropic and OpenAI strict mode).
fn text_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"text": {"type": "string"}},
        "required": ["text"],
        "additionalProperties": false
    })
}

/// Older Claude models reject `output_config` (structured outputs).
fn claude_supports_output_config(model: &str) -> bool {
    const LEGACY: [&str; 7] = [
        "claude-3",
        "claude-haiku-4",
        "claude-sonnet-4-0",
        "claude-sonnet-4-2",
        "claude-sonnet-4-5",
        "claude-opus-4-0",
        "claude-opus-4-1",
    ];
    !LEGACY.iter().any(|prefix| model.starts_with(prefix))
}

/// Gemini puts the model in the URL path, so only plain model codes pass.
fn gemini_model(model: &str) -> Result<&str, ProviderError> {
    let model = model.strip_prefix("models/").unwrap_or(model);
    let valid = model
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if valid && !model.starts_with('.') {
        Ok(model)
    } else {
        Err(ProviderError::config(format!(
            "\"{model}\" is not a valid Gemini model name."
        )))
    }
}

fn user_message(prompt: &str) -> Value {
    json!([{"role": "user", "content": prompt}])
}

fn bearer(key: &ApiKey) -> (&'static str, String) {
    ("authorization", format!("Bearer {}", key.expose()))
}

/// Builds the generation request. `format` and `low_effort` (the reasoning
/// effort to ask for) only matter to OpenCode Go (its per-model API); the other
/// providers always use their own API and body.
pub(crate) fn build(
    cfg: &ProviderConfig,
    endpoints: &Endpoints,
    prompt: &str,
    format: Wire,
    low_effort: Option<&str>,
) -> Result<Call, ProviderError> {
    let model = cfg.model.trim();
    if model.is_empty() {
        return Err(ProviderError::config(format!(
            "Choose a {} model first.",
            cfg.id.label()
        )));
    }
    let key = &cfg.api_key;
    let provider = cfg.id;
    let call = match provider {
        ProviderId::Claude => {
            let mut body = json!({
                "model": model,
                "max_tokens": MAX_TOKENS,
                "messages": user_message(prompt),
            });
            if claude_supports_output_config(model) {
                body["output_config"] =
                    json!({"format": {"type": "json_schema", "schema": text_schema()}});
            }
            Call {
                url: format!("{}/messages", endpoints.anthropic),
                headers: vec![
                    ("x-api-key", key.expose().to_string()),
                    ("anthropic-version", ANTHROPIC_VERSION.to_string()),
                ],
                body,
                shape: Shape {
                    provider,
                    wire: Wire::Messages,
                },
            }
        }
        ProviderId::Openai => Call {
            url: format!("{}/chat/completions", endpoints.openai),
            headers: vec![bearer(key)],
            body: json!({
                "model": model,
                "messages": user_message(prompt),
                // OpenAI deprecated `max_tokens` (reasoning models reject it).
                "max_completion_tokens": MAX_TOKENS,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {"name": "practice_text", "strict": true, "schema": text_schema()}
                }
            }),
            shape: Shape {
                provider,
                wire: Wire::Chat,
            },
        },
        ProviderId::Gemini => {
            let model = gemini_model(model)?;
            Call {
                url: format!("{}/models/{model}:generateContent", endpoints.gemini),
                headers: vec![("x-goog-api-key", key.expose().to_string())],
                body: json!({
                    "contents": [{"role": "user", "parts": [{"text": prompt}]}],
                    "generationConfig": {
                        "maxOutputTokens": MAX_TOKENS,
                        "responseMimeType": "application/json",
                        "responseSchema": {
                            "type": "OBJECT",
                            "properties": {"text": {"type": "STRING"}},
                            "required": ["text"]
                        }
                    }
                }),
                shape: Shape {
                    provider,
                    wire: Wire::Gemini,
                },
            }
        }
        ProviderId::OpencodeGo => opencode_go(cfg, endpoints, prompt, format, low_effort)?,
    };
    Ok(call)
}

/// OpenCode Go: Bearer key and session header, body in the model's own format.
/// Only portable fields are sent (no structured-output schema). `low_effort`
/// goes in the effort field of chat (`reasoning_effort`) and responses
/// (`reasoning.effort`); Anthropic Messages has no effort field matching the
/// catalogue's values (thinking is set with a token budget), so it gets none.
fn opencode_go(
    cfg: &ProviderConfig,
    endpoints: &Endpoints,
    prompt: &str,
    format: Wire,
    low_effort: Option<&str>,
) -> Result<Call, ProviderError> {
    let model = cfg.model.trim();
    let session = match &cfg.session_id {
        Some(id) if valid_session(id) => id.clone(),
        Some(_) => {
            return Err(ProviderError::config(
                "OpenCode Go needs a session id of 1-128 letters, digits, '.', '_' or '-'.",
            ));
        }
        None => new_session_id(),
    };
    let base = &endpoints.opencode_go;
    let mut headers = vec![(SESSION_HEADER, session), bearer(&cfg.api_key)];
    let (url, body) = match format {
        Wire::Chat => {
            let mut body =
                json!({"model": model, "messages": user_message(prompt), "max_tokens": MAX_TOKENS});
            if let Some(effort) = low_effort {
                body["reasoning_effort"] = json!(effort);
            }
            (format!("{base}/chat/completions"), body)
        }
        Wire::Messages => {
            // OpenCode's own Anthropic client also sends the key as `x-api-key`.
            headers.push(("x-api-key", cfg.api_key.expose().to_string()));
            headers.push(("anthropic-version", ANTHROPIC_VERSION.to_string()));
            (
                format!("{base}/messages"),
                json!({"model": model, "max_tokens": MAX_TOKENS, "messages": user_message(prompt)}),
            )
        }
        Wire::Responses => {
            let mut body = json!({"model": model, "input": prompt, "max_output_tokens": MAX_TOKENS, "store": false});
            if let Some(effort) = low_effort {
                body["reasoning"] = json!({"effort": effort});
            }
            (format!("{base}/responses"), body)
        }
        Wire::Gemini => {
            return Err(ProviderError::config(format!(
                "The API of the OpenCode Go model \"{model}\" is not supported."
            )));
        }
    };
    Ok(Call {
        url,
        headers,
        body,
        shape: Shape {
            provider: ProviderId::OpencodeGo,
            wire: format,
        },
    })
}

fn refused(provider: ProviderId) -> ProviderError {
    ProviderError::Refused {
        message: format!("{} declined to write this text.", provider.label()),
    }
}

fn truncated(provider: ProviderId) -> ProviderError {
    ProviderError::bad_response(format!("The answer from {} was cut off.", provider.label()))
}

fn unreadable(provider: ProviderId) -> ProviderError {
    ProviderError::bad_response(format!(
        "{} sent an answer PlexusTyping could not read.",
        provider.label()
    ))
}

fn tokens(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}

/// Reads a reply: the model's raw text (still JSON) and the token usage.
pub(crate) fn parse(
    shape: Shape,
    status: u16,
    body: &str,
) -> Result<(String, Usage), ProviderError> {
    if !(200..300).contains(&status) {
        return Err(status_error(shape.provider, status, body));
    }
    let value: Value = serde_json::from_str(body).map_err(|_| unreadable(shape.provider))?;
    let reply = match shape.wire {
        Wire::Messages => messages_reply(&value),
        Wire::Gemini => gemini_reply(&value),
        Wire::Chat => chat_reply(&value),
        Wire::Responses => responses_reply(&value),
    };
    match reply {
        Some(Reply::Text(text, usage)) => Ok((text, usage)),
        Some(Reply::Refused) => Err(refused(shape.provider)),
        Some(Reply::Truncated) => Err(truncated(shape.provider)),
        None => Err(unreadable(shape.provider)),
    }
}

enum Reply {
    Text(String, Usage),
    Refused,
    Truncated,
}

fn messages_reply(body: &Value) -> Option<Reply> {
    match body["stop_reason"].as_str() {
        Some("refusal") => return Some(Reply::Refused),
        Some("max_tokens") => return Some(Reply::Truncated),
        _ => {}
    }
    let text: String = body["content"]
        .as_array()?
        .iter()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();
    let usage = Usage {
        input_tokens: tokens(&body["usage"]["input_tokens"]),
        output_tokens: tokens(&body["usage"]["output_tokens"]),
    };
    Some(Reply::Text(text, usage))
}

fn gemini_reply(body: &Value) -> Option<Reply> {
    let Some(candidate) = body["candidates"].as_array().and_then(|c| c.first()) else {
        return body["promptFeedback"]["blockReason"]
            .is_string()
            .then_some(Reply::Refused);
    };
    let finish = candidate["finishReason"].as_str().unwrap_or_default();
    if GEMINI_BLOCKED.contains(&finish) {
        return Some(Reply::Refused);
    }
    if finish == "MAX_TOKENS" {
        return Some(Reply::Truncated);
    }
    let text: String = candidate["content"]["parts"]
        .as_array()?
        .iter()
        .filter(|part| part["thought"] != true)
        .filter_map(|part| part["text"].as_str())
        .collect();
    let meta = &body["usageMetadata"];
    let usage = Usage {
        input_tokens: tokens(&meta["promptTokenCount"]),
        output_tokens: tokens(&meta["candidatesTokenCount"]),
    };
    Some(Reply::Text(text, usage))
}

fn chat_reply(body: &Value) -> Option<Reply> {
    let choice = body["choices"].as_array()?.first()?;
    let message = &choice["message"];
    if message["refusal"].as_str().is_some_and(|r| !r.is_empty()) {
        return Some(Reply::Refused);
    }
    match choice["finish_reason"].as_str() {
        Some("content_filter") => return Some(Reply::Refused),
        Some("length") => return Some(Reply::Truncated),
        _ => {}
    }
    let text = match &message["content"] {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect(),
        _ => return None,
    };
    let usage = Usage {
        input_tokens: tokens(&body["usage"]["prompt_tokens"]),
        output_tokens: tokens(&body["usage"]["completion_tokens"]),
    };
    Some(Reply::Text(text, usage))
}

fn responses_reply(body: &Value) -> Option<Reply> {
    if body["status"] == "incomplete" {
        return Some(match body["incomplete_details"]["reason"].as_str() {
            Some("content_filter") => Reply::Refused,
            _ => Reply::Truncated,
        });
    }
    let mut text = String::new();
    for item in body["output"]
        .as_array()?
        .iter()
        .filter(|item| item["type"] == "message")
    {
        for part in item["content"].as_array()? {
            match part["type"].as_str() {
                Some("refusal") => return Some(Reply::Refused),
                Some("output_text") => text.push_str(part["text"].as_str().unwrap_or_default()),
                _ => {}
            }
        }
    }
    let usage = Usage {
        input_tokens: tokens(&body["usage"]["input_tokens"]),
        output_tokens: tokens(&body["usage"]["output_tokens"]),
    };
    Some(Reply::Text(text, usage))
}

/// The provider's own `error.message`, without control characters, capped.
fn error_detail(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let message: String = value["error"]["message"]
        .as_str()?
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let message = message.trim();
    if message.is_empty() {
        return None;
    }
    if message.chars().count() > MAX_DETAIL {
        let cut: String = message.chars().take(MAX_DETAIL).collect();
        return Some(format!("{cut}…"));
    }
    Some(message.to_string())
}

/// Maps a non-2xx status. The caller masks the key in the result.
pub(crate) fn status_error(provider: ProviderId, status: u16, body: &str) -> ProviderError {
    let name = provider.label();
    let gemini_bad_key = provider == ProviderId::Gemini
        && status == 400
        && (body.contains("API_KEY_INVALID") || body.contains("API key not valid"));
    match status {
        401 | 403 => ProviderError::Auth {
            message: format!("The {name} API key was rejected."),
        },
        _ if gemini_bad_key => ProviderError::Auth {
            message: format!("The {name} API key was rejected."),
        },
        300..=399 => ProviderError::config(format!(
            "{name} redirected the request (HTTP {status}); redirects are not followed."
        )),
        429 => ProviderError::RateLimit {
            message: format!("{name} is limiting requests; try again in a moment."),
        },
        500..=599 => ProviderError::Network {
            message: format!("{name} is unavailable right now (HTTP {status})."),
        },
        400 | 404 | 422 => ProviderError::config(match error_detail(body) {
            Some(detail) => format!("{name} rejected the request (HTTP {status}): {detail}"),
            None => format!("{name} rejected the request (HTTP {status}). Check the model name."),
        }),
        _ => ProviderError::bad_response(format!("{name} answered with HTTP {status}.")),
    }
}

/// URL and headers of the model list; `None` key is only valid for OpenCode Go,
/// whose list is public and is fetched without the key.
pub(crate) fn list_request(
    id: ProviderId,
    endpoints: &Endpoints,
    key: Option<&ApiKey>,
) -> Result<(String, Headers), ProviderError> {
    if id == ProviderId::OpencodeGo {
        return Ok((format!("{}/models", endpoints.opencode_go), Vec::new()));
    }
    let key = key.ok_or(ProviderError::NoKey)?;
    Ok(match id {
        ProviderId::Claude => (
            format!("{}/models?limit=1000", endpoints.anthropic),
            vec![
                ("x-api-key", key.expose().to_string()),
                ("anthropic-version", ANTHROPIC_VERSION.to_string()),
            ],
        ),
        ProviderId::Openai => (format!("{}/models", endpoints.openai), vec![bearer(key)]),
        ProviderId::Gemini => (
            format!("{}/models?pageSize=1000", endpoints.gemini),
            vec![("x-goog-api-key", key.expose().to_string())],
        ),
        ProviderId::OpencodeGo => unreachable!("handled above"),
    })
}

/// Model ids end up in request bodies and the UI: short, no whitespace or controls.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().count() <= MAX_ID
        && !id.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Model ids from a model-list reply, filtered to text-generation models and
/// deduplicated (OpenAI's list is sorted; the others keep the provider order).
pub(crate) fn parse_models(
    id: ProviderId,
    status: u16,
    body: &str,
) -> Result<Vec<String>, ProviderError> {
    if !(200..300).contains(&status) {
        return Err(status_error(id, status, body));
    }
    let value: Value = serde_json::from_str(body).map_err(|_| unreadable(id))?;
    let ids: Vec<String> = if id == ProviderId::Gemini {
        value["models"]
            .as_array()
            .ok_or_else(|| unreadable(id))?
            .iter()
            .filter(|m| {
                m["supportedGenerationMethods"]
                    .as_array()
                    .is_some_and(|g| g.iter().any(|g| g == "generateContent"))
            })
            .filter_map(|m| m["name"].as_str())
            .map(|name| name.strip_prefix("models/").unwrap_or(name).to_string())
            .collect()
    } else {
        value["data"]
            .as_array()
            .ok_or_else(|| unreadable(id))?
            .iter()
            .filter_map(|m| m["id"].as_str())
            .filter(|m| id != ProviderId::Openai || !OPENAI_NOT_CHAT.iter().any(|f| m.contains(f)))
            .map(str::to_string)
            .collect()
    };
    let mut seen = std::collections::HashSet::new();
    let mut ids: Vec<String> = ids
        .into_iter()
        .filter(|m| valid_id(m) && seen.insert(m.clone()))
        .collect();
    if id == ProviderId::Openai {
        ids.sort();
    }
    Ok(ids)
}

#[cfg(test)]
mod tests;
