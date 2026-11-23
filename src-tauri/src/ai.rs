//! AI practice texts wired into the app (S2, S3, S5): non-secret settings in
//! the store, keys in the system keychain, generation through the provider
//! fallback chain, then the local cache, then the corpus. ChatGPT (tu
//! suscripción) uses the signed-in account instead of a key (Sign in with
//! ChatGPT, S4): its token comes from the client's `ChatgptAuth`.
//!
//! Only target units, language, length, allowed chars and topic are sent
//! (S5), plus, for an unlimited text's continuation, the end of the current
//! generated text (never what the user typed). Keys never cross back to the frontend and are never logged. The store
//! `Mutex` is never held across an `.await`: lock, copy, drop, await, lock again.

use std::time::Duration;

use plexustyping_ai::chatgpt_auth::{AuthError, SignOut, Status};
use plexustyping_ai::{
    AiClient, ApiKey, GenerationRequest, Mode, ProviderConfig, ProviderError, ProviderId,
    ValidationIssue,
};
use plexustyping_store::{NewCachedText, Store, UsageRow};
use serde::{Deserialize, Serialize};

use crate::code::{self, PracticeMode};
use crate::commands::{
    self, AppState, DEFAULT_LENGTH, DEFAULT_TARGETS, FirstText, NextText, NextTextRequest,
    TextSource, lock_store,
};
use crate::i18n::Locale;
use crate::messages;
use crate::secrets::{self, SecretError, SharedSecrets};
use crate::unlock;

/// Store `settings` key of the JSON [`AiSettings`].
pub const SETTINGS_KEY: &str = "ai";
/// Longest fallback chain: every provider once.
pub const MAX_CHAIN: usize = ProviderId::ALL.len();
/// Longest topic, in chars.
pub const MAX_TOPIC_CHARS: usize = 60;
/// Longest model id, in chars.
pub const MAX_MODEL_CHARS: usize = 200;
/// Density range (PRD RF1: 2×–5×) and default.
pub const DENSITY_RANGE: std::ops::RangeInclusive<f32> = 2.0..=5.0;
pub const DEFAULT_DENSITY: f32 = 3.0;

/// One provider of the fallback chain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainEntry {
    pub provider: ProviderId,
    /// Model id; empty means not chosen yet (the entry is skipped).
    pub model: String,
}

/// Non-secret AI settings, stored as JSON under [`SETTINGS_KEY`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub enabled: bool,
    /// 1–[`MAX_CHAIN`] distinct providers; order = fallback order.
    pub chain: Vec<ChainEntry>,
    /// Target density over the normal frequency, `2.0..=5.0`.
    pub density: f32,
    /// Optional topic, at most [`MAX_TOPIC_CHARS`] chars.
    pub topic: Option<String>,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            enabled: false,
            chain: vec![ChainEntry {
                provider: ProviderId::Claude,
                model: ProviderId::Claude.default_model().to_owned(),
            }],
            density: DEFAULT_DENSITY,
            topic: None,
        }
    }
}

/// A provider as listed in the settings.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub label: &'static str,
    /// Short name used in the UI and notices.
    pub name: &'static str,
    pub default_model: &'static str,
}

/// A failed AI command, serialized as `{ kind, message }` (message in the interface language).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AiFailure {
    pub kind: &'static str,
    pub message: String,
}

impl AiFailure {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        AiFailure {
            kind,
            message: message.into(),
        }
    }

    fn settings(message: impl Into<String>) -> Self {
        AiFailure::new("invalidSettings", message)
    }
}

/// Why the keychain could not be used, without secret bytes.
fn keychain_failure(locale: Locale, err: SecretError) -> AiFailure {
    match err {
        SecretError::Unavailable => {
            AiFailure::new("keychain", locale.pick(messages::KEYCHAIN_UNAVAILABLE))
        }
        SecretError::Keyring { message } => AiFailure::new(
            "keychain",
            messages::KEYCHAIN_ERROR.fill(locale, &[("message", &message)]),
        ),
    }
}

/// Short provider name used in the UI and notices.
pub fn short_name(id: ProviderId) -> &'static str {
    match id {
        ProviderId::Claude => "Claude",
        ProviderId::Openai => "OpenAI",
        ProviderId::Gemini => "Gemini",
        ProviderId::OpencodeGo => "OpenCode Go",
        ProviderId::Chatgpt => "ChatGPT",
    }
}

