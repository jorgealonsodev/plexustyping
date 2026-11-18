//! Sign in with ChatGPT against a loopback mock of auth.openai.com (S1, S2).
//! ID tokens are signed with RSA keys generated per test run (no network, no
//! committed keys); the credential file lives in a scratch folder under
//! `target/` that each test removes.

mod common;

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aws_lc_rs::hmac;
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::rsa::{KeySize, PublicKeyComponents};
use aws_lc_rs::signature::{KeyPair, RSA_PKCS1_SHA256, RsaKeyPair};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use common::{Captured, Mock, Reply, mock};
use plexustyping_ai::chatgpt_auth::{
    AuthError, CREDENTIALS_FILE, ChatgptAuth, SignOut, Status, parse_query, pkce_challenge,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const DISCOVERY: &str = "/.well-known/openid-configuration";
const JWKS: &str = "/.well-known/jwks.json";
const AUTHORIZE: &str = "/api/accounts/authorize";
const TOKEN: &str = "/api/accounts/oauth/token";
const REVOKE: &str = "/api/accounts/oauth/revoke";
const CLIENT: &str = "oaiapp_TEST-client-1";
const SUB: &str = "user-sub-1";
const EMAIL: &str = "ana@example.com";
const CODE: &str = "code-SECRET-auth-777";
const ACCESS: &str = "at-SECRET-access-111";
const REFRESH: &str = "rt-SECRET-refresh-111";
const ACCESS_2: &str = "at-SECRET-access-222";
const REFRESH_2: &str = "rt-SECRET-refresh-222";
const ACCESS_3: &str = "at-SECRET-access-333";
const REFRESH_3: &str = "rt-SECRET-refresh-333";
const PLAN: &str = "chatgpt.tokens.use.direct email offline_access openid profile resource.invoke";
const NO_PLAN: &str = "email offline_access openid profile";
const SECRETS: [&str; 7] = [
    CODE, ACCESS, REFRESH, ACCESS_2, REFRESH_2, ACCESS_3, REFRESH_3,
];

// ---------------------------------------------------------------- helpers

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// A scratch folder under `target/tmp`, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "chatgpt-auth-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An RSA signing key and its JWKS entry.
struct Signer {
    key: RsaKeyPair,
    kid: String,
}

impl Signer {
    fn new(kid: &str) -> Self {
        Signer {
            key: RsaKeyPair::generate(KeySize::Rsa2048).unwrap(),
            kid: kid.into(),
        }
    }

    fn jwk(&self) -> Value {
        let parts = PublicKeyComponents::<Vec<u8>>::from(self.key.public_key());
        json!({"kty": "RSA", "alg": "RS256", "use": "sig", "kid": self.kid,
               "n": B64.encode(&parts.n), "e": B64.encode(&parts.e)})
    }

    fn sign_with_header(&self, header: &Value, claims: &Value) -> String {
        let msg = format!(
            "{}.{}",
            B64.encode(header.to_string()),
            B64.encode(claims.to_string())
        );
        let mut sig = vec![0u8; self.key.public_modulus_len()];
        self.key
            .sign(
                &RSA_PKCS1_SHA256,
                &SystemRandom::new(),
                msg.as_bytes(),
                &mut sig,
            )
            .unwrap();
        format!("{msg}.{}", B64.encode(sig))
    }

    fn sign(&self, claims: &Value) -> String {
        let header = json!({"alg": "RS256", "typ": "JWT", "kid": self.kid});
        self.sign_with_header(&header, claims)
    }
}

/// What the simulated browser does with one authorize URL: the callback query
/// to send (`None`: nothing).
type Step = Box<dyn FnOnce(&HashMap<String, String>) -> Option<String> + Send>;

struct Fixture {
    mock: Mock,
    signer: Arc<Signer>,
    dir: Scratch,
    path: PathBuf,
    auth: ChatgptAuth,
}

fn discovery(base: &str) -> Value {
    json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}{AUTHORIZE}"),
        "token_endpoint": format!("{base}{TOKEN}"),
        "jwks_uri": format!("{base}{JWKS}"),
        "revocation_endpoint": format!("{base}{REVOKE}"),
        "id_token_signing_alg_values_supported": ["RS256"],
    })
}

impl Fixture {
    /// The mock serves the discovery and a JWKS with `kid-1`, always.
    async fn new() -> Self {
        let signer = Arc::new(Signer::new("kid-1"));
        let jwks = Reply::ok(json!({"keys": [signer.jwk()]})).repeated();
        Fixture::with(
            signer,
            Some(Reply::ok(discovery("")).repeated()),
            vec![jwks],
        )
        .await
    }

    /// `discovery: None` leaves that route out (404); its `issuer` and URLs
    /// are filled in with the mock's base when they are empty.
    async fn with(signer: Arc<Signer>, discovery_reply: Option<Reply>, jwks: Vec<Reply>) -> Self {
        let mock = mock(Vec::new()).await;
        if let Some(mut reply) = discovery_reply {
            if reply.body.contains("\"issuer\":\"\"") {
                reply.body = discovery(&mock.base).to_string();
            }
            mock.route(DISCOVERY, reply);
        }
        for reply in jwks {
            mock.route(JWKS, reply);
        }
        let dir = Scratch::new();
        let path = dir.0.join("config").join(CREDENTIALS_FILE);
        let auth = manager(&mock, &path);
        Fixture {
            mock,
            signer,
            dir,
            path,
            auth,
        }
    }

    fn record(&self) -> Value {
        serde_json::from_str(&std::fs::read_to_string(&self.path).unwrap()).unwrap()
    }

    fn write_record(&self, record: Value) {
        std::fs::create_dir_all(self.path.parent().unwrap()).unwrap();
        std::fs::write(&self.path, record.to_string()).unwrap();
    }

    fn calls(&self, path: &str) -> Vec<Captured> {
        self.mock
            .requests()
            .into_iter()
            .filter(|r| r.path.split('?').next() == Some(path))
            .collect()
    }

    /// The token endpoint answers a sign-in with a valid ID token for the
    /// request's nonce; the browser comes back with `code` and `state`
    /// (`client_id` too when `issued`).
    fn approve(&self, expires_in: u64, scope: &'static str, issued: bool) -> Step {
        let (mock, signer) = (self.mock.clone(), self.signer.clone());
        Box::new(move |p| {
            let id = signer.sign(&claims(&mock.base, &p["nonce"]));
            mock.route(TOKEN, tokens(&id, ACCESS, REFRESH, expires_in, scope));
            let client = if issued {
                format!("&client_id={CLIENT}")
            } else {
                String::new()
            };
            Some(format!(
                "code={CODE}&state={}{client}&scope={}",
                p["state"],
                scope.replace(' ', "+")
            ))
        })
    }

