//! Sign in with ChatGPT: OpenAI's flow for open-source, locally run apps
//! (developers.openai.com/siwc/token-sharing-open-source), so PlexusTyping can
//! use the user's ChatGPT plan without an API key.
//!
//! Ported from PlexusTerm's working `src-tauri/src/ai/chatgpt.rs` (same author),
//! plus the RS256 signature check of the ID token against OpenAI's published
//! JWKS, as compliance with OpenAI's sign-in guide and defence in depth (see
//! `id_token`).
//!
//! Sign-in is an OAuth authorization-code flow with PKCE (S256) in the system
//! browser and a loopback listener on `127.0.0.1` (port 1455, any free port
//! when it is taken). The credential record (tokens included) lives in one
//! private app config file, [`CREDENTIALS_FILE`]; tokens never go back to the
//! frontend, into errors or into `Debug` output. Refreshes, sign-in results and
//! sign-out are serialized on one lock, so a rotating refresh token is never
//! used twice.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aws_lc_rs::digest;
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ApiKey, Endpoints};

mod callback;
mod file;
mod id_token;

use id_token::Verifier;

/// The app name shown on OpenAI's consent screen (first registration only).
pub const AGENT_NAME: &str = "PlexusTyping";
/// The first-registration client id; never saved or used for a token exchange.
pub const REGISTRATION_CLIENT: &str = "dynamic_agent_client";
/// The API the tokens are issued for.
pub const RESOURCE: &str = "https://api.openai.com/v1";
/// Identity scopes plus the ChatGPT plan usage scopes.
pub const SCOPE: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
/// The granted scope that allows inference on the ChatGPT plan.
pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
/// Path of the loopback callback.
pub const CALLBACK_PATH: &str = "/auth/callback";
/// The port OpenAI's examples use; any free port works when it is taken.
pub const PREFERRED_PORT: u16 = 1455;
/// How long a sign-in waits for the browser.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
/// The private app config file holding the credential record.
pub const CREDENTIALS_FILE: &str = "chatgpt-auth.json";

/// Access tokens are renewed this long before they expire.
const REFRESH_MARGIN_SECS: u64 = 120;
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
/// Pause before the second revocation attempt (doubled for the third).
const REVOKE_BACKOFF: Duration = Duration::from_millis(500);
/// Auth replies are small; anything far larger is refused.
const MAX_REPLY_BYTES: usize = 1024 * 1024;
/// How PlexusTyping identifies itself to auth.openai.com.
const USER_AGENT: &str = concat!("PlexusTyping/", env!("CARGO_PKG_VERSION"));
/// OAuth errors after which a refresh token can never work again.
const TERMINAL_REFRESH_ERRORS: [&str; 6] = [
    "invalid_grant",
    "invalid_refresh_token",
    "token_expired",
    "refresh_token_expired",
    "refresh_token_invalidated",
    "refresh_token_reused",
];

/// What the frontend may know about the sign-in (never a token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub signed_in: bool,
    pub email: Option<String>,
    /// The grant includes [`PLAN_SCOPE`]: inference may use the ChatGPT plan.
    pub plan_enabled: bool,
}

/// The result of [`ChatgptAuth::sign_out`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignOut {
    /// False when OpenAI could not confirm that the session was revoked.
    pub revoked: bool,
}

