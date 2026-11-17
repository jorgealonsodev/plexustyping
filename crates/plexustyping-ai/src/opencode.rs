//! OpenCode Go: which API format each model is served with, and the lowest
//! reasoning effort it accepts.
//!
//! The format comes from models.dev (`["opencode-go"].models[id].provider.npm`;
//! a model without one uses the provider default, OpenAI-compatible chat). A
//! model marked `reasoning: true` whose `reasoning_options` include an `effort`
//! option listing `none`, `minimal` or `low` is asked for the lowest of them
//! (in that order, whatever order the catalogue lists them in), so short practice
//! texts come back quickly. The catalogue is
//! fetched on the first OpenCode Go generation and kept in memory for 24 hours.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;

use crate::providers::Wire;

/// How long a fetched catalogue is used.
pub(crate) const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed fetch, chat completions are assumed for this long.
pub(crate) const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(5 * 60);
/// models.dev is a few megabytes; anything far larger is refused.
pub(crate) const MAX_CATALOGUE_BYTES: usize = 32 * 1024 * 1024;
/// Effort values asked for, lowest first.
const LOW_EFFORTS: [&str; 3] = ["none", "minimal", "low"];

/// What the catalogue says about one OpenCode Go model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModelInfo {
    /// The API format it is served with.
    pub wire: Wire,
    /// The lowest effort it offers among [`LOW_EFFORTS`], if it offers one.
    pub low_effort: Option<&'static str>,
}

/// Maps the AI SDK package models.dev names to the wire format.
pub(crate) fn wire_from_npm(npm: Option<&str>) -> Wire {
    match npm {
        Some("@ai-sdk/anthropic") => Wire::Messages,
        Some("@ai-sdk/openai") => Wire::Responses,
        _ => Wire::Chat,
    }
}

/// The lowest of [`LOW_EFFORTS`] that a reasoning model lists in an `effort`
/// option. Read leniently: an odd shape means no effort, never a failed catalogue.
fn low_effort(reasoning: &Value, options: &Value) -> Option<&'static str> {
    if reasoning.as_bool() != Some(true) {
        return None;
    }
    let offered: Vec<&str> = options
        .as_array()?
        .iter()
        .filter(|option| option["type"] == "effort")
        .filter_map(|option| option["values"].as_array())
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    LOW_EFFORTS
        .into_iter()
        .find(|effort| offered.contains(effort))
}

/// Model id → what is known about it, from a models.dev body; `None` when unreadable.
pub(crate) fn models_from_models_dev(body: &[u8]) -> Option<HashMap<String, ModelInfo>> {
    let catalogue: ModelsDev = serde_json::from_slice(body).ok()?;
    let models = catalogue.go?.models;
    Some(
        models
            .into_iter()
            .map(|(id, m)| {
                let npm = m.provider.as_ref().and_then(|p| p.npm.as_deref());
                let info = ModelInfo {
                    wire: wire_from_npm(npm),
                    low_effort: low_effort(&m.reasoning, &m.reasoning_options),
                };
                (id, info)
            })
            .collect(),
    )
}

struct Cached {
    at: Instant,
    /// `None` after a failed fetch.
    models: Option<HashMap<String, ModelInfo>>,
}

/// In-memory catalogue cache, shared by every request of one client.
#[derive(Default)]
pub(crate) struct Catalog {
    cached: Mutex<Option<Cached>>,
}

impl Catalog {
    /// What is cached about `model` when the cache is fresh; `None` means the
    /// catalogue must be fetched first.
    pub(crate) fn lookup(&self, model: &str, now: Instant) -> Option<ModelInfo> {
        let cached = self.cached.lock().unwrap_or_else(|p| p.into_inner());
        let cached = cached.as_ref()?;
        let max_age = if cached.models.is_some() {
            MAX_AGE
        } else {
            RETRY_AFTER_FAILURE
        };
        if now.saturating_duration_since(cached.at) >= max_age {
            return None;
        }
        Some(info_in(cached.models.as_ref(), model))
    }

    /// Stores a fetch result (`None` for a failed fetch) and returns what it says about `model`.
    pub(crate) fn store(
        &self,
        models: Option<HashMap<String, ModelInfo>>,
        model: &str,
        now: Instant,
    ) -> ModelInfo {
        let info = info_in(models.as_ref(), model);
        *self.cached.lock().unwrap_or_else(|p| p.into_inner()) = Some(Cached { at: now, models });
        info
    }
}