    /// Like [`Fixture::approve`], with the ID token built by `make` from the claims.
    fn approve_with(&self, make: impl FnOnce(&Signer, Value) -> String + Send + 'static) -> Step {
        let (mock, signer) = (self.mock.clone(), self.signer.clone());
        Box::new(move |p| {
            let id = make(&signer, claims(&mock.base, &p["nonce"]));
            mock.route(TOKEN, tokens(&id, ACCESS, REFRESH, 3600, PLAN));
            Some(format!(
                "code={CODE}&state={}&client_id={CLIENT}",
                p["state"]
            ))
        })
    }

    async fn sign_in(&self, steps: Vec<Step>) -> Attempt {
        let steps = Arc::new(Mutex::new(VecDeque::from(steps)));
        let opened: Arc<Mutex<Vec<(String, tokio::task::JoinHandle<Option<(u16, String)>>)>>> =
            Arc::default();
        let log = opened.clone();
        let result = self
            .auth
            .sign_in(move |url: &str| {
                let params = query(url);
                let step = steps.lock().unwrap().pop_front().expect("a scripted step");
                let task = match step(&params) {
                    Some(q) => {
                        let target = format!("{}?{q}", params["redirect_uri"]);
                        tokio::spawn(async move { visit(&target).await })
                    }
                    None => tokio::spawn(async { None }),
                };
                log.lock().unwrap().push((url.to_string(), task));
                Ok(())
            })
            .await;
        let opened = std::mem::take(&mut *opened.lock().unwrap());
        let mut urls = Vec::new();
        let mut pages = Vec::new();
        for (url, task) in opened {
            urls.push(url);
            pages.push(task.await.unwrap());
        }
        Attempt {
            result,
            urls,
            pages,
        }
    }
}

fn manager(mock: &Mock, path: &Path) -> ChatgptAuth {
    ChatgptAuth::new(&mock.endpoints(), Some(path.to_path_buf()))
        .with_callback_port(0)
        .with_sign_in_timeout(Duration::from_secs(5))
        .with_revoke_backoff(Duration::from_millis(5))
}

struct Attempt {
    result: Result<Status, AuthError>,
    /// Every authorize URL opened, in order.
    urls: Vec<String>,
    /// What the browser got back for each.
    pages: Vec<Option<(u16, String)>>,
}

impl Attempt {
    fn authorize(&self, i: usize) -> HashMap<String, String> {
        query(&self.urls[i])
    }
}

fn query(url: &str) -> HashMap<String, String> {
    parse_query(url.split_once('?').map_or("", |(_, q)| q))
}

fn form(c: &Captured) -> HashMap<String, String> {
    parse_query(&c.body)
}

fn keys(map: &HashMap<String, String>) -> Vec<&str> {
    let mut k: Vec<&str> = map.keys().map(String::as_str).collect();
    k.sort_unstable();
    k
}

fn claims(base: &str, nonce: &str) -> Value {
    json!({"iss": base, "aud": [CLIENT], "sub": SUB, "email": EMAIL,
           "email_verified": true, "nonce": nonce, "iat": now(), "exp": now() + 3600})
}

fn tokens(id_token: &str, access: &str, refresh: &str, expires_in: u64, scope: &str) -> Reply {
    Reply::ok(json!({
        "access_token": access, "refresh_token": refresh, "id_token": id_token,
        "token_type": "Bearer", "expires_in": expires_in, "scope": scope,
        "earliest_refresh_at": now()
    }))
}

fn refreshed(access: &str, refresh: &str, expires_in: u64) -> Reply {
    Reply::ok(json!({"access_token": access, "refresh_token": refresh,
                     "token_type": "Bearer", "expires_in": expires_in, "scope": PLAN}))
}

/// Raw GET of a loopback `url`; `(status, body)`.
async fn visit(url: &str) -> Option<(u16, String)> {
    let rest = url.strip_prefix("http://")?;
    let (host, path) = rest.split_at(rest.find('/')?);
    let mut sock = tokio::net::TcpStream::connect(host).await.ok()?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    sock.write_all(req.as_bytes()).await.ok()?;
    let mut out = Vec::new();
    sock.read_to_end(&mut out).await.ok()?;
    let text = String::from_utf8_lossy(&out).to_string();
    let status = text.split(' ').nth(1)?.parse().ok()?;
    let body = text.split_once("\r\n\r\n")?.1.to_string();
    Some((status, body))
}

fn assert_clean(text: &str) {
    for secret in SECRETS {
        assert!(!text.contains(secret), "secret leaked: {text}");
    }
}

/// Display, Debug and JSON of `err`, checked for secrets.
fn shown(err: &AuthError) -> String {
    let text = format!("{err} {err:?} {}", serde_json::to_string(err).unwrap());
    assert_clean(&text);
    text
}

fn signed_in() -> Status {
    Status {
        signed_in: true,
        email: Some(EMAIL.into()),
        plan_enabled: true,
    }
}

// --------------------------------------------------------------- sign-in

