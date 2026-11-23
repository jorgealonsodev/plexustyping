//! The ChatGPT plan API (S3): the streamed Responses reply, its plan errors and
//! the plan's model list. Pure functions, no I/O; ported from PlexusTerm's
//! `src-tauri/src/ai/chatgpt.rs` and checked against
//! developers.openai.com/siwc/token-sharing-open-source (models-and-inference,
//! errors-and-recovery, preview-limitations).
//!
//! Messages are Spanish (the provider is used on the user's own plan, and the
//! app shows them as notices) and never contain a token: the caller masks the
//! token in every error as for the other providers.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use serde_json::Value;

use crate::chatgpt_auth::AuthError;
use crate::{AiClient, ProviderError, ProviderId};

/// Longest model id accepted from the model list.
const MAX_ID: usize = 200;

/// The plan limit notice of S3.
pub(crate) const USAGE_LIMIT: &str = "Has alcanzado el límite de tu plan de ChatGPT";
/// What [`AuthError::NotSignedIn`] becomes (S4).
pub(crate) const NOT_SIGNED_IN: &str = "Inicia sesión en ChatGPT";
/// What [`AuthError::PlanNotEnabled`] becomes.
pub(crate) const PLAN_NOT_ENABLED: &str = "Tu cuenta de ChatGPT no tiene activado el uso del plan";

/// A ChatGPT plan message the user sees, without its language: the app
/// translates it, while [`ProviderError`] keeps carrying the Spanish text
/// ([`PlanMessage::recognize`] turns that text back into the message).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanMessage {
    /// The plan's usage limit was reached.
    UsageLimit,
    /// The plan's usage could not be checked now.
    UsageUnavailable,
    /// The account, workspace or policy does not allow the plan here.
    NotEligible,
    /// The request uses a capability (`param`) the plan does not support.
    Unsupported { param: String },
    /// The request uses a capability the plan does not support.
    UnsupportedRequest,
    /// The route or scope is not authorized for the app.
    NotAuthorized,
    /// The session could not be validated.
    InvalidUser,
    /// HTTP 401.
    SessionRejected,
    /// HTTP 403.
    Forbidden,
    /// HTTP 3xx.
    Redirected { status: u16 },
    /// HTTP 429.
    Throttled,
    /// Another refusal, with its error code.
    Rejected { status: u16, code: String },
    /// Another refusal, without an error code.
    RejectedStatus { status: u16 },
    /// No account is signed in.
    NotSignedIn,
    /// The grant lacks plan use.
    PlanNotEnabled,
}

impl PlanMessage {
    /// Every message without parameters.
    const FIXED: [PlanMessage; 11] = [
        PlanMessage::UsageLimit,
        PlanMessage::UsageUnavailable,
        PlanMessage::NotEligible,
        PlanMessage::UnsupportedRequest,
        PlanMessage::NotAuthorized,
        PlanMessage::InvalidUser,
        PlanMessage::SessionRejected,
        PlanMessage::Forbidden,
        PlanMessage::Throttled,
        PlanMessage::NotSignedIn,
        PlanMessage::PlanNotEnabled,
    ];