/// Why a ChatGPT sign-in, refresh or sign-out failed, serialized as
/// `{ "kind": ... }`. Messages are Spanish and never contain a credential.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AuthError {
    /// No folder to keep [`CREDENTIALS_FILE`] in.
    #[error("PlexusTyping no tiene una carpeta donde guardar la sesión de ChatGPT")]
    NoStorage,
    /// Reading or writing [`CREDENTIALS_FILE`] failed.
    #[error("No se pudo guardar o leer la sesión de ChatGPT: {message}")]
    Storage { message: String },
    /// [`CREDENTIALS_FILE`] is not a credential record; only sign-out resets it.
    #[error(
        "El archivo de sesión de ChatGPT ({CREDENTIALS_FILE}) está dañado. Usa «Cerrar sesión» para empezar de nuevo"
    )]
    DamagedFile,
    /// The loopback listener could not start or failed.
    #[error("No se pudo preparar la respuesta del navegador: {message}")]
    Listener { message: String },
    /// The system browser could not be opened.
    #[error("No se pudo abrir el navegador")]
    Browser,
    /// The pending sign-in was cancelled (or replaced by a newer one).
    #[error("Se canceló el inicio de sesión en ChatGPT")]
    Cancelled,
    /// No callback arrived within [`SIGN_IN_TIMEOUT`].
    #[error("El inicio de sesión en ChatGPT ha caducado. Inténtalo de nuevo")]
    Timeout,
    /// The user declined consent.
    #[error("Has rechazado el permiso, así que PlexusTyping no puede usar tu plan de ChatGPT")]
    AccessDenied,
    /// Another OAuth error in the callback.
    #[error("ChatGPT no pudo completar el inicio de sesión ({code})")]
    Authorization { code: String },
    /// The callback carried no authorization code.
    #[error("ChatGPT no devolvió un código de autorización")]
    MissingCode,
    /// A new registration returned no issued client id.
    #[error("ChatGPT no terminó de registrar PlexusTyping. Inténtalo de nuevo")]
    MissingClientId,
    /// The callback names another client than the pending attempt.
    #[error("ChatGPT devolvió otro registro de la aplicación; no se ha usado")]
    ClientIdMismatch,
    /// The code exchange failed (twice for `invalid_grant`).
    #[error("No se pudo canjear el código de ChatGPT ({detail}). Inténtalo de nuevo")]
    CodeExchange { detail: String },
    /// OpenAI's openid-configuration or signing keys (JWKS) did not answer;
    /// nothing is saved.
    #[error(
        "No se pudieron obtener las claves de OpenAI para verificar el inicio de sesión. Comprueba la conexión e inténtalo de nuevo"
    )]
    KeysUnavailable,
    /// The ID token failed validation; `reason` is a fixed text.
    #[error("ChatGPT envió un inicio de sesión que PlexusTyping no pudo verificar ({reason})")]
    InvalidIdToken { reason: String },
    /// Another account signed in than the one the registration belongs to.
    #[error("Ha iniciado sesión otra cuenta de ChatGPT distinta de la registrada; no se ha usado")]
    IdentityMismatch,
    /// No account is signed in.
    #[error("Inicia sesión en ChatGPT")]
    NotSignedIn,
    /// Signed in, but the grant lacks [`PLAN_SCOPE`]: no inference is sent.
    #[error(
        "El uso de tu plan de ChatGPT no está activado para PlexusTyping. Inicia sesión de nuevo y permítelo"
    )]
    PlanNotEnabled,
    /// The refresh token is no longer usable: sign in again.
    #[error("Tu sesión de ChatGPT ha caducado. Vuelve a iniciar sesión")]
    SessionExpired,
    /// OpenAI no longer accepts the registration (`invalid_client`).
    #[error("ChatGPT ya no acepta el registro de PlexusTyping. Cierra sesión y vuelve a iniciarla")]
    InvalidClient,
    /// The refresh was refused for another reason; the tokens are kept.
    #[error("ChatGPT no renovó la sesión ({detail})")]
    RefreshRefused { detail: String },
    /// auth.openai.com is unavailable (HTTP 5xx); the tokens are kept.
    #[error("El servicio de inicio de sesión de ChatGPT no está disponible ahora (HTTP {status})")]
    Unavailable { status: u16 },
    /// Connection failure or timeout talking to auth.openai.com.
    #[error("No se pudo conectar con auth.openai.com: {message}")]
    Network { message: String },
    /// A reply that could not be understood.
    #[error("Respuesta inesperada de auth.openai.com")]
    BadResponse,
}

/// The saved registration and token set. The host and client ids outlive a sign-out.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(default)]
    host_id: String,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
    /// Unix seconds.
    #[serde(default)]
    expires_at: u64,
}

impl std::fmt::Debug for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Record")
            .field("client_id", &self.client_id)
            .field("signed_in", &self.signed_in())
            .field("scopes", &self.scopes)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl Record {
    fn signed_in(&self) -> bool {
        self.access_token.is_some() || self.refresh_token.is_some()
    }

    fn plan_enabled(&self) -> bool {
        self.signed_in() && self.scopes.iter().any(|s| s == PLAN_SCOPE)
    }

    fn status(&self) -> Status {
        Status {
            signed_in: self.signed_in(),
            email: self.email.clone().filter(|_| self.signed_in()),
            plan_enabled: self.plan_enabled(),
        }
    }

    /// Forgets the tokens; the registration (host, client, account) is kept for a later sign-in.
    fn clear_tokens(&mut self) {
        self.id_token = None;
        self.access_token = None;
        self.refresh_token = None;
        self.scopes.clear();
        self.expires_at = 0;
    }
}