#[tokio::test]
async fn first_sign_in_registers_exchanges_with_pkce_and_saves_a_private_record() {
    let f = Fixture::new().await;
    let a = f.sign_in(vec![f.approve(3600, PLAN, true)]).await;

    assert_eq!(a.result.clone().unwrap(), signed_in());
    assert!(
        a.urls[0].starts_with(&format!("{}{AUTHORIZE}?", f.mock.base)),
        "{}",
        a.urls[0]
    );
    let q = a.authorize(0);
    assert_eq!(q["client_id"], "dynamic_agent_client");
    assert_eq!(q["agent_name_hint"], "PlexusTyping");
    assert!(q["ext_agent_host_id"].starts_with("urn:uuid:"));
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["resource"], "https://api.openai.com/v1");
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(
        q["scope"],
        "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct"
    );
    let redirect = &q["redirect_uri"];
    assert!(redirect.starts_with("http://127.0.0.1:"), "{redirect}");
    assert!(redirect.ends_with("/auth/callback"));
    assert!(q["state"].len() >= 43 && q["nonce"].len() >= 43);
    let (status, page) = a.pages[0].clone().expect("the browser got an answer");
    assert_eq!(status, 200);
    assert!(page.contains("lang=\"es\""), "{page}");
    assert!(page.contains("Ya puedes cerrar esta pestaña"), "{page}");

    let calls = f.calls(TOKEN);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "POST");
    assert_eq!(
        calls[0].header("content-type"),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(calls[0].header("authorization"), None);
    let body = form(&calls[0]);
    assert_eq!(
        keys(&body),
        [
            "client_id",
            "code",
            "code_verifier",
            "grant_type",
            "redirect_uri",
            "resource"
        ],
        "no client secret"
    );
    assert_eq!(body["grant_type"], "authorization_code");
    assert_eq!(body["client_id"], CLIENT);
    assert_eq!(body["code"], CODE);
    assert_eq!(&body["redirect_uri"], redirect);
    assert_eq!(body["resource"], "https://api.openai.com/v1");
    assert_eq!(pkce_challenge(&body["code_verifier"]), q["code_challenge"]);
    assert!(body["code_verifier"].len() >= 43);

    let r = f.record();
    assert_eq!(r["hostId"], q["ext_agent_host_id"]);
    assert_eq!(r["clientId"], CLIENT);
    assert_eq!(r["subject"], SUB);
    assert_eq!(r["email"], EMAIL);
    assert_eq!(r["accessToken"], ACCESS);
    assert_eq!(r["refreshToken"], REFRESH);
    assert!(r["idToken"].as_str().unwrap().contains('.'));
    assert!(r["expiresAt"].as_u64().unwrap() > now() + 3000);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&f.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "owner-only file");
    }
    let leftovers: Vec<_> = std::fs::read_dir(f.path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, [CREDENTIALS_FILE], "no temporary files left");

    assert_eq!(f.auth.status().await.unwrap(), signed_in());
    assert!(f.auth.is_signed_in().await.unwrap());
    assert!(!f.auth.sign_in_pending());
    assert_eq!(
        format!("{:?}", f.auth.access_token().await.unwrap()),
        "ApiKey(***)"
    );
    assert_eq!(f.calls(TOKEN).len(), 1, "a fresh token is not refreshed");
    assert_eq!(f.calls(JWKS).len(), 1);
}

#[tokio::test]
async fn the_host_id_is_saved_before_the_first_sign_in_and_kept() {
    let f = Fixture::new().await;
    let a = f.sign_in(vec![Box::new(|_| None)]).await;
    let _ = a.result;
    let host = f.record()["hostId"].as_str().unwrap().to_string();
    assert!(host.starts_with("urn:uuid:"));
    let b = f.sign_in(vec![f.approve(3600, PLAN, true)]).await;
    b.result.clone().unwrap();
    assert_eq!(b.authorize(0)["ext_agent_host_id"], host);
    assert_eq!(f.record()["hostId"], host.as_str());
}

#[tokio::test]
async fn a_wrong_state_is_refused_and_the_right_one_still_signs_in() {
    let f = Fixture::new().await;
    let good = f.approve(3600, PLAN, true);
    let probe: Arc<Mutex<Option<(u16, String)>>> = Arc::default();
    let seen = probe.clone();
    let a = f
        .sign_in(vec![Box::new(move |p| {
            let forged = format!(
                "{}?code={CODE}&state=forged&client_id={CLIENT}",
                p["redirect_uri"]
            );
            let favicon = p["redirect_uri"].replace("/auth/callback", "/favicon.ico");
            let right = good(p).unwrap();
            let redirect = p["redirect_uri"].clone();
            tokio::spawn(async move {
                // A silent preconnect never holds up the callback.
                let addr = redirect
                    .trim_start_matches("http://")
                    .split('/')
                    .next()
                    .unwrap()
                    .to_string();
                let _idle = tokio::net::TcpStream::connect(&addr).await.unwrap();
                assert_eq!(visit(&favicon).await.unwrap().0, 404);
                *seen.lock().unwrap() = visit(&forged).await;
                visit(&format!("{redirect}?{right}")).await
            });
            None
        })])
        .await;
    assert_eq!(a.result.unwrap(), signed_in());
    let (status, page) = probe.lock().unwrap().clone().unwrap();
    assert_eq!(status, 400, "the forged callback is refused");
    assert!(page.contains("No se pudo iniciar sesión"), "{page}");
    assert_eq!(f.calls(TOKEN).len(), 1, "only the right code is exchanged");
    assert_eq!(form(&f.calls(TOKEN)[0])["code"], CODE);
}

#[tokio::test]
async fn only_wrong_states_time_out_without_an_exchange_and_close_the_listener() {
    let mut f = Fixture::new().await;
    f.auth = manager(&f.mock, &f.path).with_sign_in_timeout(Duration::from_millis(400));
    let addr: Arc<Mutex<Option<String>>> = Arc::default();
    let seen = addr.clone();
    let a = f
        .sign_in(vec![Box::new(move |p| {
            *seen.lock().unwrap() = Some(
                p["redirect_uri"]
                    .trim_start_matches("http://")
                    .split('/')
                    .next()
                    .unwrap()
                    .to_string(),
            );
            Some(format!("code={CODE}&state=forged&client_id={CLIENT}"))
        })])
        .await;
    let err = a.result.unwrap_err();
    assert_eq!(err, AuthError::Timeout);
    shown(&err);
    assert_eq!(a.pages[0].as_ref().unwrap().0, 400);
    assert!(f.calls(TOKEN).is_empty());
    assert!(!f.auth.sign_in_pending());
    let addr = addr.lock().unwrap().clone().unwrap();
    assert!(
        tokio::net::TcpStream::connect(&addr).await.is_err(),
        "the listener stops after the timeout"
    );
    assert_eq!(f.record()["accessToken"], Value::Null);
}

#[tokio::test]
async fn a_pending_sign_in_can_be_cancelled() {
    let f = Arc::new(Fixture::new().await);
    let waiting = f.clone();
    let task = tokio::spawn(async move { waiting.auth.sign_in(|_| Ok(())).await });
    for _ in 0..500 {
        if f.auth.sign_in_pending() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(f.auth.sign_in_pending());
    f.auth.cancel();
    assert_eq!(task.await.unwrap().unwrap_err(), AuthError::Cancelled);
    assert!(!f.auth.sign_in_pending());
}

#[tokio::test]
async fn a_cancel_before_the_sign_in_waits_for_the_browser_still_ends_it() {
    let f = Fixture::new().await;
    let auth = &f.auth;
    // Cancelled while the browser opens: no pending sign-in exists yet.
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        auth.sign_in(|_| {
            auth.cancel();
            Ok(())
        }),
    )
    .await;
    assert_eq!(
        result.expect("ended at once").unwrap_err(),
        AuthError::Cancelled
    );
    assert!(!f.auth.sign_in_pending());
    assert!(f.calls(TOKEN).is_empty());
}