/// Kebab-case id (the serde name), e.g. `opencode-go`.
pub fn slug(id: ProviderId) -> &'static str {
    match id {
        ProviderId::Claude => "claude",
        ProviderId::Openai => "openai",
        ProviderId::Gemini => "gemini",
        ProviderId::OpencodeGo => "opencode-go",
        ProviderId::Chatgpt => "chatgpt",
    }
}

/// Keychain account of `id`'s key.
pub fn account(id: ProviderId) -> String {
    format!("ai:{}", slug(id))
}

/// Label of `id`'s keychain item (shown by Secret Service UIs).
pub fn key_label(id: ProviderId) -> String {
    format!("PlexusTyping AI key ({})", id.label())
}

/// Every provider with its names and suggested model.
pub fn providers() -> Vec<ProviderInfo> {
    ProviderId::ALL
        .into_iter()
        .map(|id| ProviderInfo {
            id,
            label: id.label(),
            name: short_name(id),
            default_model: id.default_model(),
        })
        .collect()
}

/// The one place that turns a provider error into a short reason in `locale`.
/// ChatGPT's sign-in, plan and request errors carry a message meant for the
/// user (in Spanish, e.g. "Has alcanzado el límite de tu plan de ChatGPT"),
/// which [`messages::chatgpt_message`] translates.
pub fn provider_reason(locale: Locale, id: ProviderId, err: &ProviderError) -> String {
    if id == ProviderId::Chatgpt
        && let ProviderError::Auth { message }
        | ProviderError::RateLimit { message }
        | ProviderError::Config { message } = err
    {
        return messages::chatgpt_message(locale, message);
    }
    let texts = match err {
        ProviderError::NoKey => messages::NO_KEY,
        ProviderError::Auth { .. } => messages::KEY_REJECTED,
        ProviderError::RateLimit { .. } => messages::RATE_LIMITED,
        ProviderError::Network { .. } => messages::NO_CONNECTION,
        ProviderError::Timeout => messages::TIMED_OUT,
        ProviderError::Refused { .. } => messages::REFUSED,
        ProviderError::BadResponse { .. } | ProviderError::Invalid { .. } => messages::BAD_TEXT,
        ProviderError::Config { .. } => messages::REQUEST_REJECTED,
    };
    texts.fill(locale, &[("name", short_name(id))])
}

/// One stderr line per failed attempt (S3): a rejected text lists every
/// validation issue (lengths, characters and targets only); any other failure
/// (malformed reply, timeout, rejected key…) its message, which the client has
/// already stripped of the key.
fn rejection_lines(errors: &[(ProviderId, ProviderError)]) -> Vec<String> {
    errors
        .iter()
        .map(|(id, err)| {
            let ProviderError::Invalid { issues } = err else {
                return format!(
                    "PlexusTyping: AI attempt with {} failed: {err}",
                    short_name(*id)
                );
            };
            let listed: Vec<String> = issues
                .iter()
                .map(|issue| match issue {
                    ValidationIssue::Length { len } => format!("length {len}"),
                    ValidationIssue::CharNotAllowed { ch } => format!("char not allowed {ch:?}"),
                    ValidationIssue::Markdown => "markdown".to_owned(),
                    ValidationIssue::LowDensity {
                        target,
                        found,
                        needed,
                    } => format!("low density {target:?} found {found} needed {needed}"),
                    ValidationIssue::Empty => "empty".to_owned(),
                })
                .collect();
            format!(
                "PlexusTyping: AI text from {} rejected: {}",
                short_name(*id),
                listed.join("; ")
            )
        })
        .collect()
}

/// [`ProviderError`] as a command failure.
fn provider_failure(locale: Locale, id: ProviderId, err: &ProviderError) -> AiFailure {
    let kind = match err {
        ProviderError::NoKey => "noKey",
        ProviderError::Auth { .. } => "auth",
        ProviderError::RateLimit { .. } => "rateLimit",
        ProviderError::Network { .. } => "network",
        ProviderError::Timeout => "timeout",
        ProviderError::Refused { .. } => "refused",
        ProviderError::BadResponse { .. } => "badResponse",
        ProviderError::Config { .. } => "config",
        ProviderError::Invalid { .. } => "invalid",
    };
    AiFailure::new(kind, provider_reason(locale, id, err))
}