/// The token endpoint's answer.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
}

/// Values fresh for each authorization attempt.
struct Attempt {
    state: String,
    nonce: String,
    verifier: String,
}

impl Attempt {
    fn new() -> Self {
        Attempt {
            state: random_token(),
            nonce: random_token(),
            verifier: random_token(),
        }
    }
}

/// How one authorization round ended.
enum Round {
    Done(Result<Status, AuthError>),
    /// The code exchange returned `invalid_grant`: start a fresh authorization
    /// with the client id retained for this attempt.
    Again {
        client_id: String,
    },
}

/// The ChatGPT sign-in: one per app (managed state).
pub struct ChatgptAuth {
    http: reqwest::Client,
    endpoints: Endpoints,
    verifier: Arc<Verifier>,
    path: Mutex<Option<PathBuf>>,
    /// Held for every read-modify-write of the record (shared with a refresh's own task).
    lock: Arc<tokio::sync::Mutex<()>>,
    /// The pending sign-in (its attempt number), ended when its sender is dropped or sent to.
    pending: Mutex<Option<(u64, tokio::sync::oneshot::Sender<()>)>>,
    /// Numbers sign-in attempts, so one never clears another's pending slot.
    attempts: AtomicU64,
    /// Bumped by every cancel and sign-out: a sign-in that saw an older value
    /// stops before it saves anything, wherever it is.
    cancels: AtomicU64,
    port: u16,
    timeout: Duration,
    revoke_backoff: Duration,
}

impl std::fmt::Debug for ChatgptAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatgptAuth")
            .field("auth", &self.endpoints.auth)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// Clears the pending slot of one sign-in attempt when that attempt ends, however it ends.
struct PendingSlot<'a> {
    auth: &'a ChatgptAuth,
    number: u64,
}

impl Drop for PendingSlot<'_> {
    fn drop(&mut self) {
        // Only this attempt's slot: a newer sign-in may have replaced it meanwhile.
        let mut pending = self.auth.pending.lock().unwrap_or_else(|p| p.into_inner());
        if pending.as_ref().is_some_and(|(n, _)| *n == self.number) {
            pending.take();
        }
    }
}

/// Longest lifetime taken from an `expires_in`: a larger one only means an
/// earlier renewal, never an overflow.
const MAX_EXPIRES_IN_SECS: u64 = 24 * 3600;

/// When a token given `expires_in` seconds now stops being used (one hour when absent).
fn expiry(expires_in: Option<u64>) -> u64 {
    now_secs().saturating_add(expires_in.unwrap_or(3600).min(MAX_EXPIRES_IN_SECS))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The saved record; a file that is not a record is reported, never taken as "signed out".
async fn load_from(path: &Path) -> Result<Record, AuthError> {
    let path = path.to_path_buf();
    let text = blocking(move || file::load(&path)).await?;
    match text {
        None => Ok(Record::default()),
        Some(text) => serde_json::from_str(&text).map_err(|_| AuthError::DamagedFile),
    }
}

async fn save_to(path: &Path, record: &Record) -> Result<(), AuthError> {
    let text = serde_json::to_string_pretty(record).map_err(|_| AuthError::Storage {
        message: "no se pudo preparar el registro".into(),
    })?;
    let path = path.to_path_buf();
    blocking(move || file::save(&path, &text)).await
}

/// File work runs on the blocking pool: a save may fsync or, on Windows, wait
/// for another process to release the file.
async fn blocking<T: Send + 'static>(
    op: impl FnOnce() -> Result<T, AuthError> + Send + 'static,
) -> Result<T, AuthError> {
    tokio::task::spawn_blocking(op).await.unwrap_or_else(|err| {
        Err(AuthError::Storage {
            message: err.to_string(),
        })
    })
}