    /// The Spanish text with `{name}` placeholders for [`PlanMessage::params`].
    pub fn template(&self) -> &'static str {
        match self {
            PlanMessage::UsageLimit => USAGE_LIMIT,
            PlanMessage::UsageUnavailable => {
                "ChatGPT no pudo comprobar el uso de tu plan ahora; inténtalo más tarde"
            }
            PlanMessage::NotEligible => {
                "Tu plan de ChatGPT no se puede usar en PlexusTyping (cuenta, espacio de trabajo o política)"
            }
            PlanMessage::Unsupported { .. } => {
                "ChatGPT no admite \"{param}\" en esta petición. Prueba con otro modelo"
            }
            PlanMessage::UnsupportedRequest => {
                "ChatGPT no admite esta petición. Prueba con otro modelo"
            }
            PlanMessage::NotAuthorized => "ChatGPT no autorizó esta petición de PlexusTyping",
            PlanMessage::InvalidUser => {
                "ChatGPT no pudo validar tu sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT"
            }
            PlanMessage::SessionRejected => {
                "ChatGPT no aceptó la sesión. Si sigue fallando, vuelve a iniciar sesión en ChatGPT"
            }
            PlanMessage::Forbidden => {
                "ChatGPT no permitió esta petición (región, permisos o política)"
            }
            PlanMessage::Redirected { .. } => {
                "ChatGPT redirigió la petición (HTTP {status}); no se siguen redirecciones"
            }
            PlanMessage::Throttled => {
                "ChatGPT está limitando las peticiones; inténtalo en un momento"
            }
            PlanMessage::Rejected { .. } => {
                "ChatGPT rechazó la petición (HTTP {status}, {code}). Revisa el modelo"
            }
            PlanMessage::RejectedStatus { .. } => {
                "ChatGPT rechazó la petición (HTTP {status}). Revisa el modelo"
            }
            PlanMessage::NotSignedIn => NOT_SIGNED_IN,
            PlanMessage::PlanNotEnabled => PLAN_NOT_ENABLED,
        }
    }

    /// The values of the template's placeholders, by name.
    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            PlanMessage::Unsupported { param } => vec![("param", param.clone())],
            PlanMessage::Redirected { status } | PlanMessage::RejectedStatus { status } => {
                vec![("status", status.to_string())]
            }
            PlanMessage::Rejected { status, code } => {
                vec![("status", status.to_string()), ("code", code.clone())]
            }
            _ => Vec::new(),
        }
    }

    /// The message whose Spanish text is `message`, if any.
    pub fn recognize(message: &str) -> Option<Self> {
        if let Some(fixed) = Self::FIXED.into_iter().find(|m| m.template() == message) {
            return Some(fixed);
        }
        let status = |v: &str| v.parse::<u16>().ok();
        let shaped = |m: &PlanMessage| captures(m.template(), message);
        let probe = PlanMessage::Rejected {
            status: 0,
            code: String::new(),
        };
        let found = if let Some(v) = shaped(&probe) {
            PlanMessage::Rejected {
                status: status(v.first()?)?,
                code: v.get(1)?.clone(),
            }
        } else if let Some(v) = shaped(&PlanMessage::RejectedStatus { status: 0 }) {
            PlanMessage::RejectedStatus {
                status: status(v.first()?)?,
            }
        } else if let Some(v) = shaped(&PlanMessage::Redirected { status: 0 }) {
            PlanMessage::Redirected {
                status: status(v.first()?)?,
            }
        } else {
            let v = shaped(&PlanMessage::Unsupported {
                param: String::new(),
            })?;
            PlanMessage::Unsupported {
                param: v.first()?.clone(),
            }
        };
        // Only an exact round trip counts (a value may contain a template's text).
        (found.to_string() == message).then_some(found)
    }
}

impl std::fmt::Display for PlanMessage {
    /// The Spanish text, as [`ProviderError`] carries it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self
            .params()
            .iter()
            .fold(self.template().to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            });
        f.write_str(&text)
    }
}

/// The values of `template`'s `{name}` placeholders in `message`, in order;
/// `None` when `message` does not have the template's shape.
fn captures(template: &str, message: &str) -> Option<Vec<String>> {
    let mut literals = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let end = start + rest[start..].find('}')?;
        literals.push(&rest[..start]);
        rest = &rest[end + 1..];
    }
    literals.push(rest);
    let (first, later) = literals.split_first()?;
    let mut text = message.strip_prefix(first)?;
    let mut values = Vec::new();
    for (i, literal) in later.iter().enumerate() {
        let at = if i + 1 == later.len() {
            text.strip_suffix(literal).map(str::len)?
        } else {
            text.find(literal)?
        };
        values.push(text[..at].to_owned());
        text = &text[at + literal.len()..];
    }
    Some(values)
}

impl From<AuthError> for ProviderError {
    /// A sign-in that cannot give a token. Its messages are already Spanish and
    /// never carry a credential.
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::NotSignedIn | AuthError::NoStorage => ProviderError::Auth {
                message: PlanMessage::NotSignedIn.to_string(),
            },
            AuthError::PlanNotEnabled => ProviderError::Auth {
                message: PlanMessage::PlanNotEnabled.to_string(),
            },
            AuthError::Network { .. } | AuthError::Unavailable { .. } => ProviderError::Network {
                message: err.to_string(),
            },
            other => ProviderError::Auth {
                message: other.to_string(),
            },
        }
    }
}