/// Normalized `s` (trimmed model and topic, empty topic = none) or why it is invalid.
pub fn validate_settings(locale: Locale, mut s: AiSettings) -> Result<AiSettings, AiFailure> {
    let refuse = |texts: crate::i18n::Texts| Err(AiFailure::settings(locale.pick(texts)));
    if s.chain.is_empty() || s.chain.len() > MAX_CHAIN {
        let max = MAX_CHAIN.to_string();
        return Err(AiFailure::settings(
            messages::CHAIN_SIZE.fill(locale, &[("max", &max)]),
        ));
    }
    for i in 0..s.chain.len() {
        let model = s.chain[i].model.trim().to_owned();
        if model.chars().count() > MAX_MODEL_CHARS {
            return refuse(messages::MODEL_TOO_LONG);
        }
        s.chain[i].model = model;
        if s.chain[..i]
            .iter()
            .any(|e| e.provider == s.chain[i].provider)
        {
            let name = short_name(s.chain[i].provider);
            return Err(AiFailure::settings(
                messages::CHAIN_DUPLICATE.fill(locale, &[("name", name)]),
            ));
        }
    }
    if !s.density.is_finite() || !DENSITY_RANGE.contains(&s.density) {
        return refuse(messages::DENSITY_RANGE);
    }
    s.topic = s
        .topic
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty());
    if let Some(topic) = &s.topic {
        if topic.chars().count() > MAX_TOPIC_CHARS {
            let max = MAX_TOPIC_CHARS.to_string();
            return Err(AiFailure::settings(
                messages::TOPIC_TOO_LONG.fill(locale, &[("max", &max)]),
            ));
        }
        if topic.chars().any(char::is_control) {
            return refuse(messages::TOPIC_LINE_BREAKS);
        }
    }
    Ok(s)
}

/// Stored settings; the defaults when missing or no longer valid.
pub fn load_settings(store: &Store) -> AiSettings {
    let stored = store.get_setting(SETTINGS_KEY).ok().flatten();
    stored
        .and_then(|json| serde_json::from_str(&json).ok())
        // Only whether it is valid matters here, not the message's language.
        .and_then(|s| validate_settings(Locale::default(), s).ok())
        .unwrap_or_default()
}

/// Validates and stores `s`; returns what was stored.
pub fn save_settings(
    locale: Locale,
    store: &Store,
    s: AiSettings,
) -> Result<AiSettings, AiFailure> {
    let s = validate_settings(locale, s)?;
    let json = serde_json::to_string(&s).map_err(|e| AiFailure::new("storage", e.to_string()))?;
    store.set_setting(SETTINGS_KEY, &json).map_err(|e| {
        let error = e.to_string();
        AiFailure::new(
            "storage",
            messages::SAVE_FAILED.fill(locale, &[("error", &error)]),
        )
    })?;
    Ok(s)
}

/// The stored key of `id`, `None` when there is none.
async fn load_key(
    locale: Locale,
    secrets: &SharedSecrets,
    id: ProviderId,
) -> Result<Option<ApiKey>, AiFailure> {
    let account = account(id);
    let stored = secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.get(&account)
    })
    .await
    .map_err(|e| keychain_failure(locale, e))?;
    Ok(stored.and_then(|k| ApiKey::new(k).ok()))
}

/// Whether a key is saved for `id`; the key itself never leaves the backend.
/// For ChatGPT: whether its account is signed in with plan use enabled.
pub async fn key_status(
    locale: Locale,
    client: &AiClient,
    secrets: &SharedSecrets,
    id: ProviderId,
) -> Result<bool, AiFailure> {
    if id == ProviderId::Chatgpt {
        let Some(auth) = client.chatgpt() else {
            return Ok(false);
        };
        return match auth.status().await {
            Ok(status) => Ok(status.plan_enabled),
            Err(AuthError::NoStorage) => Ok(false),
            Err(err) => Err(provider_failure(locale, id, &err.into())),
        };
    }
    Ok(load_key(locale, secrets, id).await?.is_some())
}

/// Saves `key` (trimmed) for `id` in the system keychain. ChatGPT is refused:
/// it uses the signed-in account.
pub async fn key_save(
    locale: Locale,
    secrets: &SharedSecrets,
    id: ProviderId,
    key: String,
) -> Result<(), AiFailure> {
    if id == ProviderId::Chatgpt {
        // Why ChatGPT takes no key.
        return Err(AiFailure::new(
            "config",
            locale.pick(messages::CHATGPT_NO_KEY),
        ));
    }
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err(AiFailure::new("noKey", locale.pick(messages::EMPTY_KEY)));
    }
    let (account, label) = (account(id), key_label(id));
    secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.set(&account, &label, &key)
    })
    .await
    .map_err(|e| keychain_failure(locale, e))
}