impl ChatgptAuth {
    /// A sign-in against `endpoints` (use [`Endpoints::production`]) whose record
    /// lives at `path` (`<app config dir>/`[`CREDENTIALS_FILE`]).
    pub fn new(endpoints: &Endpoints, path: Option<PathBuf>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(HTTP_TIMEOUT)
            .https_only(!endpoints.allow_http)
            .build()
            .unwrap_or_default();
        let verifier = Arc::new(Verifier::new(http.clone(), &endpoints.auth));
        ChatgptAuth {
            http,
            endpoints: endpoints.clone(),
            verifier,
            path: Mutex::new(path),
            lock: Arc::default(),
            pending: Mutex::new(None),
            attempts: AtomicU64::new(0),
            cancels: AtomicU64::new(0),
            port: PREFERRED_PORT,
            timeout: SIGN_IN_TIMEOUT,
            revoke_backoff: REVOKE_BACKOFF,
        }
    }

    /// Loopback port tried first (`0`: any free port).
    pub fn with_callback_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// How long a sign-in waits for the browser.
    pub fn with_sign_in_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Pause before the second revocation attempt.
    pub fn with_revoke_backoff(mut self, backoff: Duration) -> Self {
        self.revoke_backoff = backoff;
        self
    }

    pub fn callback_port(&self) -> u16 {
        self.port
    }

    pub fn sign_in_timeout(&self) -> Duration {
        self.timeout
    }

    /// Where the credential record is kept (ignored when `None`).
    pub fn set_path(&self, path: Option<PathBuf>) {
        let mut current = self.path.lock().unwrap_or_else(|p| p.into_inner());
        if path.is_some() {
            *current = path;
        }
    }

    fn path(&self) -> Result<PathBuf, AuthError> {
        self.path
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
            .ok_or(AuthError::NoStorage)
    }

    async fn load(&self) -> Result<Record, AuthError> {
        load_from(&self.path()?).await
    }

    async fn save(&self, record: &Record) -> Result<(), AuthError> {
        save_to(&self.path()?, record).await
    }

    fn cancelled_since(&self, epoch: u64) -> bool {
        self.cancels.load(Ordering::SeqCst) != epoch
    }

    /// Whether a sign-in is waiting for the browser.
    pub fn sign_in_pending(&self) -> bool {
        self.pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some()
    }

    pub async fn status(&self) -> Result<Status, AuthError> {
        let _guard = self.lock.lock().await;
        Ok(self.load().await?.status())
    }

    /// Signed in with ChatGPT plan use enabled (what the provider chain needs).
    pub async fn is_signed_in(&self) -> Result<bool, AuthError> {
        Ok(self.status().await?.plan_enabled)
    }

    /// Ends the running sign-in, if any: one waiting for the browser at once,
    /// one in any other step before it saves its tokens.
    pub fn cancel(&self) {
        self.cancels.fetch_add(1, Ordering::SeqCst);
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some((_, tx)) = pending {
            let _ = tx.send(());
        }
    }

    /// Signs in through the browser: `open` gets the authorization URL to show
    /// the user (it may carry an ID-token hint: never log it). Called twice when
    /// OpenAI rejects the first code with `invalid_grant`.
    pub async fn sign_in<F>(&self, open: F) -> Result<Status, AuthError>
    where
        F: Fn(&str) -> Result<(), String> + Send + Sync,
    {
        let epoch = self.cancels.load(Ordering::SeqCst);
        // A stable host id is chosen and saved before the first sign-in.
        let saved = {
            let _guard = self.lock.lock().await;
            let mut record = self.load().await?;
            if record.host_id.is_empty() {
                record.host_id = new_host_id();
                self.save(&record).await?;
            }
            record
        };
        match self.authorize(&saved, None, &open, epoch).await {
            Round::Done(result) => result,
            Round::Again { client_id } => {
                match self.authorize(&saved, Some(client_id), &open, epoch).await {
                    Round::Done(result) => result,
                    Round::Again { .. } => Err(AuthError::CodeExchange {
                        detail: "invalid_grant".into(),
                    }),
                }
            }
        }
    }

