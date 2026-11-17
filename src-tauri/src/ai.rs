//! AI practice texts wired into the app (S2, S3, S5): non-secret settings in
//! the store, keys in the system keychain, generation through the provider
//! fallback chain, then the local cache, then the corpus.
//!
//! Only target units, language, length, allowed chars and topic are sent
//! (S5). Keys never cross back to the frontend and are never logged. The store
//! `Mutex` is never held across an `.await`: lock, copy, drop, await, lock again.

use std::time::Duration;

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
use crate::secrets::{self, SecretError, SharedSecrets};

/// Store `settings` key of the JSON [`AiSettings`].
pub const SETTINGS_KEY: &str = "ai";
/// Longest fallback chain.
pub const MAX_CHAIN: usize = 4;
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
    /// 1–4 distinct providers; order = fallback order.
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

/// A failed AI command, serialized as `{ kind, message }` (message in Spanish).
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
const KEYCHAIN_UNAVAILABLE: &str =
    "Llavero no disponible: las claves nunca se guardan en texto plano";

fn keychain_failure(err: SecretError) -> AiFailure {
    match err {
        SecretError::Unavailable => AiFailure::new("keychain", KEYCHAIN_UNAVAILABLE),
        SecretError::Keyring { message } => AiFailure::new(
            "keychain",
            format!("Error del llavero del sistema: {message}"),
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
    }
}

/// Kebab-case id (the serde name), e.g. `opencode-go`.
pub fn slug(id: ProviderId) -> &'static str {
    match id {
        ProviderId::Claude => "claude",
        ProviderId::Openai => "openai",
        ProviderId::Gemini => "gemini",
        ProviderId::OpencodeGo => "opencode-go",
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

/// The one place that turns a provider error into a short Spanish reason.
pub fn provider_reason(id: ProviderId, err: &ProviderError) -> String {
    let name = short_name(id);
    match err {
        ProviderError::NoKey => format!("Falta la clave de {name}"),
        ProviderError::Auth { .. } => format!("{name} rechazó la clave"),
        ProviderError::RateLimit { .. } => format!("{name} está limitando las peticiones"),
        ProviderError::Network { .. } => "Sin conexión con la IA".to_owned(),
        ProviderError::Timeout => format!("{name} no respondió a tiempo"),
        ProviderError::Refused { .. } => format!("{name} se negó a generar el texto"),
        ProviderError::BadResponse { .. } | ProviderError::Invalid { .. } => {
            "La IA no devolvió un texto válido".to_owned()
        }
        ProviderError::Config { .. } => {
            format!("{name} rechazó la petición (revisa el modelo)")
        }
    }
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
fn provider_failure(id: ProviderId, err: &ProviderError) -> AiFailure {
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
    AiFailure::new(kind, provider_reason(id, err))
}

/// Normalized `s` (trimmed model and topic, empty topic = none) or why it is invalid.
pub fn validate_settings(mut s: AiSettings) -> Result<AiSettings, AiFailure> {
    if s.chain.is_empty() || s.chain.len() > MAX_CHAIN {
        return Err(AiFailure::settings(format!(
            "La cadena de respaldo necesita de 1 a {MAX_CHAIN} proveedores"
        )));
    }
    for i in 0..s.chain.len() {
        let model = s.chain[i].model.trim().to_owned();
        if model.chars().count() > MAX_MODEL_CHARS {
            return Err(AiFailure::settings(
                "El nombre del modelo es demasiado largo",
            ));
        }
        s.chain[i].model = model;
        if s.chain[..i]
            .iter()
            .any(|e| e.provider == s.chain[i].provider)
        {
            return Err(AiFailure::settings(format!(
                "{} aparece dos veces en la cadena de respaldo",
                short_name(s.chain[i].provider)
            )));
        }
    }
    if !s.density.is_finite() || !DENSITY_RANGE.contains(&s.density) {
        return Err(AiFailure::settings("La densidad debe estar entre 2× y 5×"));
    }
    s.topic = s
        .topic
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty());
    if let Some(topic) = &s.topic {
        if topic.chars().count() > MAX_TOPIC_CHARS {
            return Err(AiFailure::settings(format!(
                "El tema admite como máximo {MAX_TOPIC_CHARS} caracteres"
            )));
        }
        if topic.chars().any(char::is_control) {
            return Err(AiFailure::settings(
                "El tema no puede tener saltos de línea",
            ));
        }
    }
    Ok(s)
}

/// Stored settings; the defaults when missing or no longer valid.
pub fn load_settings(store: &Store) -> AiSettings {
    let stored = store.get_setting(SETTINGS_KEY).ok().flatten();
    stored
        .and_then(|json| serde_json::from_str(&json).ok())
        .and_then(|s| validate_settings(s).ok())
        .unwrap_or_default()
}

/// Validates and stores `s`; returns what was stored.
pub fn save_settings(store: &Store, s: AiSettings) -> Result<AiSettings, AiFailure> {
    let s = validate_settings(s)?;
    let json = serde_json::to_string(&s).map_err(|e| AiFailure::new("storage", e.to_string()))?;
    store
        .set_setting(SETTINGS_KEY, &json)
        .map_err(|e| AiFailure::new("storage", format!("No se pudo guardar: {e}")))?;
    Ok(s)
}

/// The stored key of `id`, `None` when there is none.
async fn load_key(secrets: &SharedSecrets, id: ProviderId) -> Result<Option<ApiKey>, AiFailure> {
    let account = account(id);
    let stored = secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.get(&account)
    })
    .await
    .map_err(keychain_failure)?;
    Ok(stored.and_then(|k| ApiKey::new(k).ok()))
}