/// Deletes `id`'s key; deleting a missing key succeeds. ChatGPT keeps nothing
/// in the keychain (signing out is its own action), so it is a no-op.
pub async fn key_delete(
    locale: Locale,
    secrets: &SharedSecrets,
    id: ProviderId,
) -> Result<(), AiFailure> {
    if id == ProviderId::Chatgpt {
        return Ok(());
    }
    let account = account(id);
    secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.delete(&account)
    })
    .await
    .map_err(|e| keychain_failure(locale, e))
}

/// Models `id` offers, with its stored key; OpenCode Go's list needs none and
/// ChatGPT's uses the signed-in account (slugs, in the plan's order).
pub async fn models(
    locale: Locale,
    client: &AiClient,
    secrets: &SharedSecrets,
    id: ProviderId,
) -> Result<Vec<String>, AiFailure> {
    let key = match id {
        ProviderId::OpencodeGo | ProviderId::Chatgpt => None,
        _ => Some(
            load_key(locale, secrets, id)
                .await?
                .ok_or_else(|| provider_failure(locale, id, &ProviderError::NoKey))?,
        ),
    };
    client
        .list_models(id, key.as_ref())
        .await
        .map_err(|e| provider_failure(locale, id, &e))
}

/// The ChatGPT access token of the signed-in account (renewed when it is
/// about to expire), or why there is none.
async fn chatgpt_token(client: &AiClient) -> Result<ApiKey, ProviderError> {
    let auth = client.chatgpt().ok_or(AuthError::NotSignedIn)?;
    Ok(auth.access_token().await?)
}

/// Checks `id`'s stored key with `model` (empty = the default model). For
/// ChatGPT, checks the sign-in (renewing its token if needed) without any
/// inference request.
pub async fn test_connection(
    locale: Locale,
    client: &AiClient,
    secrets: &SharedSecrets,
    id: ProviderId,
    model: &str,
) -> Result<(), AiFailure> {
    if id == ProviderId::Chatgpt {
        return chatgpt_token(client)
            .await
            .map(|_| ())
            .map_err(|e| provider_failure(locale, id, &e));
    }
    let key = load_key(locale, secrets, id)
        .await?
        .ok_or_else(|| provider_failure(locale, id, &ProviderError::NoKey))?;
    let mut cfg = ProviderConfig::new(id, key);
    if !model.trim().is_empty() {
        cfg.model = model.trim().to_owned();
    }
    client
        .test_connection(&cfg)
        .await
        .map_err(|e| provider_failure(locale, id, &e))
}

/// A Sign in with ChatGPT failure as a command failure: its message in
/// `locale` (never a credential) with a kind the settings can tell apart.
pub fn auth_failure(locale: Locale, err: AuthError) -> AiFailure {
    let kind = match err {
        AuthError::Cancelled => "cancelled",
        AuthError::Timeout => "timeout",
        AuthError::AccessDenied => "denied",
        AuthError::KeysUnavailable | AuthError::Unavailable { .. } | AuthError::Network { .. } => {
            "network"
        }
        _ => "config",
    };
    AiFailure::new(kind, messages::auth_error(locale, &err))
}

/// Signed out, as shown when there is no sign-in or nowhere to keep one.
const SIGNED_OUT: Status = Status {
    signed_in: false,
    email: None,
    plan_enabled: false,
};

/// Whether ChatGPT is signed in, with which account and whether plan use is
/// enabled; signed out when the app has no place to keep a sign-in.
pub async fn chatgpt_status(locale: Locale, client: &AiClient) -> Result<Status, AiFailure> {
    let Some(auth) = client.chatgpt() else {
        return Ok(SIGNED_OUT);
    };
    match auth.status().await {
        Err(AuthError::NoStorage) => Ok(SIGNED_OUT),
        other => other.map_err(|e| auth_failure(locale, e)),
    }
}

/// Signs in with ChatGPT: `open` shows the authorization URL in the system
/// browser; resolves once the browser comes back, is cancelled or times out.
pub async fn chatgpt_sign_in<F>(
    locale: Locale,
    client: &AiClient,
    open: F,
) -> Result<Status, AiFailure>
where
    F: Fn(&str) -> Result<(), String> + Send + Sync,
{
    let auth = client
        .chatgpt()
        .ok_or_else(|| auth_failure(locale, AuthError::NoStorage))?;
    auth.sign_in(open)
        .await
        .map_err(|e| auth_failure(locale, e))
}

/// Ends a pending ChatGPT sign-in, if any.
pub fn chatgpt_cancel(client: &AiClient) {
    if let Some(auth) = client.chatgpt() {
        auth.cancel();
    }
}

