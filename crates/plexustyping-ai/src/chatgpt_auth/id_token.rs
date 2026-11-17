//! ID-token validation: RS256 signature against OpenAI's published JWKS, then
//! issuer, audience (the issued client id), expiry, nonce and subject.
//!
//! The signature check is compliance with OpenAI's sign-in guide ("Verify its
//! signature against OpenAI's published JWKS") and defence in depth. The ID
//! token reaches PlexusTyping straight from the token endpoint over TLS, in
//! exchange for the code and PKCE verifier, and authorizes nothing: inference
//! uses the access token, the ID token only identifies the account and is sent
//! back as `id_token_hint`.
//!
//! The `jwks_uri` and `revocation_endpoint` come from the issuer's
//! openid-configuration and must be on the sign-in service's own origin. Keys
//! are cached, picked by `kid`, and fetched again once when a token names an
//! unknown `kid` (key rotation). Only RS256 is accepted, and only while the
//! discovery publishes it in `id_token_signing_alg_values_supported`.

use std::sync::Mutex;

use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet, KeyAlgorithm};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use super::{AuthError, origin, read_body};

/// Allowed clock skew for `exp` (OpenAI's example uses 5 s).
const CLOCK_LEEWAY_SECS: u64 = 5;

/// The validated identity in an ID token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Claims {
    pub sub: String,
    pub email: Option<String>,
}

#[derive(Deserialize)]
struct RawClaims {
    sub: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    nonce: Option<String>,
}

/// The parts of the openid-configuration PlexusTyping uses.
#[derive(Clone, Deserialize)]
struct Discovery {
    issuer: String,
    jwks_uri: String,
    #[serde(default)]
    revocation_endpoint: Option<String>,
    #[serde(default)]
    id_token_signing_alg_values_supported: Option<Vec<String>>,
}

fn invalid(reason: &str) -> AuthError {
    AuthError::InvalidIdToken {
        reason: reason.to_string(),
    }
}

pub(super) struct Verifier {
    http: reqwest::Client,
    /// The sign-in service root, which is also the expected issuer.
    issuer: String,
    discovery: Mutex<Option<Discovery>>,
    keys: Mutex<Option<JwkSet>>,
}

impl Verifier {
    pub(super) fn new(http: reqwest::Client, issuer: &str) -> Self {
        Verifier {
            http,
            issuer: issuer.trim_end_matches('/').to_string(),
            discovery: Mutex::new(None),
            keys: Mutex::new(None),
        }
    }

    /// The discovery document or the JWKS; any failure is [`AuthError::KeysUnavailable`].
    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, AuthError> {
        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| AuthError::KeysUnavailable)?;
        if !response.status().is_success() {
            return Err(AuthError::KeysUnavailable);
        }
        let body = read_body(response)
            .await
            .map_err(|_| AuthError::KeysUnavailable)?;
        serde_json::from_slice(&body).map_err(|_| AuthError::KeysUnavailable)
    }

    /// The issuer's openid-configuration (cached), checked to be its own.
    async fn discovery(&self) -> Result<Discovery, AuthError> {
        if let Some(found) = self
            .discovery
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
        {
            return Ok(found);
        }
        let url = format!("{}/.well-known/openid-configuration", self.issuer);
        let found: Discovery = self.get_json(&url).await?;
        let own = |u: &str| origin(u).is_some() && origin(u) == origin(&self.issuer);
        if found.issuer.trim_end_matches('/') != self.issuer || !own(&found.jwks_uri) {
            return Err(AuthError::KeysUnavailable);
        }
        *self.discovery.lock().unwrap_or_else(|p| p.into_inner()) = Some(found.clone());
        Ok(found)
    }

    /// The `revocation_endpoint`, when the discovery names one on the issuer's origin.
    pub(super) async fn revocation_endpoint(&self) -> Option<String> {
        let endpoint = self.discovery().await.ok()?.revocation_endpoint?;
        (origin(&endpoint).is_some() && origin(&endpoint) == origin(&self.issuer))
            .then_some(endpoint)
    }

    /// The RSA key `kid`, fetching the JWKS again once when it is unknown.
    async fn key(&self, kid: &str) -> Result<DecodingKey, AuthError> {
        let cached = self.keys.lock().unwrap_or_else(|p| p.into_inner()).clone();
        let set = match cached {
            Some(set) if set.find(kid).is_some() => set,
            _ => {
                let uri = self.discovery().await?.jwks_uri;
                let set: JwkSet = self.get_json(&uri).await?;
                *self.keys.lock().unwrap_or_else(|p| p.into_inner()) = Some(set.clone());
                set
            }
        };
        let jwk = set
            .find(kid)
            .ok_or_else(|| invalid("clave de firma desconocida"))?;
        let rsa = matches!(jwk.algorithm, AlgorithmParameters::RSA(_));
        let rs256 = matches!(jwk.common.key_algorithm, None | Some(KeyAlgorithm::RS256));
        if !rsa || !rs256 {
            return Err(invalid("clave de firma no admitida"));
        }
        DecodingKey::from_jwk(jwk).map_err(|_| invalid("clave de firma no admitida"))
    }

    /// Verifies `token` for `client_id`; `nonce` is checked when given (a
    /// refreshed ID token carries none).
    pub(super) async fn validate(
        &self,
        token: &str,
        client_id: &str,
        nonce: Option<&str>,
    ) -> Result<Claims, AuthError> {
        // `none`, HS* and every other algorithm are refused before any key is looked up.
        let header = jsonwebtoken::decode_header(token).map_err(|_| invalid("formato"))?;
        if header.alg != Algorithm::RS256 {
            return Err(invalid("algoritmo de firma no admitido"));
        }
        let kid = header.kid.ok_or_else(|| invalid("sin clave de firma"))?;
        let published = self
            .discovery()
            .await?
            .id_token_signing_alg_values_supported;
        if published.is_some_and(|algs| !algs.iter().any(|a| a == "RS256")) {
            return Err(invalid("algoritmo de firma no admitido"));
        }
        let key = self.key(&kid).await?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[client_id]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = CLOCK_LEEWAY_SECS;
        let data = jsonwebtoken::decode::<RawClaims>(token, &key, &validation).map_err(|err| {
            use jsonwebtoken::errors::ErrorKind as K;
            invalid(match err.kind() {
                K::InvalidSignature => "firma",
                K::ExpiredSignature => "caducado",
                K::InvalidAudience => "audiencia",
                K::InvalidIssuer => "emisor",
                K::InvalidAlgorithm => "algoritmo de firma no admitido",
                K::MissingRequiredClaim(_) => "faltan datos",
                _ => "formato",
            })
        })?;
        let claims = data.claims;
        if let Some(expected) = nonce
            && claims.nonce.as_deref() != Some(expected)
        {
            return Err(invalid("nonce"));
        }
        if claims.sub.trim().is_empty() {
            return Err(invalid("sin cuenta"));
        }
        Ok(Claims {
            sub: claims.sub,
            email: claims.email.filter(|e| !e.trim().is_empty()),
        })
    }
}
