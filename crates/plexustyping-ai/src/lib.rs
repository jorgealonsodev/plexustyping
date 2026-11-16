//! AI text generation for PlexusTyping practice texts.
//!
//! [`AiClient`] talks to four providers (Claude, OpenAI, Gemini and OpenCode Go)
//! through one HTTP sender: each provider is a pure request builder plus a pure
//! reply parser. On top of it sit the Spanish generation prompt
//! ([`build_prompt`]), tolerant parsing of the `{"text": "..."}` reply
//! ([`parse_text_json`]), validation of the generated text ([`validate`]) and a
//! provider fallback chain ([`AiClient::generate_with_fallback`]).
//!
//! Privacy: a [`GenerationRequest`] only carries target characters, language,
//! mode, length, allowed characters and topic — never what the user typed. API
//! keys never appear in `Debug` output or error messages, redirects are never
//! followed, and [`Endpoints::production`] is the only set of hosts used.

use std::fmt;
use std::ops::Range;
use std::time::Duration;

use serde::{Deserialize, Serialize};

mod client;
mod opencode;
mod parse;
mod prompt;
mod providers;
mod validate;

pub use client::{AiClient, TextProvider};
pub use parse::parse_text_json;
pub use prompt::build_prompt;
pub use validate::{BASELINE_PER_CHAR, ValidationIssue, validate};

/// Default per-request timeout (PRD RF2: 10 s).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Supported text providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    /// Anthropic Messages API.
    Claude,
    /// OpenAI Chat Completions API.
    Openai,
    /// Google Gemini `generateContent` API.
    Gemini,
    /// OpenCode Go subscription (open models over several API formats).
    OpencodeGo,
}

impl ProviderId {
    /// Every provider, in display order.
    pub const ALL: [ProviderId; 4] = [
        ProviderId::Claude,
        ProviderId::Openai,
        ProviderId::Gemini,
        ProviderId::OpencodeGo,
    ];

    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            ProviderId::Claude => "Claude (Anthropic)",
            ProviderId::Openai => "OpenAI",
            ProviderId::Gemini => "Google Gemini",
            ProviderId::OpencodeGo => "OpenCode Go",
        }
    }

    /// Suggested default model: a default only, the user can pick any model
    /// from [`AiClient::list_models`]. OpenCode Go has none (`""`): the user
    /// picks one from the list.
    pub fn default_model(self) -> &'static str {
        match self {
            ProviderId::Claude => "claude-haiku-5-5",
            ProviderId::Openai => "gpt-5-mini",
            ProviderId::Gemini => "gemini-3.5-flash-lite",
            ProviderId::OpencodeGo => "",
        }
    }
}

/// What kind of text to generate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Mode {
    /// Natural-language prose.
    Prose,
    /// A code snippet in `lang` (refined in phase 2).
    Code {
        /// Programming language name, e.g. `"rust"`.
        lang: String,
    },
}

/// Parameters of one generation. Carries no user-typed text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationRequest {
    /// Language code, e.g. `"es"` or `"en"`.
    pub language: String,
    /// Prose or code.
    pub mode: Mode,
    /// Characters or n-grams to practise, e.g. `["ñ", "ci"]`.
    pub targets: Vec<String>,
    /// Multiplier over the normal frequency, clamped to `2.0..=5.0`.
    pub density: f32,
    /// Accepted length in characters (end exclusive, like any `Range`).
    pub length: Range<usize>,
    /// Characters the layout can type; space is always allowed.
    pub allowed_chars: String,
    /// Optional topic; `None` means free topic.
    pub topic: Option<String>,
}

/// Token usage reported by the provider (zero when it reports none).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// Prompt tokens.
    pub input_tokens: u64,
    /// Completion tokens.
    pub output_tokens: u64,
}

/// A validated generated text with its origin and usage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    /// The normalized text returned by [`validate`].
    pub text: String,
    /// Provider that produced it.
    pub provider: ProviderId,
    /// Model that produced it.
    pub model: String,
    /// Tokens consumed by this call.
    pub usage: Usage,
}