/// Revokes the ChatGPT session and forgets its tokens (even when OpenAI could
/// not confirm the revocation: `revoked` is then false).
pub async fn chatgpt_sign_out(locale: Locale, client: &AiClient) -> Result<SignOut, AiFailure> {
    let auth = client
        .chatgpt()
        .ok_or_else(|| auth_failure(locale, AuthError::NoStorage))?;
    auth.sign_out().await.map_err(|e| auth_failure(locale, e))
}

/// A listed model: `id` is stored and sent, `name` is shown.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelName {
    pub id: String,
    pub name: String,
}

/// The ChatGPT plan's models with their display names, read with the
/// signed-in account (the other providers keep `ai_models`).
pub async fn chatgpt_models(
    locale: Locale,
    client: &AiClient,
) -> Result<Vec<ModelName>, AiFailure> {
    let id = ProviderId::Chatgpt;
    let listed = client
        .chatgpt_model_names()
        .await
        .map_err(|e| provider_failure(locale, id, &e))?;
    Ok(listed
        .into_iter()
        .map(|(id, name)| ModelName { id, name })
        .collect())
}

/// Per-provider usage on `day` and in total.
pub fn usage(store: &Store, day: &str) -> Result<Vec<UsageRow>, String> {
    store.usage_totals(day).map_err(|e| e.to_string())
}

/// What the frontend asks for (everything owned: the future outlives the IPC args).
#[derive(Clone, Debug)]
pub struct TextParams {
    pub language: String,
    pub layout: String,
    pub seed: u64,
    pub target_count: usize,
    pub length: usize,
    pub fallback: Vec<String>,
    /// The user's local day, `YYYY-MM-DD`, for the usage counters.
    pub day: String,
    /// Prose or code of one language.
    pub mode: PracticeMode,
    /// Longest code snippet in lines (code only), clamped to 5..=20.
    pub max_lines: usize,
    /// Language of the notices (why the AI was not used).
    pub locale: Locale,
}

/// `day` when it looks like `YYYY-MM-DD`, else today in UTC.
pub fn usage_day(day: &str) -> String {
    let b = day.as_bytes();
    let shape = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        });
    if shape {
        return day.to_owned();
    }
    let days = std::time::UNIX_EPOCH
        .elapsed()
        .unwrap_or(Duration::ZERO)
        .as_secs()
        / 86_400;
    civil_date(days as i64)
}

