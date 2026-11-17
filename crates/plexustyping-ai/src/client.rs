//! The one place that sends HTTP requests.

use std::time::{Duration, Instant};

use crate::opencode::{Catalog, MAX_CATALOGUE_BYTES, ModelInfo, models_from_models_dev};
use crate::providers::{build, list_request, new_session_id, parse, parse_models};
use crate::{
    ApiKey, DEFAULT_DEADLINE, DEFAULT_TIMEOUT, Endpoints, Generated, GenerationRequest,
    ProviderConfig, ProviderError, ProviderId, Usage, build_prompt, parse_text_json, validate,
};

/// The common provider interface (PRD RF2 `TextProvider`), implemented by
/// [`AiClient`] for every [`ProviderId`]; lets callers swap in a fake.
#[async_trait::async_trait]
pub trait TextProvider: Send + Sync {
    /// See [`AiClient::generate`].
    async fn generate(
        &self,
        cfg: &ProviderConfig,
        req: &GenerationRequest,
    ) -> Result<Generated, ProviderError>;
    /// See [`AiClient::list_models`].
    async fn list_models(
        &self,
        id: ProviderId,
        key: Option<&ApiKey>,
    ) -> Result<Vec<String>, ProviderError>;
    /// See [`AiClient::test_connection`].
    async fn test_connection(&self, cfg: &ProviderConfig) -> Result<(), ProviderError>;
}

/// How PlexusTyping identifies itself to every provider.
const USER_AGENT: &str = concat!("PlexusTyping/", env!("CARGO_PKG_VERSION"));
/// Provider replies are small; anything far larger is refused.
const MAX_REPLY_BYTES: usize = 4 * 1024 * 1024;
/// The cheapest generation, used to check an OpenCode Go key.
const PING_PROMPT: &str = "Responde solo con JSON: {\"text\": \"ok\"}";

/// HTTP client for the four providers: HTTPS only, redirects off (keys are
/// never re-sent to another host), 5 s connect timeout, fixed [`Endpoints`],
/// an overall deadline per fallback chain, and the in-memory OpenCode Go
/// format catalogue.
pub struct AiClient {
    http: Result<reqwest::Client, String>,
    endpoints: Endpoints,
    catalog: Catalog,
    /// OpenCode Go session id used when a config has none.
    session: String,
    /// Total time for one [`AiClient::generate_with_fallback`] call.
    deadline: Duration,
}