#[tokio::test]
async fn a_sign_out_during_the_code_exchange_wins_over_its_answer() {
    let f = Arc::new(Fixture::new().await);
    let (mock, signer) = (f.mock.clone(), f.signer.clone());
    let slow: Step = Box::new(move |p| {
        let id = signer.sign(&claims(&mock.base, &p["nonce"]));
        mock.route(
            TOKEN,
            tokens(&id, ACCESS, REFRESH, 3600, PLAN).delayed(Duration::from_millis(500)),
        );
        Some(format!(
            "code={CODE}&state={}&client_id={CLIENT}&scope={}",
            p["state"],
            PLAN.replace(' ', "+")
        ))
    });
    let signing = f.clone();
    let task = tokio::spawn(async move { signing.sign_in(vec![slow]).await.result });
    for _ in 0..500 {
        if !f.calls(TOKEN).is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(f.calls(TOKEN).len(), 1, "the exchange is under way");

    f.auth.sign_out().await.unwrap();

    assert_eq!(task.await.unwrap().unwrap_err(), AuthError::Cancelled);
    let record = f.record();
    assert!(record["accessToken"].is_null(), "{record}");
    assert!(record["refreshToken"].is_null(), "{record}");
    assert!(!f.auth.status().await.unwrap().signed_in);
}

#[tokio::test]
async fn a_browser_that_does_not_open_leaves_no_pending_sign_in() {
    let f = Fixture::new().await;
    let err = f
        .auth
        .sign_in(|url| Err(format!("could not open {url}")))
        .await
        .unwrap_err();
    assert_eq!(err, AuthError::Browser);
    assert!(
        !shown(&err).contains("authorize"),
        "the URL is never echoed"
    );
    assert!(!f.auth.sign_in_pending());
}

#[tokio::test]
async fn the_preferred_port_falls_back_to_a_free_one() {
    let f = Fixture::new().await;
    let taken = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = taken.local_addr().unwrap().port();
    let auth = manager(&f.mock, &f.path).with_callback_port(port);
    let urls: Arc<Mutex<Vec<String>>> = Arc::default();
    let log = urls.clone();
    let sign_in = auth.sign_in(move |u| {
        log.lock().unwrap().push(u.to_string());
        Ok(())
    });
    let opened = async {
        while urls.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    // Saving the host id may take a while on a slow disk: wait for the browser, not a fixed time.
    tokio::select! {
        result = sign_in => panic!("the sign-in ended without a callback: {result:?}"),
        waited = tokio::time::timeout(Duration::from_secs(5), opened) => {
            assert!(waited.is_ok(), "the browser was never opened");
        }
    }
    let redirect = query(&urls.lock().unwrap()[0])["redirect_uri"].clone();
    assert!(redirect.starts_with("http://127.0.0.1:"));
    assert!(!redirect.contains(&format!(":{port}/")), "{redirect}");
}

#[tokio::test]
async fn declined_consent_exchanges_nothing() {
    let f = Fixture::new().await;
    let a = f
        .sign_in(vec![Box::new(|p| {
            Some(format!("error=access_denied&state={}", p["state"]))
        })])
        .await;
    assert_eq!(a.result.unwrap_err(), AuthError::AccessDenied);
    assert!(f.calls(TOKEN).is_empty());
    assert_eq!(f.record()["clientId"], Value::Null);
}

#[tokio::test]
async fn a_new_registration_without_an_issued_client_id_is_incomplete() {
    let f = Fixture::new().await;
    let a = f
        .sign_in(vec![Box::new(|p| {
            Some(format!("code={CODE}&state={}", p["state"]))
        })])
        .await;
    assert_eq!(a.result.unwrap_err(), AuthError::MissingClientId);
    let b = f
        .sign_in(vec![Box::new(|p| {
            Some(format!(
                "code={CODE}&state={}&client_id=dynamic_agent_client",
                p["state"]
            ))
        })])
        .await;
    assert_eq!(b.result.unwrap_err(), AuthError::MissingClientId);
    assert!(f.calls(TOKEN).is_empty());
}

#[tokio::test]
async fn reauthorization_reuses_the_client_with_both_hints_and_refuses_another_client() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    let id_token = f.record()["idToken"].as_str().unwrap().to_string();
    let host = f.record()["hostId"].as_str().unwrap().to_string();

    let a = f
        .sign_in(vec![Box::new(|p| {
            Some(format!(
                "code={CODE}&state={}&client_id=oaiapp_other",
                p["state"]
            ))
        })])
        .await;
    let q = a.authorize(0);
    assert_eq!(q["client_id"], CLIENT);
    assert_eq!(q["id_token_hint"], id_token);
    assert_eq!(q["login_hint"], EMAIL);
    assert_eq!(q["ext_agent_host_id"], host);
    assert!(!q.contains_key("agent_name_hint"));
    assert_eq!(a.result.unwrap_err(), AuthError::ClientIdMismatch);
    assert_eq!(
        f.calls(TOKEN).len(),
        1,
        "nothing exchanged for another client"
    );
    assert_eq!(f.record()["accessToken"], ACCESS);

    // On reauthorization the callback may omit the client id.
    let b = f.sign_in(vec![f.approve(3600, PLAN, false)]).await;
    assert_eq!(b.result.unwrap(), signed_in());
    assert_eq!(form(&f.calls(TOKEN)[1])["client_id"], CLIENT);
}

#[tokio::test]
async fn reauthorization_by_another_account_keeps_the_old_tokens() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    let a = f
        .sign_in(vec![f.approve_with(|s, mut c| {
            c["sub"] = json!("someone-else");
            s.sign(&c)
        })])
        .await;
    assert_eq!(a.result.unwrap_err(), AuthError::IdentityMismatch);
    assert_eq!(f.record()["accessToken"], ACCESS);
    assert_eq!(f.record()["subject"], SUB);
}

