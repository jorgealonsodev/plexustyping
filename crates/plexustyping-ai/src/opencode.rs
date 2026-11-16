//! OpenCode Go: which API format each model is served with.
//!
//! The format comes from models.dev (`["opencode-go"].models[id].provider.npm`;
//! a model without one uses the provider default, OpenAI-compatible chat). The
//! catalogue is fetched on the first OpenCode Go generation and kept in memory
//! for 24 hours.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::providers::Wire;

/// How long a fetched catalogue is used.
pub(crate) const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed fetch, chat completions are assumed for this long.
pub(crate) const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(5 * 60);
/// models.dev is a few megabytes; anything far larger is refused.
pub(crate) const MAX_CATALOGUE_BYTES: usize = 32 * 1024 * 1024;

/// Maps the AI SDK package models.dev names to the wire format.
pub(crate) fn wire_from_npm(npm: Option<&str>) -> Wire {
    match npm {
        Some("@ai-sdk/anthropic") => Wire::Messages,
        Some("@ai-sdk/openai") => Wire::Responses,
        _ => Wire::Chat,
    }
}

/// Model id → format, from a models.dev body; `None` when unreadable.
pub(crate) fn formats_from_models_dev(body: &[u8]) -> Option<HashMap<String, Wire>> {
    let catalogue: ModelsDev = serde_json::from_slice(body).ok()?;
    let models = catalogue.go?.models;
    Some(
        models
            .into_iter()
            .map(|(id, m)| {
                let npm = m.provider.as_ref().and_then(|p| p.npm.as_deref());
                let wire = wire_from_npm(npm);
                (id, wire)
            })
            .collect(),
    )
}

struct Cached {
    at: Instant,
    /// `None` after a failed fetch.
    formats: Option<HashMap<String, Wire>>,
}

/// In-memory catalogue cache, shared by every request of one client.
#[derive(Default)]
pub(crate) struct Catalog {
    cached: Mutex<Option<Cached>>,
}

impl Catalog {
    /// The cached format of `model` when the cache is fresh; `None` means the
    /// catalogue must be fetched first.
    pub(crate) fn lookup(&self, model: &str, now: Instant) -> Option<Wire> {
        let cached = self.cached.lock().unwrap_or_else(|p| p.into_inner());
        let cached = cached.as_ref()?;
        let max_age = if cached.formats.is_some() {
            MAX_AGE
        } else {
            RETRY_AFTER_FAILURE
        };
        if now.saturating_duration_since(cached.at) >= max_age {
            return None;
        }
        Some(format_in(cached.formats.as_ref(), model))
    }

    /// Stores a fetch result (`None` for a failed fetch) and returns `model`'s format.
    pub(crate) fn store(
        &self,
        formats: Option<HashMap<String, Wire>>,
        model: &str,
        now: Instant,
    ) -> Wire {
        let wire = format_in(formats.as_ref(), model);
        *self.cached.lock().unwrap_or_else(|p| p.into_inner()) = Some(Cached { at: now, formats });
        wire
    }
}

/// `model`'s format; chat completions when unknown or nothing was fetched.
fn format_in(formats: Option<&HashMap<String, Wire>>, model: &str) -> Wire {
    formats
        .and_then(|f| f.get(model))
        .copied()
        .unwrap_or(Wire::Chat)
}

#[derive(Deserialize)]
struct ModelsDev {
    #[serde(rename = "opencode-go")]
    go: Option<DevProvider>,
}

#[derive(Deserialize)]
struct DevProvider {
    #[serde(default)]
    models: HashMap<String, DevModel>,
}

#[derive(Deserialize)]
struct DevModel {
    #[serde(default)]
    provider: Option<DevSdk>,
}

#[derive(Deserialize)]
struct DevSdk {
    #[serde(default)]
    npm: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_packages_map_to_formats() {
        assert_eq!(wire_from_npm(Some("@ai-sdk/anthropic")), Wire::Messages);
        assert_eq!(wire_from_npm(Some("@ai-sdk/openai")), Wire::Responses);
        assert_eq!(wire_from_npm(Some("@ai-sdk/openai-compatible")), Wire::Chat);
        assert_eq!(wire_from_npm(Some("@ai-sdk/google")), Wire::Chat);
        assert_eq!(wire_from_npm(None), Wire::Chat);
    }

    #[test]
    fn models_dev_catalogue_is_read() {
        let body = serde_json::json!({
            "anthropic": {"models": {"claude-x": {"provider": {"npm": "@ai-sdk/anthropic"}}}},
            "opencode-go": {"npm": "@ai-sdk/openai-compatible", "models": {
                "minimax-m3": {"name": "MiniMax", "provider": {"npm": "@ai-sdk/anthropic"}},
                "gpt-oss": {"provider": {"npm": "@ai-sdk/openai"}},
                "kimi-k2.6": {"name": "Kimi"}
            }}
        });
        let formats = formats_from_models_dev(body.to_string().as_bytes()).unwrap();
        assert_eq!(formats.len(), 3);
        assert_eq!(formats["minimax-m3"], Wire::Messages);
        assert_eq!(formats["gpt-oss"], Wire::Responses);
        assert_eq!(formats["kimi-k2.6"], Wire::Chat);
        assert_eq!(formats_from_models_dev(b"{}"), None);
        assert_eq!(formats_from_models_dev(b"nope"), None);
    }

    #[test]
    fn cache_is_fresh_for_24_hours_and_failures_for_5_minutes() {
        let t0 = Instant::now();
        let catalog = Catalog::default();
        assert_eq!(catalog.lookup("m", t0), None);
        let formats = HashMap::from([("m".to_string(), Wire::Messages)]);
        assert_eq!(catalog.store(Some(formats), "m", t0), Wire::Messages);
        assert_eq!(catalog.lookup("m", t0 + MAX_AGE / 2), Some(Wire::Messages));
        assert_eq!(catalog.lookup("other", t0), Some(Wire::Chat));
        assert_eq!(catalog.lookup("m", t0 + MAX_AGE), None);

        assert_eq!(catalog.store(None, "m", t0), Wire::Chat);
        assert_eq!(
            catalog.lookup("m", t0 + Duration::from_secs(60)),
            Some(Wire::Chat)
        );
        assert_eq!(catalog.lookup("m", t0 + RETRY_AFTER_FAILURE), None);
    }
}