/// What is known about `model`; chat completions without an effort when
/// unknown or nothing was fetched.
fn info_in(models: Option<&HashMap<String, ModelInfo>>, model: &str) -> ModelInfo {
    models
        .and_then(|m| m.get(model))
        .copied()
        .unwrap_or_default()
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
    /// Kept as raw JSON so an unexpected shape never drops the catalogue.
    #[serde(default)]
    reasoning: Value,
    #[serde(default)]
    reasoning_options: Value,
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

    fn info(wire: Wire, low_effort: Option<&'static str>) -> ModelInfo {
        ModelInfo { wire, low_effort }
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
        let models = models_from_models_dev(body.to_string().as_bytes()).unwrap();
        assert_eq!(models.len(), 3);
        assert_eq!(models["minimax-m3"], info(Wire::Messages, None));
        assert_eq!(models["gpt-oss"], info(Wire::Responses, None));
        assert_eq!(models["kimi-k2.6"], info(Wire::Chat, None));
        assert_eq!(models_from_models_dev(b"{}"), None);
        assert_eq!(models_from_models_dev(b"nope"), None);
    }

    #[test]
    fn models_dev_effort_options_give_the_lowest_effort() {
        let effort = |values: serde_json::Value| serde_json::json!({"reasoning": true, "reasoning_options": [{"type": "effort", "values": values}]});
        let body = serde_json::json!({"opencode-go": {"models": {
            "glm-5.3-flash": effort(serde_json::json!(["low", "high", "max"])),
            "gpt-oss": {
                "provider": {"npm": "@ai-sdk/openai"},
                "reasoning": true,
                "reasoning_options": [{"type": "effort", "values": ["high", "minimal", "low"]}]
            },
            "gpt-6-luna": effort(serde_json::json!(["none", "low", "high"])),
            "muse-spark": effort(serde_json::json!(["minimal", "high"])),
            "only-high": effort(serde_json::json!(["high", "max"])),
            "not-reasoning": {"reasoning": false, "reasoning_options": [{"type": "effort", "values": ["low"]}]},
            "no-options": {"reasoning": true},
            "other-option": {"reasoning": true, "reasoning_options": [{"type": "budget", "values": ["low"]}]},
            "odd-shapes": {"reasoning": "yes", "reasoning_options": {"type": "effort"}},
            "odd-values": {"reasoning": true, "reasoning_options": [7, {"type": "effort", "values": "low"}]}
        }}});
        let models = models_from_models_dev(body.to_string().as_bytes()).unwrap();
        assert_eq!(models.len(), 10, "odd entries never drop the catalogue");
        assert_eq!(models["glm-5.3-flash"], info(Wire::Chat, Some("low")));
        assert_eq!(models["gpt-oss"], info(Wire::Responses, Some("minimal")));
        assert_eq!(models["gpt-6-luna"], info(Wire::Chat, Some("none")));
        assert_eq!(models["muse-spark"], info(Wire::Chat, Some("minimal")));
        for id in [
            "only-high",
            "not-reasoning",
            "no-options",
            "other-option",
            "odd-shapes",
            "odd-values",
        ] {
            assert_eq!(models[id], info(Wire::Chat, None), "{id}");
        }
    }

    #[test]
    fn cache_is_fresh_for_24_hours_and_failures_for_5_minutes() {
        let t0 = Instant::now();
        let catalog = Catalog::default();
        assert_eq!(catalog.lookup("m", t0), None);
        let low = info(Wire::Messages, Some("low"));
        let models = HashMap::from([("m".to_string(), low)]);
        assert_eq!(catalog.store(Some(models), "m", t0), low);
        assert_eq!(catalog.lookup("m", t0 + MAX_AGE / 2), Some(low));
        assert_eq!(catalog.lookup("other", t0), Some(ModelInfo::default()));
        assert_eq!(catalog.lookup("m", t0 + MAX_AGE), None);

        assert_eq!(catalog.store(None, "m", t0), ModelInfo::default());
        assert_eq!(
            catalog.lookup("m", t0 + Duration::from_secs(60)),
            Some(ModelInfo::default())
        );
        assert_eq!(catalog.lookup("m", t0 + RETRY_AFTER_FAILURE), None);
        assert_eq!(ModelInfo::default(), info(Wire::Chat, None));
    }
}