/// An error code reduced to safe characters for a message.
fn safe_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(64)
        .collect()
}

/// The `response` object of the stream's terminal event, with the text of its
/// `output_text` deltas; or the error the stream carried. Only
/// `response.completed` and `response.incomplete` end it; a stream that stops
/// before either is not a success ("Treat inference as successful only after
/// receiving response.completed").
fn terminal_event(body: &str) -> Result<(Value, String), ProviderError> {
    let mut data = String::new();
    let mut deltas = String::new();
    // `lines` also splits `\r\n`; the final empty line flushes the last event.
    for line in body.lines().chain(std::iter::once("")) {
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            continue;
        }
        if !line.trim().is_empty() || data.is_empty() {
            continue;
        }
        let event: Value = serde_json::from_str(&data).unwrap_or(Value::Null);
        data.clear();
        match event["type"].as_str() {
            Some("response.output_text.delta") => {
                deltas.push_str(event["delta"].as_str().unwrap_or_default());
            }
            Some("response.completed") => return Ok((event["response"].clone(), deltas)),
            // The event type is what says the answer was cut, even when the
            // `response` it carries has no status.
            Some("response.incomplete") => {
                let mut response = event["response"].clone();
                if let Some(object) = response.as_object_mut() {
                    object.insert("status".into(), "incomplete".into());
                }
                return Ok((response, deltas));
            }
            Some("response.failed") => {
                let error = &event["response"]["error"];
                return Err(api_error(
                    0,
                    error["code"].as_str(),
                    error["param"].as_str(),
                ));
            }
            Some("error") => {
                let code = event["code"]
                    .as_str()
                    .or_else(|| event["error"]["code"].as_str());
                let param = event["param"]
                    .as_str()
                    .or_else(|| event["error"]["param"].as_str());
                return Err(api_error(0, code, param));
            }
            _ => {}
        }
    }
    Err(ProviderError::Network {
        message: "La respuesta de ChatGPT se cortó antes de terminar; inténtalo de nuevo.".into(),
    })
}

/// The terminal `response` as a Responses API body, with the delta text as its
/// output when the event carried none.
pub(crate) fn stream_response(body: &str) -> Result<Value, ProviderError> {
    let (mut response, deltas) = terminal_event(body)?;
    // Checked first: indexing a non-object `Value` mutably panics.
    if !response.is_object() {
        return Err(ProviderError::bad_response(
            "ChatGPT sent an answer PlexusTyping could not read.",
        ));
    }
    let has_text = response["output"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item["content"]
                .as_array()
                .is_some_and(|parts| parts.iter().any(|p| p["type"] == "output_text"))
        })
    });
    if !has_text && response["status"] != "incomplete" && !deltas.is_empty() {
        response["output"] = serde_json::json!([{
            "type": "message",
            "content": [{"type": "output_text", "text": deltas}]
        }]);
    }
    Ok(response)
}

/// Maps a non-2xx answer. Admission failures may carry `{"detail": "..."}`
/// instead of an error object; that text is diagnostic only and never shown.
pub(crate) fn status_error(status: u16, body: &str) -> ProviderError {
    let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let error = &value["error"];
    api_error(status, error["code"].as_str(), error["param"].as_str())
}