    /// One authorization round: browser, callback, code exchange, ID-token check, save.
    /// `retained` is the client id kept from a round whose code was rejected.
    /// `epoch` is the cancel count when the sign-in began.
    async fn authorize<F>(
        &self,
        saved: &Record,
        retained: Option<String>,
        open: &F,
        epoch: u64,
    ) -> Round
    where
        F: Fn(&str) -> Result<(), String> + Send + Sync,
    {
        let done = |err: AuthError| Round::Done(Err(err));
        if self.cancelled_since(epoch) {
            return done(AuthError::Cancelled);
        }
        let listener = match callback::bind(self.port).await {
            Ok(listener) => listener,
            Err(err) => return done(err),
        };
        let port = match listener.local_addr() {
            Ok(addr) => addr.port(),
            Err(err) => {
                return done(AuthError::Listener {
                    message: err.to_string(),
                });
            }
        };
        let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
        let attempt = Attempt::new();
        // The client this attempt is for: retained, saved, or a new registration.
        let expected = retained.or_else(|| saved.client_id.clone());
        let url = authorize_url(
            &self.endpoints,
            saved,
            expected.as_deref(),
            &redirect_uri,
            &attempt,
        );
        if open(&url).is_err() {
            return done(AuthError::Browser);
        }

        let (tx, mut cancelled) = tokio::sync::oneshot::channel();
        let number = self.attempts.fetch_add(1, Ordering::Relaxed);
        // A new sign-in ends the previous one (its sender is dropped).
        *self.pending.lock().unwrap_or_else(|p| p.into_inner()) = Some((number, tx));
        let slot = PendingSlot { auth: self, number };
        // A cancel that came before the slot was registered had nothing to send to.
        if self.cancelled_since(epoch) {
            return done(AuthError::Cancelled);
        }

        let params = tokio::select! {
            params = callback::wait(&listener, &attempt.state) => match params {
                Ok(params) => params,
                Err(err) => return done(err),
            },
            _ = &mut cancelled => return done(AuthError::Cancelled),
            _ = tokio::time::sleep(self.timeout) => return done(AuthError::Timeout),
        };
        drop(listener);
        drop(slot);

        if let Some(error) = params.get("error") {
            if error == "access_denied" {
                return done(AuthError::AccessDenied);
            }
            return done(AuthError::Authorization {
                code: safe_code(error),
            });
        }
        let Some(code) = params.get("code").filter(|c| !c.is_empty()) else {
            return done(AuthError::MissingCode);
        };
        let client_id = match (&expected, params.get("client_id")) {
            (None, Some(issued)) if valid_client_id(issued) => issued.clone(),
            (None, _) => return done(AuthError::MissingClientId),
            (Some(expected), Some(issued)) if issued != expected => {
                return done(AuthError::ClientIdMismatch);
            }
            (Some(expected), _) => expected.clone(),
        };

        let exchanged = post_form(
            &self.http,
            &token_url(&self.endpoints),
            &[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", code),
                ("code_verifier", &attempt.verifier),
                ("redirect_uri", &redirect_uri),
                ("resource", RESOURCE),
            ],
        )
        .await;
        let (status, body) = match exchanged {
            Ok(reply) => reply,
            Err(err) => return done(err),
        };
        if !(200..300).contains(&status) {
            let error = oauth_error(&body);
            if error.as_deref() == Some("invalid_grant") {
                return Round::Again { client_id };
            }
            return done(AuthError::CodeExchange {
                detail: error.unwrap_or_else(|| format!("HTTP {status}")),
            });
        }
        Round::Done(
            self.finish(saved, client_id, &attempt.nonce, &body, epoch)
                .await,
        )
    }

    /// Validates the token response of a sign-in and saves the new token set.
    async fn finish(
        &self,
        saved: &Record,
        client_id: String,
        nonce: &str,
        body: &[u8],
        epoch: u64,
    ) -> Result<Status, AuthError> {
        let tokens: TokenResponse =
            serde_json::from_slice(body).map_err(|_| AuthError::BadResponse)?;
        let id_token = tokens.id_token.clone().ok_or(AuthError::BadResponse)?;
        let claims = self
            .verifier
            .validate(&id_token, &client_id, Some(nonce))
            .await?;
        if saved.subject.as_ref().is_some_and(|s| *s != claims.sub) {
            return Err(AuthError::IdentityMismatch);
        }
        // Only the token response's granted scopes count (not the callback's).
        let granted = tokens.scope.clone().unwrap_or_default();

        let guard = self.lock.lock().await;
        // Checked under the lock a sign-out clears the tokens with: a sign-out or cancel
        // during the exchange wins over its answer, whose grant is then revoked.
        if self.cancelled_since(epoch) {
            drop(guard);
            return self.discard(tokens.refresh_token, client_id).await;
        }
        let previous = self.load().await?;
        let mut record = previous.clone();
        record.host_id = saved.host_id.clone();
        record.client_id = Some(client_id.clone());
        record.subject = Some(claims.sub);
        record.email = claims.email;
        record.id_token = Some(id_token);
        record.access_token = Some(tokens.access_token);
        record.refresh_token = tokens.refresh_token;
        record.scopes = granted.split_whitespace().map(str::to_string).collect();
        record.expires_at = expiry(tokens.expires_in);
        self.save(&record).await?;
        // A plain cancel takes no lock: one that landed during the save still wins.
        if self.cancelled_since(epoch) {
            // The new grant is revoked even when the restore fails (then its error is returned).
            let restored = self.save(&previous).await;
            drop(guard);
            let cancelled = self.discard(record.refresh_token, client_id).await;
            restored?;
            return cancelled;
        }
        Ok(record.status())
    }

