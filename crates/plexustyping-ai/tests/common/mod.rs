//! Shared test helpers: a loopback HTTP mock server and request builders.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use plexustyping_ai::{
    AiClient, ApiKey, Endpoints, GenerationRequest, Mode, ProviderConfig, ProviderId,
};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const KEY: &str = "sk-test-SECRET-key-123";

/// One request as received by the mock server.
#[derive(Clone, Debug)]
pub struct Captured {
    pub method: String,
    /// Path with query, e.g. `/v1/models?limit=1000`.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Captured {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("request body is JSON")
    }
}

/// A scripted reply.
#[derive(Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
    pub delay: Duration,
    /// Extra raw header lines, each without the trailing CRLF.
    pub headers: Vec<String>,
    /// The route keeps answering instead of being used up by one request.
    pub repeat: bool,
}

impl Reply {
    pub fn ok(body: Value) -> Self {
        Reply::status(200, &body.to_string())
    }

    pub fn status(status: u16, body: &str) -> Self {
        Reply {
            status,
            body: body.to_string(),
            delay: Duration::ZERO,
            headers: Vec::new(),
            repeat: false,
        }
    }

    /// The same reply for every matching request.
    pub fn repeated(mut self) -> Self {
        self.repeat = true;
        self
    }

    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn header(mut self, line: &str) -> Self {
        self.headers.push(line.to_string());
        self
    }
}

type Routes = Arc<Mutex<Vec<(String, Reply)>>>;

/// A running mock server; requests are logged in arrival order. Clones share it.
#[derive(Clone)]
pub struct Mock {
    pub base: String,
    log: Arc<Mutex<Vec<Captured>>>,
    routes: Routes,
}

impl Mock {
    /// Adds a route after start (e.g. a reply that depends on the request flow).
    pub fn route(&self, path: &str, reply: Reply) {
        self.routes.lock().unwrap().push((path.to_string(), reply));
    }

    pub fn requests(&self) -> Vec<Captured> {
        self.log.lock().unwrap().clone()
    }

    /// Requests whose path (without query) is `path`.
    pub fn hits(&self, path: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.path.split('?').next() == Some(path))
            .count()
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints::loopback(&self.base)
    }

    pub fn client(&self) -> AiClient {
        AiClient::new(self.endpoints())
    }
}

/// Serves on 127.0.0.1:0. Each request takes the first unused route whose path
/// (query ignored) matches; unmatched requests get 404.
pub async fn mock(routes: Vec<(&str, Reply)>) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log: Arc<Mutex<Vec<Captured>>> = Arc::default();
    let routes: Vec<(String, Reply)> = routes
        .into_iter()
        .map(|(p, r)| (p.to_string(), r))
        .collect();
    let routes: Routes = Arc::new(Mutex::new(routes));
    let task_routes = routes.clone();
    let task_log = log.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let log = task_log.clone();
            let routes = task_routes.clone();
            tokio::spawn(async move {
                let Some(captured) = read_request(&mut sock).await else {
                    return;
                };
                let path = captured.path.split('?').next().unwrap_or("").to_string();
                log.lock().unwrap().push(captured);
                let reply = {
                    let mut routes = routes.lock().unwrap();
                    match routes.iter().position(|(p, _)| *p == path) {
                        Some(i) if routes[i].1.repeat => routes[i].1.clone(),
                        Some(i) => routes.remove(i).1,
                        None => Reply::status(404, "{}"),
                    }
                };
                tokio::time::sleep(reply.delay).await;
                let mut extra = String::new();
                for line in &reply.headers {
                    extra.push_str(line);
                    extra.push_str("\r\n");
                }
                let resp = format!(
                    "HTTP/1.1 {} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}Connection: close\r\n\r\n{}",
                    reply.status,
                    reply.body.len(),
                    reply.body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    Mock { base, log, routes }
}

async fn read_request(sock: &mut TcpStream) -> Option<Captured> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        let n = sock.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    while buf.len() < head_end + len {
        let n = sock.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let end = buf.len().min(head_end + len);
    let body = String::from_utf8_lossy(&buf[head_end..end]).to_string();
    Some(Captured {
        method,
        path,
        headers,
        body,
    })
}

pub fn key() -> ApiKey {
    ApiKey::new(KEY).unwrap()
}

pub fn config(id: ProviderId, model: &str) -> ProviderConfig {
    let mut cfg = ProviderConfig::new(id, key());
    cfg.model = model.into();
    cfg.timeout = Duration::from_secs(5);
    cfg
}

/// A small Spanish request whose limits suit short hand-written texts.
pub fn small_request() -> GenerationRequest {
    GenerationRequest {
        language: "es".into(),
        mode: Mode::Prose,
        targets: vec!["z".into()],
        frequencies: Vec::new(),
        density: 3.0,
        length: 5..60,
        allowed_chars: "abcdefghijklmnñopqrstuvwxyzABCDEFGHIJKLMNÑOPQRSTUVWXYZ.,".into(),
        restrict_chars: false,
        topic: None,
        continue_from: None,
    }
}

/// `{"text": text}` as a model would answer it.
pub fn text_json(text: &str) -> String {
    serde_json::json!({ "text": text }).to_string()
}

/// A Chat Completions reply whose content is `content`.
pub fn chat_reply(content: &str) -> Reply {
    Reply::ok(serde_json::json!({
        "choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 80, "completion_tokens": 30}
    }))
}