/// The error for a plan error code (`status` 0 when it came in the stream),
/// following OpenAI's "Errors and recovery" table.
fn api_error(status: u16, code: Option<&str>, param: Option<&str>) -> ProviderError {
    let config = |message: PlanMessage| ProviderError::Config {
        message: message.to_string(),
    };
    match code.unwrap_or_default() {
        "subscription_sharing_usage_limit_exceeded" => ProviderError::RateLimit {
            message: PlanMessage::UsageLimit.to_string(),
        },
        // "Usage availability could not be checked. Preserve credentials and retry later."
        "subscription_sharing_usage_unavailable" | "subscription_sharing_user_unavailable" => {
            ProviderError::RateLimit {
                message: PlanMessage::UsageUnavailable.to_string(),
            }
        }
        "subscription_sharing_user_not_eligible" => config(PlanMessage::NotEligible),
        "subscription_sharing_unsupported_capability" => {
            match param.map(safe_code).filter(|p| !p.is_empty()) {
                Some(param) => config(PlanMessage::Unsupported { param }),
                None => config(PlanMessage::UnsupportedRequest),
            }
        }
        "subscription_sharing_route_not_supported"
        | "chatpass_v2_scope_not_authorized"
        | "chatpass_v2_invalid_authorization_context" => config(PlanMessage::NotAuthorized),
        "subscription_sharing_invalid_user" => ProviderError::Auth {
            message: PlanMessage::InvalidUser.to_string(),
        },
        other => {
            let code = Some(safe_code(other)).filter(|c| !c.is_empty());
            match (status, code) {
                (401, _) => ProviderError::Auth {
                    message: PlanMessage::SessionRejected.to_string(),
                },
                (403, _) => ProviderError::Auth {
                    message: PlanMessage::Forbidden.to_string(),
                },
                (300..=399, _) => config(PlanMessage::Redirected { status }),
                (429, _) => ProviderError::RateLimit {
                    message: PlanMessage::Throttled.to_string(),
                },
                (500..=599, _) => ProviderError::Network {
                    message: format!("ChatGPT no está disponible ahora (HTTP {status})"),
                },
                (0, Some(code)) => {
                    ProviderError::bad_response(format!("ChatGPT no pudo responder ({code})"))
                }
                (0, None) => ProviderError::bad_response("ChatGPT no pudo responder"),
                (_, Some(code)) => config(PlanMessage::Rejected { status, code }),
                (_, None) => config(PlanMessage::RejectedStatus { status }),
            }
        }
    }
}

/// Display names (slug → name) of the plan's models, from the last catalogue
/// read. Names are only shown in the settings; the slug is what is stored and sent.
static NAMES: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(Mutex::default);

/// The display name of `slug` from the last catalogue read, else the slug.
pub(crate) fn display_name(slug: &str) -> String {
    let names = NAMES.lock().unwrap_or_else(|p| p.into_inner());
    names.get(slug).cloned().unwrap_or_else(|| slug.to_string())
}

/// Remembers the display names of a catalogue's listed models (a model listed
/// without a usable name forgets the old one).
fn remember_names(listed: &[(String, Option<String>)]) {
    let mut names = NAMES.lock().unwrap_or_else(|p| p.into_inner());
    for (slug, name) in listed {
        let name = name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty() && !n.chars().any(char::is_control))
            .map(|n| n.chars().take(MAX_ID).collect::<String>());
        match name {
            Some(name) => names.insert(slug.clone(), name),
            None => names.remove(slug),
        };
    }
}

impl AiClient {
    /// The ChatGPT plan's models as `(slug, display name)` in server order, read
    /// like [`AiClient::list_models`] with the signed-in account; the name is the
    /// slug when the catalogue gives none.
    pub async fn chatgpt_model_names(&self) -> Result<Vec<(String, String)>, ProviderError> {
        let slugs = self.list_models(ProviderId::Chatgpt, None).await?;
        Ok(slugs
            .into_iter()
            .map(|slug| {
                let name = display_name(&slug);
                (slug, name)
            })
            .collect())
    }
}