    /// Ends a cancelled sign-in: the refresh token its answer carried is revoked
    /// (best effort, at most [`DISCARD_REVOKE_LIMIT`] so a cancel stays prompt),
    /// so no unused grant stays alive.
    async fn discard(
        &self,
        refresh: Option<String>,
        client_id: String,
    ) -> Result<Status, AuthError> {
        if let Some(refresh) = refresh {
            let _ =
                tokio::time::timeout(DISCARD_REVOKE_LIMIT, self.revoke(&refresh, &client_id)).await;
        }
        Err(AuthError::Cancelled)
    }

    /// A valid access token for inference, renewed first when it is about to expire.
    pub async fn access_token(&self) -> Result<ApiKey, AuthError> {
        self.token(None).await
    }

    /// A new access token after the API rejected `rejected` (HTTP 401): renewed
    /// now, even before its expiry, unless another request already replaced it
    /// (then the saved one is returned without a refresh).
    pub async fn refresh_rejected(&self, rejected: &ApiKey) -> Result<ApiKey, AuthError> {
        self.token(Some(rejected.expose())).await
    }

    /// The saved access token while it is valid and is not `rejected`; otherwise
    /// one refresh (serialized on the record lock).
    async fn token(&self, rejected: Option<&str>) -> Result<ApiKey, AuthError> {
        let path = self.path()?;
        let guard = self.lock.clone().lock_owned().await;
        let mut record = load_from(&path).await?;
        if !record.signed_in() {
            return Err(AuthError::NotSignedIn);
        }
        if !record.plan_enabled() {
            return Err(AuthError::PlanNotEnabled);
        }
        if let Some(token) = &record.access_token
            && now_secs() + REFRESH_MARGIN_SECS < record.expires_at
            && rejected != Some(token.as_str())
        {
            return ApiKey::new(token.clone()).map_err(|_| AuthError::NotSignedIn);
        }
        let (Some(refresh_token), Some(client_id)) =
            (record.refresh_token.clone(), record.client_id.clone())
        else {
            record.clear_tokens();
            save_to(&path, &record).await?;
            return Err(AuthError::SessionExpired);
        };
        // The refresh runs on its own task holding the lock: once the rotating refresh token is
        // sent, the answer is read and saved even when the request that asked is dropped.
        let http = self.http.clone();
        let url = token_url(&self.endpoints);
        let verifier = self.verifier.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let refresh = Refresh {
                http: &http,
                url: &url,
                path: &path,
                verifier: &verifier,
            };
            refresh.run(record, &refresh_token, &client_id).await
        })
        .await
        .map_err(|_| AuthError::Network {
            message: "la renovación se interrumpió".into(),
        })?
    }

    /// Revokes the renewable session, then forgets the tokens even when revocation failed.
    /// The host id, client id and email (the next `login_hint`) stay; the account subject is
    /// forgotten, so any ChatGPT account may sign in next.
    pub async fn sign_out(&self) -> Result<SignOut, AuthError> {
        self.cancel();
        let path = self.path()?;
        // The tokens are forgotten first, under the lock; the revocation then runs on a copy,
        // so a stalled network never holds up the other requests.
        let (refresh, client_id) = {
            let _guard = self.lock.lock().await;
            match load_from(&path).await {
                Ok(mut record) => {
                    let copy = (record.refresh_token.clone(), record.client_id.clone());
                    record.clear_tokens();
                    // The next sign-in may be another ChatGPT account: only a renewal or a
                    // sign-in while still signed in must stay on the same one. The email is
                    // kept as the next sign-in's `login_hint`.
                    record.subject = None;
                    save_to(&path, &record).await?;
                    copy
                }
                // A damaged file starts over: there is nothing that could be revoked.
                Err(AuthError::DamagedFile) => {
                    save_to(&path, &Record::default()).await?;
                    return Ok(SignOut { revoked: false });
                }
                Err(err) => return Err(err),
            }
        };
        let revoked = match (refresh, client_id) {
            (Some(refresh), Some(client_id)) => self.revoke(&refresh, &client_id).await,
            _ => true,
        };
        Ok(SignOut { revoked })
    }

    /// Revokes the refresh token at the issuer's `revocation_endpoint` (from the
    /// openid-configuration, on the sign-in service's own origin), retrying
    /// network and server failures twice.
    async fn revoke(&self, refresh: &str, client_id: &str) -> bool {
        let Some(endpoint) = self.verifier.revocation_endpoint().await else {
            return false;
        };
        for attempt in 0..3u32 {
            if attempt > 0 {
                tokio::time::sleep(self.revoke_backoff * attempt).await;
            }
            let sent = post_form(
                &self.http,
                &endpoint,
                &[
                    ("token", refresh),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", client_id),
                ],
            )
            .await;
            match sent {
                Ok((200..=299, _)) => return true,
                Ok((500..=599, _)) | Err(_) => continue,
                Ok(_) => return false,
            }
        }
        false
    }
}