/// Whether a key is saved for `id`; the key itself never leaves the backend.
pub async fn key_status(secrets: &SharedSecrets, id: ProviderId) -> Result<bool, AiFailure> {
    Ok(load_key(secrets, id).await?.is_some())
}

/// Saves `key` (trimmed) for `id` in the system keychain.
pub async fn key_save(
    secrets: &SharedSecrets,
    id: ProviderId,
    key: String,
) -> Result<(), AiFailure> {
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err(AiFailure::new("noKey", "La clave está vacía"));
    }
    let (account, label) = (account(id), key_label(id));
    secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.set(&account, &label, &key)
    })
    .await
    .map_err(keychain_failure)
}

/// Deletes `id`'s key; deleting a missing key succeeds.
pub async fn key_delete(secrets: &SharedSecrets, id: ProviderId) -> Result<(), AiFailure> {
    let account = account(id);
    secrets::run(secrets, move |s| {
        if !s.available() {
            return Err(SecretError::Unavailable);
        }
        s.delete(&account)
    })
    .await
    .map_err(keychain_failure)
}

/// Models `id` offers, with its stored key; OpenCode Go's list needs none.
pub async fn models(
    client: &AiClient,
    secrets: &SharedSecrets,
    id: ProviderId,
) -> Result<Vec<String>, AiFailure> {
    let key = match id {
        ProviderId::OpencodeGo => None,
        _ => Some(
            load_key(secrets, id)
                .await?
                .ok_or_else(|| provider_failure(id, &ProviderError::NoKey))?,
        ),
    };
    client
        .list_models(id, key.as_ref())
        .await
        .map_err(|e| provider_failure(id, &e))
}