#[tokio::test]
async fn invalid_grant_at_the_code_exchange_starts_a_fresh_authorization() {
    let f = Fixture::new().await;
    let mock = f.mock.clone();
    let rejected: Step = Box::new(move |p| {
        let echo = json!({"error": "invalid_grant", "error_description": CODE});
        mock.route(TOKEN, Reply::status(400, &echo.to_string()));
        Some(format!(
            "code={CODE}&state={}&client_id={CLIENT}",
            p["state"]
        ))
    });
    let a = f
        .sign_in(vec![rejected, f.approve(3600, PLAN, false)])
        .await;
    assert_eq!(a.result.clone().unwrap(), signed_in());
    assert_eq!(a.urls.len(), 2, "the browser is opened again");
    let (first, second) = (a.authorize(0), a.authorize(1));
    assert_eq!(first["client_id"], "dynamic_agent_client");
    assert_eq!(
        second["client_id"], CLIENT,
        "the issued client id is retained"
    );
    assert!(!second.contains_key("agent_name_hint"));
    assert_ne!(first["state"], second["state"]);
    assert_ne!(first["nonce"], second["nonce"]);
    assert_ne!(first["code_challenge"], second["code_challenge"]);
    assert_eq!(first["ext_agent_host_id"], second["ext_agent_host_id"]);
    assert_eq!(f.calls(TOKEN).len(), 2);
}