impl AiClient {
    /// Client for `endpoints` (use [`Endpoints::production`]).
    pub fn new(endpoints: Endpoints) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .https_only(!endpoints.allow_http)
            .build()
            .map_err(|err| err.to_string());
        AiClient {
            http,
            endpoints,
            catalog: Catalog::default(),
            session: new_session_id(),
            deadline: DEFAULT_DEADLINE,
        }
    }

    /// The same client with `deadline` as the total time of one
    /// [`AiClient::generate_with_fallback`] call.
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Total time of one [`AiClient::generate_with_fallback`] call
    /// ([`DEFAULT_DEADLINE`] unless set with [`AiClient::with_deadline`]).
    pub fn deadline(&self) -> Duration {
        self.deadline
    }

    /// Builds the prompt from `req` (only its fields are sent), calls the
    /// provider once, parses the `{"text": ...}` answer and validates it:
    /// [`ProviderError::Invalid`] when validation fails. The returned text is
    /// the normalized one.
    pub async fn generate(
        &self,
        cfg: &ProviderConfig,
        req: &GenerationRequest,
    ) -> Result<Generated, ProviderError> {
        let (raw, usage) = self.complete(cfg, &build_prompt(req), true).await?;
        let text = parse_text_json(&raw)?;
        let text = validate(&text, req).map_err(|issues| ProviderError::Invalid { issues })?;
        Ok(Generated {
            text,
            provider: cfg.id,
            model: cfg.model.trim().to_string(),
            usage,
            free_text: false,
            failed_attempts: Vec::new(),
        })
    }

    /// Model ids the provider offers, filtered to text models. Claude, OpenAI
    /// and Gemini need `key` ([`ProviderError::NoKey`] without it); OpenCode
    /// Go's list is public and is fetched without the key.
    pub async fn list_models(
        &self,
        id: ProviderId,
        key: Option<&ApiKey>,
    ) -> Result<Vec<String>, ProviderError> {
        let mask = |err: ProviderError| match key {
            Some(key) => err.masked(key.expose()),
            None => err,
        };
        let (url, headers) = list_request(id, &self.endpoints, key)?;
        let mut request = self.http()?.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let (status, body) = fetch(request, DEFAULT_TIMEOUT, id, MAX_REPLY_BYTES)
            .await
            .map_err(mask)?;
        parse_models(id, status, &body).map_err(mask)
    }

    /// Checks the key with the cheapest authenticated call: the model list for
    /// Claude, OpenAI and Gemini; a tiny generation with `cfg.model` for
    /// OpenCode Go, whose model list is public.
    pub async fn test_connection(&self, cfg: &ProviderConfig) -> Result<(), ProviderError> {
        match cfg.id {
            ProviderId::OpencodeGo => self.complete(cfg, PING_PROMPT, false).await.map(|_| ()),
            id => self.list_models(id, Some(&cfg.api_key)).await.map(|_| ()),
        }
    }

    /// Tries `chain` in order and returns the first valid text. A provider
    /// gets one retry when its text fails validation
    /// ([`ProviderError::Invalid`]) or the reply is malformed
    /// ([`ProviderError::BadResponse`]); any other error moves to the next
    /// one. The whole call ends within [`AiClient::deadline`]: the attempt
    /// running when it expires fails with [`ProviderError::Timeout`], and so
    /// does every remaining provider, without a request. When all fail, every
    /// attempt's error is returned in order (empty for an empty chain);
    /// falling back to the local corpus is the caller's job.
    pub async fn generate_with_fallback(
        &self,
        chain: &[ProviderConfig],
        req: &GenerationRequest,
    ) -> Result<Generated, Vec<(ProviderId, ProviderError)>> {
        let end = Instant::now() + self.deadline;
        let mut errors = Vec::new();
        // The retry after a rejected or malformed text asks for any text: no targets.
        let free = GenerationRequest {
            targets: Vec::new(),
            frequencies: Vec::new(),
            ..req.clone()
        };
        for cfg in chain {
            for attempt in 0..2 {
                let asked = if attempt == 0 { req } else { &free };
                let left = end.saturating_duration_since(Instant::now());
                let result = if left.is_zero() {
                    Err(ProviderError::Timeout)
                } else {
                    tokio::time::timeout(left, self.generate(cfg, asked))
                        .await
                        .unwrap_or(Err(ProviderError::Timeout))
                };
                match result {
                    Ok(generated) => {
                        return Ok(Generated {
                            free_text: attempt == 1 && !req.targets.is_empty(),
                            failed_attempts: errors,
                            ..generated
                        });
                    }
                    Err(err) => {
                        let retry = attempt == 0
                            && matches!(
                                err,
                                ProviderError::Invalid { .. } | ProviderError::BadResponse { .. }
                            );
                        errors.push((cfg.id, err));
                        if !retry {
                            break;
                        }
                    }
                }
            }
        }
        Err(errors)
    }

    fn http(&self) -> Result<&reqwest::Client, ProviderError> {
        self.http
            .as_ref()
            .map_err(|err| ProviderError::config(format!("The HTTP client could not start: {err}")))
    }

    /// Sends `prompt` and returns the model's raw text with the usage. With
    /// `generation`, an OpenCode Go reasoning model is asked for its lowest
    /// effort (the connection test sends the plain request). Every error is
    /// masked with the key.
    async fn complete(
        &self,
        cfg: &ProviderConfig,
        prompt: &str,
        generation: bool,
    ) -> Result<(String, Usage), ProviderError> {
        let key = cfg.api_key.expose();
        let mut format = ModelInfo::default();
        let mut with_session;
        let mut cfg = cfg;
        if cfg.id == ProviderId::OpencodeGo && !cfg.model.trim().is_empty() {
            format = self.opencode_format(cfg).await;
            if cfg.session_id.is_none() {
                with_session = cfg.clone();
                with_session.session_id = Some(self.session.clone());
                cfg = &with_session;
            }
        }
        let low_effort = format.low_effort.filter(|_| generation);
        let call = build(cfg, &self.endpoints, prompt, format.wire, low_effort)
            .map_err(|e| e.masked(key))?;
        let mut request = self.http()?.post(&call.url).json(&call.body);
        for (name, value) in &call.headers {
            request = request.header(*name, value);
        }
        let (status, body) = fetch(request, cfg.timeout, cfg.id, MAX_REPLY_BYTES)
            .await
            .map_err(|e| e.masked(key))?;
        parse(call.shape, status, &body).map_err(|e| e.masked(key))
    }

    /// The OpenCode Go API format and lowest effort of `cfg.model`, fetching
    /// models.dev when the cache is empty or stale; chat completions without an
    /// effort when it cannot be read.
    async fn opencode_format(&self, cfg: &ProviderConfig) -> ModelInfo {
        let model = cfg.model.trim();
        if let Some(info) = self.catalog.lookup(model, Instant::now()) {
            return info;
        }
        let models = match self.http() {
            Ok(http) => {
                let request = http.get(&self.endpoints.models_dev);
                match fetch(request, cfg.timeout, cfg.id, MAX_CATALOGUE_BYTES).await {
                    Ok((200..=299, body)) => models_from_models_dev(body.as_bytes()),
                    _ => None,
                }
            }
            Err(_) => None,
        };
        self.catalog.store(models, model, Instant::now())
    }
}