/// `YYYY-MM-DD` of `days` since 1970-01-01 (Howard Hinnant's civil_from_days).
fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_ms() -> i64 {
    std::time::UNIX_EPOCH
        .elapsed()
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Configs of the chain entries that have a model and a key (for ChatGPT, a
/// signed-in account with plan use), and the reason for the first unusable entry.
async fn usable_chain(
    locale: Locale,
    client: &AiClient,
    secrets: &SharedSecrets,
    settings: &AiSettings,
) -> (Vec<ProviderConfig>, Option<String>) {
    let mut configs = Vec::new();
    let mut reason = None;
    for entry in &settings.chain {
        let id = entry.provider;
        if entry.model.is_empty() {
            reason.get_or_insert_with(|| {
                messages::MISSING_MODEL.fill(locale, &[("name", short_name(id))])
            });
            continue;
        }
        if id == ProviderId::Chatgpt {
            match chatgpt_token(client).await {
                Ok(token) => {
                    let mut cfg = ProviderConfig::new(id, token);
                    cfg.model = entry.model.clone();
                    configs.push(cfg);
                }
                Err(err) => {
                    reason.get_or_insert_with(|| provider_reason(locale, id, &err));
                }
            }
            continue;
        }
        match load_key(locale, secrets, id).await {
            Ok(Some(key)) => {
                let mut cfg = ProviderConfig::new(id, key);
                cfg.model = entry.model.clone();
                configs.push(cfg);
            }
            Ok(None) => {
                reason.get_or_insert_with(|| provider_reason(locale, id, &ProviderError::NoKey));
            }
            Err(_) => {
                reason.get_or_insert_with(|| locale.pick(messages::KEYCHAIN_SHORT).to_owned());
            }
        }
    }
    (configs, reason)
}

/// What the next text practises, by practice mode.
enum Plan {
    Prose(commands::TextPlan),
    Code(code::CodePlan),
}

impl Plan {
    fn targets(&self) -> &[String] {
        match self {
            Plan::Prose(plan) => &plan.targets,
            Plan::Code(plan) => &plan.targets,
        }
    }

    fn corpus(&self, seed: u64) -> Result<NextText, String> {
        match self {
            Plan::Prose(plan) => commands::corpus_text(plan, seed),
            Plan::Code(plan) => code::corpus_code(plan, seed),
        }
    }

    /// The generation request: only targets (with their natural frequency in
    /// the prose or code corpus, 0 when it has none), language, length,
    /// allowed chars and topic (S5).
    fn request(
        &self,
        language: &str,
        settings: &AiSettings,
        restrict_chars: bool,
    ) -> Result<GenerationRequest, String> {
        let targets = self.targets();
        let (mode, length, allowed_chars, frequencies) = match self {
            Plan::Prose(plan) => (
                Mode::Prose,
                // A wider margin than the corpus range: AI texts often run long (S5).
                commands::ai_length(plan.length.start),
                plan.allowed.iter().filter(|c| **c != '\n').collect(),
                targets.iter().map(|t| plan.corpus.frequency(t)).collect(),
            ),
            Plan::Code(plan) => (
                Mode::Code {
                    lang: code::language(&plan.lang)?.name.to_owned(),
                },
                code::code_length(plan.max_lines),
                code::code_allowed_chars(),
                targets
                    .iter()
                    .map(|t| plexustyping_corpus::code_frequency(&plan.lang, t).unwrap_or(0.0))
                    .collect(),
            ),
        };
        Ok(GenerationRequest {
            language: language.to_owned(),
            mode,
            targets: targets.to_vec(),
            frequencies,
            density: settings.density,
            length,
            allowed_chars,
            restrict_chars: restrict_chars && matches!(self, Plan::Prose(_)),
            topic: settings.topic.clone(),
            continue_from: None,
        })
    }

    /// A generated or cached text with its syntax tokens in code mode.
    fn finish(&self, text: NextText) -> NextText {
        match self {
            Plan::Prose(_) => text,
            Plan::Code(plan) => code::with_syntax(text, code::default_syntax(&plan.lang)),
        }
    }
}

/// The next practice text: AI when enabled and usable, else a cached AI
/// text, else the corpus; `notice` says why the AI was not used. With the AI
/// disabled, a rotating cached text (no notice), else the corpus. Code texts
/// use the code model, are cached per language and come with syntax tokens.
/// Words texts always come from the corpus with the prose model: no AI
/// request and no cache, even with the AI enabled.
pub async fn next_text(state: &AppState, p: &TextParams) -> Result<NextText, String> {
    let (model, settings, restriction) = {
        let store = lock_store(&state.store);
        let profile = state.active_profile();
        let restriction = match p.mode {
            PracticeMode::Prose => unlock::restriction(&store, &profile, &p.language, &p.layout)?,
            _ => None,
        };
        (
            commands::load_model_for(&store, &profile, p.mode.stats_mode())?,
            load_settings(&store),
            restriction,
        )
    };
    let prose_request = || NextTextRequest {
        language: &p.language,
        layout: &p.layout,
        seed: p.seed,
        target_count: p.target_count,
        length: p.length,
        fallback: &p.fallback,
    };
    let plan = match &p.mode {
        PracticeMode::Words => {
            let plan = commands::plan_text(&model, &prose_request())?;
            return commands::words_text(&plan, p.seed);
        }
        PracticeMode::Prose => {
            let mut plan = commands::plan_text(&model, &prose_request())?;
            // Progressive unlock (S5): only the active characters; below 15
            // letters, words and pseudowords, never the AI or the cache.
            if let Some(r) = &restriction {
                unlock::restrict_plan(&mut plan, r);
                if r.letters.len() < unlock::AI_MIN_LETTERS {
                    return unlock::letters_text(&plan, r, &model, p.seed);
                }
            }
            Plan::Prose(plan)
        }
        PracticeMode::Code { lang } => Plan::Code(code::plan_code(
            &model,
            lang,
            p.target_count,
            p.max_lines,
            p.seed,
            &p.fallback,
        )?),
    };
    let cache_mode = p.mode.cache_mode();
    // Cached texts may use locked characters: none while some letter is locked.
    let cache_ok = restriction.as_ref().is_none_or(|r| r.all_letters);
    if !settings.enabled {
        // AI off: a saved text, rotating so consecutive texts differ, else the corpus.
        if !cache_ok {
            return plan.corpus(p.seed);
        }
        let cached = {
            let store = lock_store(&state.store);
            store.take_cached_text(&p.language, &cache_mode, plan.targets())
        };
        match cached {
            Ok(Some(c)) => {
                return Ok(plan.finish(NextText {
                    source: TextSource::Cache,
                    provider: Some(c.provider),
                    model: Some(c.model),
                    ..NextText::corpus(c.text, c.targets)
                }));
            }
            Ok(None) => {}
            Err(e) => eprintln!("PlexusTyping: could not read the text cache: {e}"),
        }
        return plan.corpus(p.seed);
    }
    // Unlock from 15 letters: the prompt names the active characters (S5).
    let restricted = restriction.as_ref().is_some_and(|r| !r.all_letters);
    let request = plan.request(&p.language, &settings, restricted)?;
    let ai = ai_text(
        p.locale,
        state,
        &plan,
        &settings,
        &request,
        &p.day,
        Some(&cache_mode),
    );
    let reason = match ai.await {
        Ok(text) => return Ok(text),
        Err(reason) => reason,
    };
    let cached = if cache_ok {
        let store = lock_store(&state.store);
        // Never a text already shown: the corpus gives a new one instead.
        store.take_unused_cached_text(&p.language, &cache_mode, plan.targets())
    } else {
        Ok(None)
    };
    match cached {
        Ok(Some(c)) => {
            return Ok(plan.finish(NextText {
                source: TextSource::Cache,
                provider: Some(c.provider),
                model: Some(c.model),
                notice: Some(messages::FROM_CACHE.fill(p.locale, &[("reason", &reason)])),
                ..NextText::corpus(c.text, c.targets)
            }));
        }
        Ok(None) => {}
        Err(e) => eprintln!("PlexusTyping: could not read the text cache: {e}"),
    }
    let mut text = plan.corpus(p.seed)?;
    text.notice = Some(messages::FROM_CORPUS.fill(p.locale, &[("reason", &reason)]));
    Ok(text)
}

/// Length in chars of each continuation of an unlimited text (S2).
pub const CONTINUE_LENGTH: usize = 600;
/// Longest `tail` a continuation accepts, in chars (S4).
pub const MAX_TAIL_CHARS: usize = 2000;

/// What an unlimited text continues with: AI prose or corpus words (S4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContinueMode {
    Prose,
    Words,
}

/// What the frontend sends to continue an unlimited text (owned, like [`TextParams`]).
#[derive(Clone, Debug)]
pub struct ContinueParams {
    pub language: String,
    pub layout: String,
    pub mode: ContinueMode,
    /// The end of the current text (generated, never typed), at most [`MAX_TAIL_CHARS`].
    pub tail: String,
    pub target_count: usize,
    pub seed: u64,
    /// The user's local day, `YYYY-MM-DD`, for the usage counters.
    pub day: String,
    /// Skip the AI: the frontend asks so after a failed or slow continuation, chaining corpus texts.
    pub offline: bool,
    /// Language of the notices and errors.
    pub locale: Locale,
}

/// An AI text for `request` through the usable chain of `settings`, with its
/// usage counted and, when `cache_mode` is given, cached as already used; or
/// the reason (in `locale`) the AI could not give one.
async fn ai_text(
    locale: Locale,
    state: &AppState,
    plan: &Plan,
    settings: &AiSettings,
    request: &GenerationRequest,
    day: &str,
    cache_mode: Option<&str>,
) -> Result<NextText, String> {
    let (chain, missing) = usable_chain(locale, &state.ai, &state.secrets, settings).await;
    if chain.is_empty() {
        return Err(missing.unwrap_or_else(|| locale.pick(messages::CHAIN_EMPTY).to_owned()));
    }
    let generated = match state.ai.generate_with_fallback(&chain, request).await {
        Ok(generated) => generated,
        Err(errors) => {
            for line in rejection_lines(&errors) {
                eprintln!("{line}");
            }
            return Err(errors
                .first()
                .map(|(id, err)| provider_reason(locale, *id, err))
                .unwrap_or_else(|| locale.pick(messages::NO_ANSWER).to_owned()));
        }
    };
    let provider = slug(generated.provider);
    // A text from the retry without targets practises none of them; a prose
    // text (whose prompt names no targets, S9) only those it really contains.
    let lower = generated.text.to_lowercase();
    let present: Vec<String>;
    let targets: &[String] = if generated.free_text {
        &[]
    } else if matches!(plan, Plan::Prose(_)) {
        present = plan
            .targets()
            .iter()
            .filter(|t| lower.contains(&t.to_lowercase()))
            .cloned()
            .collect();
        &present
    } else {
        plan.targets()
    };
    let store = lock_store(&state.store);
    let cached = cache_mode
        .map(|mode| {
            store.cache_text(&NewCachedText {
                provider,
                model: &generated.model,
                language: &request.language,
                mode,
                targets,
                text: &generated.text,
                created_at_ms: now_ms(),
                used: true,
            })
        })
        .transpose();
    let usage = &generated.usage;
    let counted = store.record_usage(
        provider,
        &usage_day(day),
        usage.input_tokens,
        usage.output_tokens,
    );
    if let Err(e) = cached.and(counted) {
        eprintln!("PlexusTyping: could not store the AI text or its usage: {e}");
    }
    // Earlier rejected attempts were billed too: log why, as when all fail.
    for line in rejection_lines(&generated.failed_attempts) {
        eprintln!("{line}");
    }
    Ok(plan.finish(NextText {
        source: TextSource::Ai,
        provider: Some(provider.to_owned()),
        model: Some(generated.model),
        ..NextText::corpus(generated.text, targets.to_vec())
    }))
}

/// The next part of an unlimited text (S2, S4), about [`CONTINUE_LENGTH`]
/// chars. Prose: the AI continues `tail` with the targets, allowed chars and
/// progressive unlock of [`next_text`]; without the AI, a corpus text (with
/// the reason as a notice when the AI is enabled but not usable). Words: more
/// corpus words, never the AI. Continuations are never cached nor taken from
/// the cache; usage is counted as for [`next_text`].
pub async fn continue_text(state: &AppState, p: &ContinueParams) -> Result<NextText, String> {
    if p.tail.chars().count() > MAX_TAIL_CHARS {
        let max = MAX_TAIL_CHARS.to_string();
        return Err(messages::TAIL_TOO_LONG.fill(p.locale, &[("max", &max)]));
    }
    let (model, settings, restriction) = {
        let store = lock_store(&state.store);
        let profile = state.active_profile();
        let restriction = match p.mode {
            ContinueMode::Prose => unlock::restriction(&store, &profile, &p.language, &p.layout)?,
            ContinueMode::Words => None,
        };
        (
            commands::load_model(&store, &profile)?,
            load_settings(&store),
            restriction,
        )
    };
    let request = NextTextRequest {
        language: &p.language,
        layout: &p.layout,
        seed: p.seed,
        target_count: p.target_count,
        length: CONTINUE_LENGTH,
        fallback: &[],
    };
    let mut plan = commands::plan_text(&model, &request)?;
    if p.mode == ContinueMode::Words {
        return commands::words_text(&plan, p.seed);
    }
    if let Some(r) = &restriction {
        unlock::restrict_plan(&mut plan, r);
        if r.letters.len() < unlock::AI_MIN_LETTERS {
            return unlock::letters_text(&plan, r, &model, p.seed);
        }
    }
    let plan = Plan::Prose(plan);
    if !settings.enabled || p.offline {
        return plan.corpus(p.seed);
    }
    let restricted = restriction.as_ref().is_some_and(|r| !r.all_letters);
    let request = GenerationRequest {
        continue_from: Some(p.tail.clone()),
        ..plan.request(&p.language, &settings, restricted)?
    };
    match ai_text(p.locale, state, &plan, &settings, &request, &p.day, None).await {
        Ok(text) => Ok(text),
        Err(reason) => {
            let mut text = plan.corpus(p.seed)?;
            text.notice = Some(messages::FROM_CORPUS.fill(p.locale, &[("reason", &reason)]));
            Ok(text)
        }
    }
}

/// The diagnostic text for a new profile practising prose on a `layout` that
/// can type it, otherwise like [`next_text`] with the default target count and
/// length. Words mode never shows the diagnostic (it has capitals and signs).
#[allow(
    clippy::too_many_arguments,
    reason = "the command's arguments plus the interface language"
)]
pub async fn first_text(
    locale: Locale,
    state: &AppState,
    language: &str,
    layout: &str,
    mode: &PracticeMode,
    max_lines: usize,
    seed: u64,
    day: &str,
) -> Result<FirstText, String> {
    if *mode == PracticeMode::Prose {
        let diagnostic = {
            let store = lock_store(&state.store);
            let profile = state.active_profile();
            // Progressive unlock starts from the home row: no diagnostic text.
            if unlock::is_enabled(&store, profile.id) {
                None
            } else {
                commands::diagnostic_for_new_profile(&store, &profile, language, layout)?
            }
        };
        if let Some(first) = diagnostic {
            return Ok(first);
        }
    }
    let params = TextParams {
        language: language.to_owned(),
        layout: layout.to_owned(),
        seed,
        target_count: DEFAULT_TARGETS,
        length: DEFAULT_LENGTH,
        fallback: Vec::new(),
        day: day.to_owned(),
        mode: mode.clone(),
        max_lines,
        locale,
    };
    Ok(FirstText {
        text: next_text(state, &params).await?,
        diagnostic: false,
    })
}

#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;