#[tokio::test]
async fn invalid_grant_twice_ends_the_sign_in() {
    let f = Fixture::new().await;
    let reject = |f: &Fixture| -> Step {
        let mock = f.mock.clone();
        Box::new(move |p| {
            mock.route(TOKEN, Reply::status(400, r#"{"error":"invalid_grant"}"#));
            Some(format!(
                "code={CODE}&state={}&client_id={CLIENT}",
                p["state"]
            ))
        })
    };
    let a = f.sign_in(vec![reject(&f), reject(&f)]).await;
    let err = a.result.unwrap_err();
    assert_eq!(
        err,
        AuthError::CodeExchange {
            detail: "invalid_grant".into()
        }
    );
    shown(&err);
    assert_eq!(a.urls.len(), 2);
    assert_eq!(f.record()["accessToken"], Value::Null);
}

#[tokio::test]
async fn a_rejected_code_exchange_hides_what_the_server_echoed() {
    let f = Fixture::new().await;
    let mock = f.mock.clone();
    let a = f
        .sign_in(vec![Box::new(move |p| {
            let echo = json!({"error": "invalid_request",
                              "error_description": format!("bad {CODE} {REFRESH}")});
            mock.route(TOKEN, Reply::status(400, &echo.to_string()));
            Some(format!(
                "code={CODE}&state={}&client_id={CLIENT}",
                p["state"]
            ))
        })])
        .await;
    let err = a.result.unwrap_err();
    assert_eq!(
        err,
        AuthError::CodeExchange {
            detail: "invalid_request".into()
        }
    );
    shown(&err);
}

// --------------------------------------------------------------- ID token

#[tokio::test]
async fn id_tokens_that_fail_verification_save_nothing() {
    type Make = fn(&Signer, Value) -> String;
    let cases: [(&str, Make, &str); 9] = [
        ("wrong key", |_, c| Signer::new("kid-1").sign(&c), "firma"),
        (
            "tampered payload",
            |s, c| {
                let token = s.sign(&c);
                let parts: Vec<&str> = token.split('.').collect();
                let mut forged = c.clone();
                forged["sub"] = json!("attacker");
                format!(
                    "{}.{}.{}",
                    parts[0],
                    B64.encode(forged.to_string()),
                    parts[2]
                )
            },
            "firma",
        ),
        (
            "alg none",
            |_, c| {
                let header = json!({"alg": "none", "kid": "kid-1"});
                format!(
                    "{}.{}.",
                    B64.encode(header.to_string()),
                    B64.encode(c.to_string())
                )
            },
            "formato",
        ),
        (
            "HS256",
            |_, c| {
                let header = json!({"alg": "HS256", "typ": "JWT", "kid": "kid-1"});
                let msg = format!(
                    "{}.{}",
                    B64.encode(header.to_string()),
                    B64.encode(c.to_string())
                );
                let key = hmac::Key::new(hmac::HMAC_SHA256, b"public-key-bytes");
                let tag = hmac::sign(&key, msg.as_bytes());
                format!("{msg}.{}", B64.encode(tag.as_ref()))
            },
            "algoritmo",
        ),
        (
            "unknown kid",
            |_, c| Signer::new("kid-unknown").sign(&c),
            "clave de firma desconocida",
        ),
        (
            "nonce",
            |s, mut c| {
                c["nonce"] = json!("another-nonce");
                s.sign(&c)
            },
            "nonce",
        ),
        (
            "expired",
            |s, mut c| {
                c["exp"] = json!(now() - 600);
                c["iat"] = json!(now() - 4200);
                s.sign(&c)
            },
            "caducado",
        ),
        (
            "audience",
            |s, mut c| {
                c["aud"] = json!(["oaiapp_someone-else"]);
                s.sign(&c)
            },
            "audiencia",
        ),
        (
            "issuer",
            |s, mut c| {
                c["iss"] = json!("https://auth.openai.com.evil.example");
                s.sign(&c)
            },
            "emisor",
        ),
    ];
    for (name, make, reason) in cases {
        let f = Fixture::new().await;
        let a = f.sign_in(vec![f.approve_with(make)]).await;
        let err = a.result.unwrap_err();
        assert!(
            matches!(&err, AuthError::InvalidIdToken { reason: r } if r.contains(reason)),
            "{name}: {err:?}"
        );
        shown(&err);
        let r = f.record();
        assert_eq!(r["accessToken"], Value::Null, "{name}");
        assert_eq!(r["clientId"], Value::Null, "{name}");
        assert!(!f.auth.status().await.unwrap().signed_in, "{name}");
        if name == "unknown kid" {
            assert_eq!(
                f.calls(JWKS).len(),
                1,
                "a cold cache is fetched once, then refused"
            );
        }
    }
}

/// A reauthorization whose ID token is signed by `signer`.
fn reauth_signed_by(f: &Fixture, signer: Arc<Signer>) -> Step {
    let mock = f.mock.clone();
    Box::new(move |p| {
        let id = signer.sign(&claims(&mock.base, &p["nonce"]));
        mock.route(TOKEN, tokens(&id, ACCESS_2, REFRESH_2, 3600, PLAN));
        Some(format!("code={CODE}&state={}", p["state"]))
    })
}

#[tokio::test]
async fn a_rotated_signing_key_is_fetched_again_once_and_accepted() {
    let old = Arc::new(Signer::new("kid-1"));
    let rotated = Arc::new(Signer::new("kid-2"));
    let before = Reply::ok(json!({"keys": [old.jwk()]}));
    let after = Reply::ok(json!({"keys": [old.jwk(), rotated.jwk()]}));
    let f = Fixture::with(
        old,
        Some(Reply::ok(discovery("")).repeated()),
        vec![before, after],
    )
    .await;
    // The first sign-in caches the JWKS with kid-1 only.
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    assert_eq!(f.calls(JWKS).len(), 1);
    let a = f.sign_in(vec![reauth_signed_by(&f, rotated)]).await;
    assert_eq!(a.result.unwrap(), signed_in());
    assert_eq!(f.calls(JWKS).len(), 2, "one refetch for the new kid");
    assert_eq!(f.record()["accessToken"], ACCESS_2);
}

#[tokio::test]
async fn an_unknown_kid_is_refetched_once_then_refused() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    let before = f.record();
    let a = f
        .sign_in(vec![reauth_signed_by(
            &f,
            Arc::new(Signer::new("kid-unknown")),
        )])
        .await;
    let err = a.result.unwrap_err();
    assert!(
        matches!(&err, AuthError::InvalidIdToken { reason } if reason.contains("desconocida")),
        "{err:?}"
    );
    assert_eq!(
        f.calls(JWKS).len(),
        2,
        "cached once, refetched once, then refused"
    );
    assert_eq!(f.record(), before, "nothing replaced");
}

#[tokio::test]
async fn unreachable_keys_fail_the_sign_in_and_write_nothing() {
    let signer = Arc::new(Signer::new("kid-1"));
    let jwks_down = vec![Reply::status(503, "{}").repeated()];
    let cases: [(&str, Option<Reply>, Vec<Reply>); 3] = [
        (
            "JWKS down",
            Some(Reply::ok(discovery("")).repeated()),
            jwks_down,
        ),
        (
            "discovery down",
            Some(Reply::status(503, "{}").repeated()),
            Vec::new(),
        ),
        ("no discovery", None, Vec::new()),
    ];
    for (name, discovery_reply, jwks) in cases {
        let f = Fixture::with(signer.clone(), discovery_reply, jwks).await;
        // A record from an earlier session (host id only): it must stay byte for byte.
        f.write_record(json!({"hostId": "urn:uuid:00000000-0000-4000-8000-000000000000"}));
        let before = std::fs::read(&f.path).unwrap();
        let a = f.sign_in(vec![f.approve(3600, PLAN, true)]).await;
        let err = a.result.unwrap_err();
        assert_eq!(err, AuthError::KeysUnavailable, "{name}");
        assert!(shown(&err).contains("No se pudieron obtener las claves de OpenAI"));
        assert_eq!(
            std::fs::read(&f.path).unwrap(),
            before,
            "{name}: nothing written"
        );
        assert!(!f.auth.status().await.unwrap().signed_in, "{name}");
    }
}

#[tokio::test]
async fn a_discovery_without_rs256_is_refused() {
    let signer = Arc::new(Signer::new("kid-1"));
    let jwks = Reply::ok(json!({"keys": [signer.jwk()]})).repeated();
    let mut f = Fixture::with(signer, None, vec![jwks]).await;
    let mut doc = discovery(&f.mock.base);
    doc["id_token_signing_alg_values_supported"] = json!(["HS256"]);
    f.mock.route(DISCOVERY, Reply::ok(doc).repeated());
    f.auth = manager(&f.mock, &f.path);
    let a = f.sign_in(vec![f.approve(3600, PLAN, true)]).await;
    assert!(
        matches!(&a.result, Err(AuthError::InvalidIdToken { reason }) if reason.contains("algoritmo")),
        "{:?}",
        a.result
    );
    assert_eq!(f.record()["accessToken"], Value::Null);
}

// --------------------------------------------------------------- plan scope

#[tokio::test]
async fn a_grant_without_plan_use_stays_signed_in_but_gives_no_token() {
    let f = Fixture::new().await;
    let a = f.sign_in(vec![f.approve(3600, NO_PLAN, true)]).await;
    assert_eq!(
        a.result.unwrap(),
        Status {
            signed_in: true,
            email: Some(EMAIL.into()),
            plan_enabled: false
        }
    );
    assert!(!f.auth.is_signed_in().await.unwrap());
    let err = f.auth.access_token().await.unwrap_err();
    assert_eq!(err, AuthError::PlanNotEnabled);
    shown(&err);
    assert_eq!(f.calls(TOKEN).len(), 1, "no refresh, no inference token");
}

#[tokio::test]
async fn the_callback_scope_does_not_count_only_the_token_response() {
    let f = Fixture::new().await;
    let (mock, signer) = (f.mock.clone(), f.signer.clone());
    let a = f
        .sign_in(vec![Box::new(move |p| {
            let id = signer.sign(&claims(&mock.base, &p["nonce"]));
            mock.route(TOKEN, tokens(&id, ACCESS, REFRESH, 3600, NO_PLAN));
            Some(format!(
                "code={CODE}&state={}&client_id={CLIENT}&scope={}",
                p["state"],
                PLAN.replace(' ', "+")
            ))
        })])
        .await;
    assert!(!a.result.unwrap().plan_enabled);
}

// --------------------------------------------------------------- refresh

#[tokio::test]
async fn refresh_rotates_both_tokens_and_uses_the_latest_refresh_token() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    f.mock.route(TOKEN, refreshed(ACCESS_2, REFRESH_2, 0));
    f.mock.route(TOKEN, refreshed(ACCESS_3, REFRESH_3, 3600));

    f.auth.access_token().await.unwrap();
    let r = f.record();
    assert_eq!(r["accessToken"], ACCESS_2);
    assert_eq!(r["refreshToken"], REFRESH_2);
    f.auth.access_token().await.unwrap();
    assert_eq!(f.record()["accessToken"], ACCESS_3);
    assert_eq!(f.record()["refreshToken"], REFRESH_3);
    f.auth.access_token().await.unwrap();

    let calls = f.calls(TOKEN);
    assert_eq!(calls.len(), 3, "the third call reuses the fresh token");
    let first = form(&calls[1]);
    assert_eq!(
        keys(&first),
        ["client_id", "grant_type", "refresh_token", "resource"],
        "no scope, no secret"
    );
    assert_eq!(first["grant_type"], "refresh_token");
    assert_eq!(first["client_id"], CLIENT);
    assert_eq!(first["refresh_token"], REFRESH);
    assert_eq!(first["resource"], "https://api.openai.com/v1");
    assert_eq!(form(&calls[2])["refresh_token"], REFRESH_2);
}