/// Longest wait for revoking the grant of a cancelled sign-in.
const DISCARD_REVOKE_LIMIT: Duration = Duration::from_secs(5);

/// One refresh, run on its own task while it holds the record lock.
struct Refresh<'a> {
    http: &'a reqwest::Client,
    url: &'a str,
    path: &'a Path,
    verifier: &'a Verifier,
}

impl Refresh<'_> {
    /// Renews the access token with the rotating refresh token and saves the new set.
    async fn run(
        &self,
        mut record: Record,
        refresh_token: &str,
        client_id: &str,
    ) -> Result<ApiKey, AuthError> {
        let (status, body) = post_form(
            self.http,
            self.url,
            &[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("refresh_token", refresh_token),
                ("resource", RESOURCE),
            ],
        )
        .await?;
        if !(200..300).contains(&status) {
            let error = oauth_error(&body);
            if error
                .as_deref()
                .is_some_and(|e| TERMINAL_REFRESH_ERRORS.contains(&e))
            {
                record.clear_tokens();
                save_to(self.path, &record).await?;
                return Err(AuthError::SessionExpired);
            }
            if error.as_deref() == Some("invalid_client") {
                return Err(AuthError::InvalidClient);
            }
            return Err(match status {
                500..=599 => AuthError::Unavailable { status },
                _ => AuthError::RefreshRefused {
                    detail: error.unwrap_or_else(|| format!("HTTP {status}")),
                },
            });
        }
        let tokens: TokenResponse =
            serde_json::from_slice(&body).map_err(|_| AuthError::BadResponse)?;
        record.access_token = Some(tokens.access_token.clone());
        if let Some(replacement) = tokens.refresh_token {
            record.refresh_token = Some(replacement);
        }
        if let Some(scope) = tokens.scope {
            record.scopes = scope.split_whitespace().map(str::to_string).collect();
        }
        record.expires_at = expiry(tokens.expires_in);
        // Saved before anything else may wait on the network: the old refresh token is
        // already spent, so losing the new one would end the session.
        save_to(self.path, &record).await?;
        // A new ID token is kept only when it checks out for the same account (it becomes the
        // next sign-in's `id_token_hint`); a refresh carries no nonce.
        if let Some(id_token) = tokens.id_token
            && self
                .verifier
                .validate(&id_token, client_id, None)
                .await
                .is_ok_and(|claims| record.subject.as_deref() == Some(claims.sub.as_str()))
        {
            record.id_token = Some(id_token);
            save_to(self.path, &record).await?;
        }
        if !record.plan_enabled() {
            return Err(AuthError::PlanNotEnabled);
        }
        ApiKey::new(tokens.access_token).map_err(|_| AuthError::BadResponse)
    }
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    SystemRandom::new()
        .fill(&mut bytes)
        .expect("the system random source failed");
    bytes
}