/// Sends `request` with `timeout` and reads at most `max` bytes of body.
async fn fetch(
    request: reqwest::RequestBuilder,
    timeout: Duration,
    provider: ProviderId,
    max: usize,
) -> Result<(u16, String), ProviderError> {
    let mut response = request
        .timeout(timeout)
        .send()
        .await
        .map_err(|err| transport_error(provider, err))?;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| transport_error(provider, err))?
    {
        if body.len() + chunk.len() > max {
            return Err(ProviderError::bad_response(format!(
                "The answer from {} was too large.",
                provider.label()
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok((status, String::from_utf8_lossy(&body).into_owned()))
}

/// Timeout, or a network error whose text never includes the URL.
fn transport_error(provider: ProviderId, err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        return ProviderError::Timeout;
    }
    let err = err.without_url();
    let mut detail = err.to_string();
    let mut source = std::error::Error::source(&err);
    while let Some(cause) = source {
        detail.push_str(": ");
        detail.push_str(&cause.to_string());
        source = cause.source();
    }
    ProviderError::Network {
        message: format!("Could not reach {}: {detail}.", provider.label()),
    }
}

#[async_trait::async_trait]
impl TextProvider for AiClient {
    async fn generate(
        &self,
        cfg: &ProviderConfig,
        req: &GenerationRequest,
    ) -> Result<Generated, ProviderError> {
        AiClient::generate(self, cfg, req).await
    }

    async fn list_models(
        &self,
        id: ProviderId,
        key: Option<&ApiKey>,
    ) -> Result<Vec<String>, ProviderError> {
        AiClient::list_models(self, id, key).await
    }

    async fn test_connection(&self, cfg: &ProviderConfig) -> Result<(), ProviderError> {
        AiClient::test_connection(self, cfg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenerationRequest, Mode};

    #[tokio::test]
    async fn production_client_refuses_plain_http_before_connecting() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let endpoints = Endpoints {
            openai: format!("{base}/v1"),
            ..Endpoints::production()
        };
        let mut cfg = ProviderConfig::new(ProviderId::Openai, ApiKey::new("sk-x").unwrap());
        cfg.timeout = Duration::from_secs(2);
        let req = GenerationRequest {
            language: "es".into(),
            mode: Mode::Prose,
            targets: vec!["z".into()],
            frequencies: Vec::new(),
            density: 3.0,
            length: 5..60,
            allowed_chars: "abcz".into(),
            topic: None,
        };

        let err = AiClient::new(endpoints)
            .generate(&cfg, &req)
            .await
            .unwrap_err();

        assert!(matches!(err, ProviderError::Network { .. }), "{err:?}");
        let accepted = tokio::time::timeout(Duration::from_millis(200), listener.accept()).await;
        assert!(accepted.is_err(), "no connection may reach the http host");
    }

    #[test]
    fn the_default_deadline_bounds_the_whole_chain() {
        assert_eq!(DEFAULT_DEADLINE, Duration::from_secs(60));
        let client = AiClient::new(Endpoints::production());
        assert_eq!(client.deadline(), DEFAULT_DEADLINE);
        let short = client.with_deadline(Duration::from_millis(5));
        assert_eq!(short.deadline(), Duration::from_millis(5));
    }
}