#[tokio::test]
async fn concurrent_refreshes_send_one_request() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    f.mock.route(
        TOKEN,
        refreshed(ACCESS_2, REFRESH_2, 3600).delayed(Duration::from_millis(200)),
    );
    let (a, b, c) = tokio::join!(
        f.auth.access_token(),
        f.auth.access_token(),
        f.auth.access_token()
    );
    for got in [a, b, c] {
        got.unwrap();
    }
    assert_eq!(f.calls(TOKEN).len(), 2, "one exchange + one refresh");
    assert_eq!(f.record()["accessToken"], ACCESS_2);
}

#[tokio::test]
async fn a_dropped_request_still_saves_the_rotated_tokens() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    f.mock.route(
        TOKEN,
        refreshed(ACCESS_2, REFRESH_2, 3600).delayed(Duration::from_millis(200)),
    );
    let dropped = tokio::time::timeout(Duration::from_millis(50), f.auth.access_token()).await;
    assert!(dropped.is_err());
    // The renewal goes on in the background; a slow disk may take a while to save it.
    for _ in 0..500 {
        if f.record()["refreshToken"] == REFRESH_2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(f.record()["refreshToken"], REFRESH_2);
}

#[tokio::test]
async fn terminal_refresh_errors_forget_the_tokens_only() {
    for code in [
        "invalid_grant",
        "invalid_refresh_token",
        "token_expired",
        "refresh_token_expired",
        "refresh_token_invalidated",
        "refresh_token_reused",
    ] {
        let f = Fixture::new().await;
        f.sign_in(vec![f.approve(0, PLAN, true)])
            .await
            .result
            .unwrap();
        let echo = json!({"error": code, "error_description": REFRESH});
        f.mock.route(TOKEN, Reply::status(400, &echo.to_string()));
        let err = f.auth.access_token().await.unwrap_err();
        assert_eq!(err, AuthError::SessionExpired, "{code}");
        shown(&err);
        let r = f.record();
        assert_eq!(r["accessToken"], Value::Null, "{code}");
        assert_eq!(r["refreshToken"], Value::Null, "{code}");
        assert_eq!(r["idToken"], Value::Null, "{code}");
        assert_eq!(r["clientId"], CLIENT, "{code}");
        assert!(r["hostId"].as_str().unwrap().starts_with("urn:uuid:"));
        assert!(!f.auth.status().await.unwrap().signed_in);
    }
}

#[tokio::test]
async fn other_refresh_failures_keep_the_tokens() {
    let cases: [(u16, &str, AuthError); 3] = [
        (
            503,
            "{\"detail\":\"rt-SECRET-refresh-111\"}",
            AuthError::Unavailable { status: 503 },
        ),
        (
            401,
            r#"{"error":"invalid_client"}"#,
            AuthError::InvalidClient,
        ),
        (
            400,
            r#"{"error":"invalid_request"}"#,
            AuthError::RefreshRefused {
                detail: "invalid_request".into(),
            },
        ),
    ];
    for (status, body, expected) in cases {
        let f = Fixture::new().await;
        f.sign_in(vec![f.approve(0, PLAN, true)])
            .await
            .result
            .unwrap();
        f.mock.route(TOKEN, Reply::status(status, body));
        let err = f.auth.access_token().await.unwrap_err();
        assert_eq!(err, expected);
        shown(&err);
        assert_eq!(f.record()["refreshToken"], REFRESH);
    }
}

#[tokio::test]
async fn a_refresh_network_failure_keeps_the_tokens() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    let r = f.record();
    let dead = ChatgptAuth::new(
        &plexustyping_ai::Endpoints::loopback("http://127.0.0.1:9"),
        Some(f.path.clone()),
    );
    let err = dead.access_token().await.unwrap_err();
    assert!(matches!(err, AuthError::Network { .. }), "{err:?}");
    shown(&err);
    assert_eq!(f.record(), r);
}

#[tokio::test]
async fn a_refreshed_id_token_is_kept_only_when_it_verifies() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    let original = f.record()["idToken"].clone();
    let mut c = claims(&f.mock.base, "");
    c.as_object_mut().unwrap().remove("nonce");
    let forged = Signer::new("kid-1").sign(&c);
    f.mock.route(
        TOKEN,
        Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                         "id_token": forged, "expires_in": 0, "scope": PLAN})),
    );
    f.auth.access_token().await.unwrap();
    assert_eq!(
        f.record()["idToken"],
        original,
        "a bad signature is not kept"
    );
    assert_eq!(
        f.record()["refreshToken"],
        REFRESH_2,
        "the rotation is saved"
    );

    let good = f.signer.sign(&c);
    f.mock.route(
        TOKEN,
        Reply::ok(json!({"access_token": ACCESS_3, "refresh_token": REFRESH_3,
                         "id_token": good, "expires_in": 3600, "scope": PLAN})),
    );
    f.auth.access_token().await.unwrap();
    assert_eq!(f.record()["idToken"], good.as_str());
}

#[tokio::test]
async fn the_rotated_refresh_token_is_saved_before_the_id_token_is_checked() {
    let signer = Arc::new(Signer::new("kid-1"));
    let keys = json!({"keys": [signer.jwk()]});
    // The sign-in reads the keys at once; the refetch for an unknown kid is slow.
    let f = Fixture::with(
        signer,
        Some(Reply::ok(discovery("")).repeated()),
        vec![
            Reply::ok(keys.clone()),
            Reply::ok(keys).delayed(Duration::from_secs(3)).repeated(),
        ],
    )
    .await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    let mut c = claims(&f.mock.base, "");
    c.as_object_mut().unwrap().remove("nonce");
    f.mock.route(
        TOKEN,
        Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                         "id_token": Signer::new("kid-new").sign(&c),
                         "expires_in": 3600, "scope": PLAN})),
    );

    let gave_up = tokio::time::timeout(Duration::from_secs(1), f.auth.access_token()).await;

    assert!(gave_up.is_err(), "still checking the ID token");
    assert_eq!(f.record()["refreshToken"], REFRESH_2);
    assert_eq!(f.record()["accessToken"], ACCESS_2);
}