/// The plan's models from `GET /models`: `models[]` with `visibility == "list"`,
/// in server order, as `(slug, display_name)`; slugs are deduplicated and must
/// be short, without whitespace or control characters. Their display names are
/// remembered for [`AiClient::chatgpt_model_names`].
pub(crate) fn models(value: &Value) -> Option<Vec<(String, Option<String>)>> {
    let mut seen = std::collections::HashSet::new();
    let listed: Vec<(String, Option<String>)> = value
        .get("models")?
        .as_array()?
        .iter()
        .filter(|m| m["visibility"] == "list")
        .filter_map(|m| {
            let slug = m["slug"].as_str()?;
            let valid = !slug.is_empty()
                && slug.chars().count() <= MAX_ID
                && !slug.chars().any(|c| c.is_control() || c.is_whitespace());
            (valid && seen.insert(slug.to_string())).then(|| {
                (
                    slug.to_string(),
                    m["display_name"].as_str().map(str::to_string),
                )
            })
        })
        .collect();
    remember_names(&listed);
    Some(listed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_a_terminal_event_ends_the_stream() {
        let completed = "event: response.created\ndata: {\"type\":\"response.created\"}\n\nevent: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\nevent: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":2,\"output_tokens\":1}}}\n\n";
        let response = stream_response(completed).unwrap();
        assert_eq!(response["status"], "completed");
        assert_eq!(response["output"][0]["content"][0]["text"], "Hi");
        assert_eq!(response["usage"]["input_tokens"], 2);
        let crlf = completed.replace('\n', "\r\n");
        assert_eq!(stream_response(&crlf).unwrap()["status"], "completed");
        // The last event needs no blank line after it.
        let unterminated =
            "data: {\"type\":\"response.incomplete\",\"response\":{\"status\":\"incomplete\"}}";
        assert_eq!(
            stream_response(unterminated).unwrap()["status"],
            "incomplete"
        );
        let cut = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hi\"}\n\n";
        assert!(matches!(
            stream_response(cut),
            Err(ProviderError::Network { .. })
        ));
        assert!(matches!(
            stream_response(""),
            Err(ProviderError::Network { .. })
        ));
    }

    #[test]
    fn an_incomplete_event_without_status_is_still_incomplete() {
        let body = format!(
            "data: {}\n\ndata: {}\n\n",
            json!({"type": "response.output_text.delta", "delta": "partial"}),
            json!({"type": "response.incomplete", "response": {"output": []}})
        );
        let response = stream_response(&body).unwrap();
        assert_eq!(response["status"], "incomplete");
        assert_eq!(response["output"], json!([]));
    }

    #[test]
    fn a_terminal_event_without_a_response_object_is_unreadable() {
        for response in [json!("x"), json!([1]), json!(null), json!(3)] {
            for kind in ["response.completed", "response.incomplete"] {
                let body = format!(
                    "data: {}\n\ndata: {}\n\n",
                    json!({"type": "response.output_text.delta", "delta": "partial"}),
                    json!({"type": kind, "response": response})
                );
                assert!(
                    matches!(
                        stream_response(&body),
                        Err(ProviderError::BadResponse { .. })
                    ),
                    "{kind} {response}"
                );
            }
        }
    }

    #[test]
    fn data_split_over_several_lines_is_one_event() {
        let body = "data: {\"type\":\"response.completed\",\ndata: \"response\":{\"status\":\"completed\",\"output\":[]}}\n\n";
        assert_eq!(stream_response(body).unwrap()["status"], "completed");
    }

    #[test]
    fn the_final_output_wins_over_the_deltas() {
        let body = format!(
            "data: {}\n\ndata: {}\n\n",
            json!({"type": "response.output_text.delta", "delta": "partial"}),
            json!({"type": "response.completed", "response": {"status": "completed",
                   "output": [{"type": "message", "content": [{"type": "output_text", "text": "full"}]}]}})
        );
        let response = stream_response(&body).unwrap();
        assert_eq!(response["output"][0]["content"][0]["text"], "full");
    }

    #[test]
    fn a_failed_stream_carries_the_plan_error() {
        let failed = "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"subscription_sharing_usage_limit_exceeded\"}}}\n\n";
        assert_eq!(
            stream_response(failed).unwrap_err(),
            ProviderError::RateLimit {
                message: USAGE_LIMIT.into()
            }
        );
        let nested = "data: {\"type\":\"error\",\"error\":{\"code\":\"subscription_sharing_unsupported_capability\",\"param\":\"tools\"}}\n\n";
        assert_eq!(
            stream_response(nested).unwrap_err(),
            ProviderError::Config {
                message: "ChatGPT no admite \"tools\" en esta petición. Prueba con otro modelo"
                    .into()
            }
        );
    }

    #[test]
    fn auth_errors_become_provider_errors_without_tokens() {
        assert_eq!(
            ProviderError::from(AuthError::NotSignedIn),
            ProviderError::Auth {
                message: NOT_SIGNED_IN.into()
            }
        );
        assert_eq!(
            ProviderError::from(AuthError::NoStorage),
            ProviderError::Auth {
                message: NOT_SIGNED_IN.into()
            }
        );
        assert_eq!(
            ProviderError::from(AuthError::PlanNotEnabled),
            ProviderError::Auth {
                message: PLAN_NOT_ENABLED.into()
            }
        );
        assert!(matches!(
            ProviderError::from(AuthError::Unavailable { status: 503 }),
            ProviderError::Network { .. }
        ));
        assert_eq!(
            ProviderError::from(AuthError::SessionExpired),
            ProviderError::Auth {
                message: "Tu sesión de ChatGPT ha caducado. Vuelve a iniciar sesión".into()
            }
        );
    }

    #[test]
    fn models_keep_listed_ones_in_server_order() {
        let value = json!({"models": [
            {"slug": "b", "display_name": "B", "visibility": "list"},
            {"slug": "hidden", "visibility": "hide"},
            {"slug": "a", "visibility": "list"},
            {"slug": "b", "visibility": "list"}
        ]});
        assert_eq!(
            models(&value).unwrap(),
            [
                ("b".to_string(), Some("B".to_string())),
                ("a".to_string(), None)
            ]
        );
        assert_eq!(models(&json!({"data": []})), None);
    }

    #[test]
    fn display_names_follow_the_last_catalogue_read() {
        let first = json!({"models": [
            {"slug": "names-x", "display_name": "X One", "visibility": "list"},
            {"slug": "names-y", "display_name": "Y", "visibility": "hide"}
        ]});
        models(&first).unwrap();
        assert_eq!(display_name("names-x"), "X One");
        assert_eq!(
            display_name("names-y"),
            "names-y",
            "hidden models are not named"
        );
        assert_eq!(display_name("names-unknown"), "names-unknown");
        // A later catalogue without a name drops the old one; a blank name is the slug.
        let second = json!({"models": [
            {"slug": "names-x", "visibility": "list"},
            {"slug": "names-z", "display_name": "  ", "visibility": "list"}
        ]});
        models(&second).unwrap();
        assert_eq!(display_name("names-x"), "names-x");
        assert_eq!(display_name("names-z"), "names-z");
    }

    fn every_plan_message() -> Vec<PlanMessage> {
        let mut all = PlanMessage::FIXED.to_vec();
        all.extend([
            PlanMessage::Unsupported {
                param: "tools".into(),
            },
            PlanMessage::Redirected { status: 302 },
            PlanMessage::Rejected {
                status: 400,
                code: "bad_model".into(),
            },
            PlanMessage::RejectedStatus { status: 404 },
        ]);
        all
    }

    #[test]
    fn plan_messages_are_recognized_from_their_spanish_text() {
        for message in every_plan_message() {
            let text = message.to_string();
            assert!(!text.contains('{'), "{text}");
            assert_eq!(PlanMessage::recognize(&text), Some(message), "{text}");
        }
        assert_eq!(
            PlanMessage::Rejected {
                status: 400,
                code: "x".into()
            }
            .to_string(),
            "ChatGPT rechazó la petición (HTTP 400, x). Revisa el modelo"
        );
        assert_eq!(PlanMessage::recognize("something else"), None);
        assert_eq!(
            PlanMessage::recognize(
                "ChatGPT redirigió la petición (HTTP abc); no se siguen redirecciones"
            ),
            None
        );
    }

    #[test]
    fn plan_errors_carry_the_text_of_their_plan_message() {
        let cases = [
            (
                api_error(0, Some("subscription_sharing_usage_limit_exceeded"), None),
                PlanMessage::UsageLimit,
            ),
            (
                api_error(
                    0,
                    Some("subscription_sharing_unsupported_capability"),
                    Some("tools"),
                ),
                PlanMessage::Unsupported {
                    param: "tools".into(),
                },
            ),
            (api_error(429, None, None), PlanMessage::Throttled),
            (
                api_error(400, Some("bad"), None),
                PlanMessage::Rejected {
                    status: 400,
                    code: "bad".into(),
                },
            ),
            (
                ProviderError::from(AuthError::PlanNotEnabled),
                PlanMessage::PlanNotEnabled,
            ),
        ];
        for (err, expected) in cases {
            let (ProviderError::Auth { message }
            | ProviderError::RateLimit { message }
            | ProviderError::Config { message }) = &err
            else {
                panic!("{err:?}");
            };
            assert_eq!(PlanMessage::recognize(message), Some(expected));
        }
    }
}