/// Checks `id`'s stored key with `model` (empty = the default model).
pub async fn test_connection(
    client: &AiClient,
    secrets: &SharedSecrets,
    id: ProviderId,
    model: &str,
) -> Result<(), AiFailure> {
    let key = load_key(secrets, id)
        .await?
        .ok_or_else(|| provider_failure(id, &ProviderError::NoKey))?;
    let mut cfg = ProviderConfig::new(id, key);
    if !model.trim().is_empty() {
        cfg.model = model.trim().to_owned();
    }
    client
        .test_connection(&cfg)
        .await
        .map_err(|e| provider_failure(id, &e))
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

/// Configs of the chain entries that have a model and a key, and the reason
/// for the first unusable entry.
async fn usable_chain(
    secrets: &SharedSecrets,
    settings: &AiSettings,
) -> (Vec<ProviderConfig>, Option<String>) {
    let mut configs = Vec::new();
    let mut reason = None;
    for entry in &settings.chain {
        let id = entry.provider;
        if entry.model.is_empty() {
            reason.get_or_insert_with(|| format!("Falta el modelo de {}", short_name(id)));
            continue;
        }
        match load_key(secrets, id).await {
            Ok(Some(key)) => {
                let mut cfg = ProviderConfig::new(id, key);
                cfg.model = entry.model.clone();
                configs.push(cfg);
            }
            Ok(None) => {
                reason.get_or_insert_with(|| provider_reason(id, &ProviderError::NoKey));
            }
            Err(_) => {
                reason.get_or_insert_with(|| "Llavero no disponible".to_owned());
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
    fn request(&self, language: &str, settings: &AiSettings) -> Result<GenerationRequest, String> {
        let targets = self.targets();
        let (mode, length, allowed_chars, frequencies) = match self {
            Plan::Prose(plan) => (
                Mode::Prose,
                plan.length.clone(),
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
            topic: settings.topic.clone(),
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
/// text, else the corpus; `notice` says why the AI was not used. Code texts
/// use the code model, are cached per language and come with syntax tokens.
pub async fn next_text(state: &AppState, p: &TextParams) -> Result<NextText, String> {
    let (model, settings) = {
        let store = lock_store(&state.store);
        (
            commands::load_model_for(&store, &state.profile, p.mode.stats_mode())?,
            load_settings(&store),
        )
    };
    let plan = match &p.mode {
        PracticeMode::Prose => {
            let req = NextTextRequest {
                language: &p.language,
                layout: &p.layout,
                seed: p.seed,
                target_count: p.target_count,
                length: p.length,
                fallback: &p.fallback,
            };
            Plan::Prose(commands::plan_text(&model, &req)?)
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
    if !settings.enabled {
        return plan.corpus(p.seed);
    }
    let cache_mode = p.mode.cache_mode();
    let (chain, missing) = usable_chain(&state.secrets, &settings).await;
    let reason = if chain.is_empty() {
        missing.unwrap_or_else(|| "La cadena de respaldo está vacía".to_owned())
    } else {
        let request = plan.request(&p.language, &settings)?;
        match state.ai.generate_with_fallback(&chain, &request).await {
            Ok(generated) => {
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
                let cached = store.cache_text(&NewCachedText {
                    provider,
                    model: &generated.model,
                    language: &p.language,
                    mode: &cache_mode,
                    targets,
                    text: &generated.text,
                    created_at_ms: now_ms(),
                    used: true,
                });
                let usage = &generated.usage;
                let counted = store.record_usage(
                    provider,
                    &usage_day(&p.day),
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
                return Ok(plan.finish(NextText {
                    source: TextSource::Ai,
                    provider: Some(provider.to_owned()),
                    model: Some(generated.model),
                    ..NextText::corpus(generated.text, targets.to_vec())
                }));
            }
            Err(errors) => {
                for line in rejection_lines(&errors) {
                    eprintln!("{line}");
                }
                errors
                    .first()
                    .map(|(id, err)| provider_reason(*id, err))
                    .unwrap_or_else(|| "La IA no respondió".to_owned())
            }
        }
    };
    let cached = {
        let store = lock_store(&state.store);
        // Never a text already shown: the corpus gives a new one instead.
        store.take_unused_cached_text(&p.language, &cache_mode, plan.targets())
    };
    match cached {
        Ok(Some(c)) => {
            return Ok(plan.finish(NextText {
                source: TextSource::Cache,
                provider: Some(c.provider),
                model: Some(c.model),
                notice: Some(format!("{reason}: texto del caché")),
                ..NextText::corpus(c.text, c.targets)
            }));
        }
        Ok(None) => {}
        Err(e) => eprintln!("PlexusTyping: could not read the text cache: {e}"),
    }
    let mut text = plan.corpus(p.seed)?;
    text.notice = Some(format!("{reason}: texto del corpus local"));
    Ok(text)
}

/// The diagnostic text for a new profile practising prose on a `layout` that
/// can type it, otherwise like [`next_text`] with the default target count and length.
pub async fn first_text(
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
            commands::diagnostic_for_new_profile(&store, &state.profile, language, layout)?
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
    };
    Ok(FirstText {
        text: next_text(state, &params).await?,
        diagnostic: false,
    })
}

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