#[tokio::test]
async fn a_huge_expires_in_is_capped_instead_of_overflowing() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(u64::MAX, PLAN, true)])
        .await
        .result
        .unwrap();
    let day = 24 * 3600;
    let expires = f.record()["expiresAt"].as_u64().unwrap();
    assert!(expires > now() && expires <= now() + day, "{expires}");

    f.mock
        .route(TOKEN, refreshed(ACCESS_2, REFRESH_2, u64::MAX));
    let rejected = f.auth.access_token().await.unwrap();
    f.auth.refresh_rejected(&rejected).await.unwrap();
    assert_eq!(f.record()["accessToken"], ACCESS_2);
    let expires = f.record()["expiresAt"].as_u64().unwrap();
    assert!(expires > now() && expires <= now() + day, "{expires}");
    assert_eq!(f.record()["refreshToken"], REFRESH_2);
}

#[tokio::test]
async fn a_refresh_that_drops_plan_use_saves_the_tokens_and_gives_none() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(0, PLAN, true)])
        .await
        .result
        .unwrap();
    f.mock.route(
        TOKEN,
        Reply::ok(json!({"access_token": ACCESS_2, "refresh_token": REFRESH_2,
                         "expires_in": 3600, "scope": NO_PLAN})),
    );
    assert_eq!(
        f.auth.access_token().await.unwrap_err(),
        AuthError::PlanNotEnabled
    );
    assert_eq!(f.record()["refreshToken"], REFRESH_2);
    assert!(!f.auth.is_signed_in().await.unwrap());
}

// --------------------------------------------------------------- sign-out

#[tokio::test]
async fn sign_out_revokes_at_the_discovered_endpoint_and_keeps_the_registration() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    let host = f.record()["hostId"].clone();
    f.mock.route(REVOKE, Reply::status(200, ""));

    assert_eq!(f.auth.sign_out().await.unwrap(), SignOut { revoked: true });
    let calls = f.calls(REVOKE);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "POST");
    let body = form(&calls[0]);
    assert_eq!(keys(&body), ["client_id", "token", "token_type_hint"]);
    assert_eq!(body["token"], REFRESH);
    assert_eq!(body["token_type_hint"], "refresh_token");
    assert_eq!(body["client_id"], CLIENT);
    let r = f.record();
    for gone in ["accessToken", "refreshToken", "idToken"] {
        assert_eq!(r[gone], Value::Null, "{gone}");
    }
    assert_eq!(r["hostId"], host);
    assert_eq!(r["clientId"], CLIENT);
    assert_eq!(
        f.auth.status().await.unwrap(),
        Status {
            signed_in: false,
            email: None,
            plan_enabled: false
        }
    );
    assert_eq!(
        f.auth.access_token().await.unwrap_err(),
        AuthError::NotSignedIn
    );

    // The next sign-in reuses the client, with the email hint but no ID-token hint.
    let again = f.sign_in(vec![f.approve(3600, PLAN, false)]).await;
    again.result.clone().unwrap();
    let q = again.authorize(0);
    assert_eq!(q["client_id"], CLIENT);
    assert_eq!(q["ext_agent_host_id"], host.as_str().unwrap());
    assert_eq!(q["login_hint"], EMAIL);
    assert!(!q.contains_key("id_token_hint"));
    assert!(!q.contains_key("agent_name_hint"));
}

#[tokio::test]
async fn sign_out_retries_server_errors_then_clears_locally() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    for _ in 0..3 {
        f.mock.route(REVOKE, Reply::status(503, "{}"));
    }
    assert_eq!(f.auth.sign_out().await.unwrap(), SignOut { revoked: false });
    assert_eq!(f.calls(REVOKE).len(), 3);
    assert_eq!(f.record()["refreshToken"], Value::Null);
}

#[tokio::test]
async fn sign_out_never_sends_the_token_to_another_origin() {
    let mock = mock(Vec::new()).await;
    let evil = common::mock(Vec::new()).await;
    let mut doc = discovery(&mock.base);
    doc["revocation_endpoint"] = json!(format!("{}{REVOKE}", evil.base));
    mock.route(DISCOVERY, Reply::ok(doc));
    let dir = Scratch::new();
    let path = dir.0.join(CREDENTIALS_FILE);
    std::fs::write(
        &path,
        json!({"hostId": "urn:uuid:h", "clientId": CLIENT, "subject": SUB,
               "accessToken": ACCESS, "refreshToken": REFRESH,
               "scopes": PLAN.split(' ').collect::<Vec<_>>(), "expiresAt": now() + 3600})
        .to_string(),
    )
    .unwrap();
    let auth = manager(&mock, &path);
    assert_eq!(auth.sign_out().await.unwrap(), SignOut { revoked: false });
    assert!(evil.requests().is_empty());
    let r: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(r["refreshToken"], Value::Null);
}

#[tokio::test]
async fn a_damaged_file_is_reported_and_only_sign_out_resets_it() {
    let f = Fixture::new().await;
    f.write_record(json!({"hostId": 5}));
    assert_eq!(f.auth.status().await.unwrap_err(), AuthError::DamagedFile);
    std::fs::write(&f.path, "{not json").unwrap();
    assert_eq!(f.auth.status().await.unwrap_err(), AuthError::DamagedFile);
    assert_eq!(
        f.auth.access_token().await.unwrap_err(),
        AuthError::DamagedFile
    );
    let a = f.sign_in(vec![]).await;
    assert_eq!(a.result.unwrap_err(), AuthError::DamagedFile);
    assert!(a.urls.is_empty(), "no browser for a damaged file");
    assert_eq!(f.auth.sign_out().await.unwrap(), SignOut { revoked: false });
    assert!(!f.auth.status().await.unwrap().signed_in);
}

#[tokio::test]
async fn without_a_folder_nothing_is_stored() {
    let f = Fixture::new().await;
    let auth = ChatgptAuth::new(&f.mock.endpoints(), None);
    assert_eq!(auth.status().await.unwrap_err(), AuthError::NoStorage);
    assert_eq!(auth.access_token().await.unwrap_err(), AuthError::NoStorage);
    auth.set_path(Some(f.path.clone()));
    assert!(!auth.status().await.unwrap().signed_in);
}

// ---------------------------------------------------------------- leaks

#[tokio::test]
async fn no_token_appears_in_debug_output() {
    let f = Fixture::new().await;
    f.sign_in(vec![f.approve(3600, PLAN, true)])
        .await
        .result
        .unwrap();
    let token = f.auth.access_token().await.unwrap();
    let status = f.auth.status().await.unwrap();
    let text = format!(
        "{token:?} {:?} {:#?} {status:?} {}",
        f.auth,
        f.auth,
        serde_json::to_string(&status).unwrap()
    );
    assert_clean(&text);
    assert!(!text.contains(f.record()["idToken"].as_str().unwrap()));
    let _ = &f.dir;
}