/// 32 random bytes, base64url (43 characters, a valid PKCE verifier).
pub fn random_token() -> String {
    B64.encode(random_bytes::<32>())
}

/// A UUIDv4 URN, the host id format OpenAI accepts.
pub fn new_host_id() -> String {
    let mut b = random_bytes::<16>();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The PKCE S256 challenge of `verifier`: base64url(SHA-256), no padding.
pub fn pkce_challenge(verifier: &str) -> String {
    B64.encode(digest::digest(&digest::SHA256, verifier.as_bytes()))
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
}

/// Decodes a query string; a repeated key keeps its first value.
pub fn parse_query(query: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        out.entry(k.into_owned()).or_insert_with(|| v.into_owned());
    }
    out
}

fn auth_base(endpoints: &Endpoints) -> &str {
    endpoints.auth.trim_end_matches('/')
}

fn token_url(endpoints: &Endpoints) -> String {
    format!("{}/api/accounts/oauth/token", auth_base(endpoints))
}

/// The browser URL of one authorization attempt. `client_id` is the issued
/// client to reauthorize (`None`: a new registration).
fn authorize_url(
    endpoints: &Endpoints,
    record: &Record,
    client_id: Option<&str>,
    redirect_uri: &str,
    attempt: &Attempt,
) -> String {
    let challenge = pkce_challenge(&attempt.verifier);
    let mut params: Vec<(&str, &str)> = vec![("response_type", "code")];
    match client_id {
        Some(client_id) => {
            params.push(("client_id", client_id));
            // The hints belong to the saved account: only with its own client.
            if record.client_id.as_deref() == Some(client_id) {
                if let Some(hint) = &record.id_token {
                    params.push(("id_token_hint", hint));
                }
                if let Some(email) = &record.email {
                    params.push(("login_hint", email));
                }
            }
        }
        None => {
            params.push(("client_id", REGISTRATION_CLIENT));
            params.push(("agent_name_hint", AGENT_NAME));
        }
    }
    params.extend([
        ("ext_agent_host_id", record.host_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("scope", SCOPE),
        ("resource", RESOURCE),
        ("state", attempt.state.as_str()),
        ("nonce", attempt.nonce.as_str()),
        ("code_challenge_method", "S256"),
        ("code_challenge", challenge.as_str()),
    ]);
    format!(
        "{}/api/accounts/authorize?{}",
        auth_base(endpoints),
        form_encode(&params)
    )
}

/// Issued client ids look like `oaiapp_...`; anything odd is refused.
fn valid_client_id(id: &str) -> bool {
    id != REGISTRATION_CLIENT
        && !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// An OAuth error code reduced to safe characters for a message.
fn safe_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(64)
        .collect()
}

/// The OAuth `error` code of a reply (never `error_description`, which may
/// echo a submitted value).
fn oauth_error(body: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(body).ok()?;
    let code = value["error"]
        .as_str()
        .or_else(|| value["error"]["code"].as_str())?;
    Some(safe_code(code)).filter(|c| !c.is_empty())
}

/// A network error whose text never includes the URL.
fn transport_error(err: reqwest::Error) -> AuthError {
    let message = if err.is_timeout() {
        "tiempo de espera agotado".to_string()
    } else if err.is_connect() {
        "sin conexión".to_string()
    } else {
        err.without_url().to_string()
    };
    AuthError::Network { message }
}

/// Reads at most [`MAX_REPLY_BYTES`] of a reply.
async fn read_body(mut response: reqwest::Response) -> Result<Vec<u8>, AuthError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
        if body.len() + chunk.len() > MAX_REPLY_BYTES {
            return Err(AuthError::BadResponse);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Form-encoded POST without any client secret; `(status, body)`.
async fn post_form(
    http: &reqwest::Client,
    url: &str,
    pairs: &[(&str, &str)],
) -> Result<(u16, Vec<u8>), AuthError> {
    let response = http
        .post(url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .body(form_encode(pairs))
        .send()
        .await
        .map_err(transport_error)?;
    let status = response.status().as_u16();
    Ok((status, read_body(response).await?))
}

/// `scheme://host[:port]` of a URL.
fn origin(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_once("://")?;
    let end = rest.find('/').unwrap_or(rest.len());
    Some(&url[..scheme.len() + 3 + end])
}

#[cfg(test)]
mod tests;