/// Provider failure, serialized as `{ "kind": ..., ... }`. Messages never
/// contain the API key.
#[derive(Clone, Debug, PartialEq, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ProviderError {
    /// No API key configured.
    #[error("no API key is configured")]
    NoKey,
    /// The key was rejected.
    #[error("{message}")]
    Auth {
        /// User-facing explanation.
        message: String,
    },
    /// The provider is limiting requests (HTTP 429).
    #[error("{message}")]
    RateLimit {
        /// User-facing explanation.
        message: String,
    },
    /// Connection failure or provider outage (HTTP 5xx).
    #[error("{message}")]
    Network {
        /// User-facing explanation.
        message: String,
    },
    /// The request exceeded the configured timeout.
    #[error("the provider did not answer in time")]
    Timeout,
    /// The model declined to answer.
    #[error("{message}")]
    Refused {
        /// User-facing explanation.
        message: String,
    },
    /// The reply was malformed or cut off.
    #[error("{message}")]
    BadResponse {
        /// User-facing explanation.
        message: String,
    },
    /// Wrong model, rejected request, redirect or invalid settings.
    #[error("{message}")]
    Config {
        /// User-facing explanation.
        message: String,
    },
    /// The generated text failed [`validate`].
    #[error("the generated text failed validation ({} issues)", issues.len())]
    Invalid {
        /// Every problem found.
        issues: Vec<ValidationIssue>,
    },
}

impl ProviderError {
    /// Replaces every occurrence of `key` in the message with `***`.
    pub(crate) fn masked(self, key: &str) -> Self {
        if key.is_empty() {
            return self;
        }
        let mask = |message: String| message.replace(key, "***");
        match self {
            ProviderError::Auth { message } => ProviderError::Auth {
                message: mask(message),
            },
            ProviderError::RateLimit { message } => ProviderError::RateLimit {
                message: mask(message),
            },
            ProviderError::Network { message } => ProviderError::Network {
                message: mask(message),
            },
            ProviderError::Refused { message } => ProviderError::Refused {
                message: mask(message),
            },
            ProviderError::BadResponse { message } => ProviderError::BadResponse {
                message: mask(message),
            },
            ProviderError::Config { message } => ProviderError::Config {
                message: mask(message),
            },
            other => other,
        }
    }

    pub(crate) fn config(message: impl Into<String>) -> Self {
        ProviderError::Config {
            message: message.into(),
        }
    }

    pub(crate) fn bad_response(message: impl Into<String>) -> Self {
        ProviderError::BadResponse {
            message: message.into(),
        }
    }
}

/// An API key; its bytes never show up in `Debug` output.
#[derive(Clone)]
pub struct ApiKey(String);

impl ApiKey {
    /// Trims `key`; an empty key is [`ProviderError::NoKey`].
    pub fn new(key: impl Into<String>) -> Result<Self, ProviderError> {
        let key = key.into();
        let key = key.trim();
        if key.is_empty() {
            Err(ProviderError::NoKey)
        } else {
            Ok(ApiKey(key.to_string()))
        }
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

/// Connection settings for one provider.
#[derive(Clone, Debug)]
pub struct ProviderConfig {
    /// Provider.
    pub id: ProviderId,
    /// API key (from the system keychain).
    pub api_key: ApiKey,
    /// Model id; must not be empty when generating.
    pub model: String,
    /// Per-request timeout ([`DEFAULT_TIMEOUT`] by default).
    pub timeout: Duration,
    /// OpenCode Go `x-opencode-session` value (1–128 of `A-Z a-z 0-9 . _ -`);
    /// `None` uses one id generated per [`AiClient`].
    pub session_id: Option<String>,
}

impl ProviderConfig {
    /// Config with the provider's default model and the default timeout.
    pub fn new(id: ProviderId, api_key: ApiKey) -> Self {
        ProviderConfig {
            id,
            api_key,
            model: id.default_model().to_string(),
            timeout: DEFAULT_TIMEOUT,
            session_id: None,
        }
    }
}

/// Endpoint roots. [`Endpoints::production`] is the only allowlist (PRD §6:
/// no network access except the four providers, plus the models.dev catalogue
/// that says which API format each OpenCode Go model uses).
#[derive(Clone, Debug)]
pub struct Endpoints {
    /// Anthropic API root (`/messages`, `/models`).
    pub(crate) anthropic: String,
    /// Gemini API root (`/models/{model}:generateContent`, `/models`).
    pub(crate) gemini: String,
    /// OpenAI API root (`/chat/completions`, `/models`).
    pub(crate) openai: String,
    /// OpenCode Go root (`/chat/completions`, `/messages`, `/responses`, `/models`).
    pub(crate) opencode_go: String,
    /// Full models.dev catalogue URL.
    pub(crate) models_dev: String,
}

impl Endpoints {
    /// The real provider hosts.
    pub fn production() -> Self {
        Endpoints {
            anthropic: "https://api.anthropic.com/v1".into(),
            gemini: "https://generativelanguage.googleapis.com/v1beta".into(),
            openai: "https://api.openai.com/v1".into(),
            opencode_go: "https://opencode.ai/inference/go/v1".into(),
            models_dev: "https://models.dev/api.json".into(),
        }
    }

    /// Test-only: every root under `base` (e.g. `http://127.0.0.1:PORT`), with
    /// the production paths (`/v1`, `/v1beta`, `/inference/go/v1`, `/api.json`).
    #[doc(hidden)]
    pub fn loopback(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Endpoints {
            anthropic: format!("{base}/v1"),
            gemini: format!("{base}/v1beta"),
            openai: format!("{base}/v1"),
            opencode_go: format!("{base}/inference/go/v1"),
            models_dev: format!("{base}/api.json"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(url: &str) -> &str {
        let rest = url.strip_prefix("https://").expect("https only");
        rest.split('/').next().unwrap()
    }

    #[test]
    fn production_hosts_are_exactly_the_four_providers_and_models_dev() {
        let e = Endpoints::production();
        let mut hosts: Vec<&str> = [
            &e.anthropic,
            &e.gemini,
            &e.openai,
            &e.opencode_go,
            &e.models_dev,
        ]
        .iter()
        .map(|u| host(u))
        .collect();
        hosts.sort_unstable();
        assert_eq!(
            hosts,
            [
                "api.anthropic.com",
                "api.openai.com",
                "generativelanguage.googleapis.com",
                "models.dev",
                "opencode.ai",
            ]
        );
        assert_eq!(e.opencode_go, "https://opencode.ai/inference/go/v1");
    }

    #[test]
    fn api_key_trims_rejects_empty_and_redacts_debug() {
        assert_eq!(ApiKey::new("  \n").unwrap_err(), ProviderError::NoKey);
        let key = ApiKey::new("  sk-SECRET-1 ").unwrap();
        assert_eq!(key.expose(), "sk-SECRET-1");
        assert_eq!(format!("{key:?}"), "ApiKey(***)");
        let cfg = ProviderConfig::new(ProviderId::Claude, key);
        let shown = format!("{cfg:?} {cfg:#?}");
        assert!(!shown.contains("SECRET"), "{shown}");
        assert!(shown.contains("ApiKey(***)"));
        assert_eq!(cfg.timeout, Duration::from_secs(10));
        assert_eq!(cfg.model, "claude-haiku-5-5");
    }

    #[test]
    fn provider_metadata_and_serde_shapes() {
        assert_eq!(ProviderId::Openai.default_model(), "gpt-5-mini");
        assert_eq!(ProviderId::Gemini.default_model(), "gemini-3.5-flash-lite");
        assert_eq!(ProviderId::OpencodeGo.default_model(), "");
        assert_eq!(ProviderId::OpencodeGo.label(), "OpenCode Go");
        assert_eq!(
            serde_json::to_string(&ProviderId::OpencodeGo).unwrap(),
            "\"opencode-go\""
        );
        assert_eq!(
            serde_json::to_value(Mode::Code {
                lang: "rust".into()
            })
            .unwrap(),
            serde_json::json!({"kind": "code", "lang": "rust"})
        );
        assert_eq!(
            serde_json::to_value(ProviderError::Auth {
                message: "m".into()
            })
            .unwrap(),
            serde_json::json!({"kind": "auth", "message": "m"})
        );
        assert_eq!(
            serde_json::to_value(ProviderError::Timeout).unwrap(),
            serde_json::json!({"kind": "timeout"})
        );
    }

    #[test]
    fn masked_replaces_the_key_in_messages() {
        let err = ProviderError::config("bad key sk-1 here sk-1").masked("sk-1");
        assert_eq!(err, ProviderError::config("bad key *** here ***"));
        assert_eq!(ProviderError::Timeout.masked("k"), ProviderError::Timeout);
    }
}
