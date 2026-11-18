//! A loopback HTTP mock of the AI providers for tests (never a real provider).

use std::sync::{Arc, Mutex};

use plexustyping_ai::{AiClient, Endpoints};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One received request: path (with query) and body.
#[derive(Clone, Debug)]
pub struct Captured {
    pub path: String,
    pub body: String,
}

/// A running mock; requests are logged in arrival order.
pub struct Mock {
    base: String,
    log: Arc<Mutex<Vec<Captured>>>,
}

impl Mock {
    pub fn requests(&self) -> Vec<Captured> {
        self.log.lock().unwrap().clone()
    }

    pub fn endpoints(&self) -> Endpoints {
        Endpoints::loopback(&self.base)
    }

    pub fn client(&self) -> AiClient {
        AiClient::new(self.endpoints())
    }
}

/// Serves on 127.0.0.1:0. Each request takes the first unused `(path, status,
/// body)` route whose path (query ignored) matches; others get 404.
pub async fn mock(routes: Vec<(&str, u16, String)>) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log: Arc<Mutex<Vec<Captured>>> = Arc::default();
    let routes: Vec<(String, u16, String)> = routes
        .into_iter()
        .map(|(p, s, b)| (p.to_owned(), s, b))
        .collect();
    let routes = Arc::new(Mutex::new(routes));
    let task_log = log.clone();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let (log, routes) = (task_log.clone(), routes.clone());
            tokio::spawn(async move {
                let Some(captured) = read_request(&mut sock).await else {
                    return;
                };
                let path = captured.path.split('?').next().unwrap_or("").to_owned();
                log.lock().unwrap().push(captured);
                let (status, body) = {
                    let mut routes = routes.lock().unwrap();
                    match routes.iter().position(|(p, ..)| *p == path) {
                        Some(i) => {
                            let (_, status, body) = routes.remove(i);
                            (status, body)
                        }
                        None => (404, "{}".to_owned()),
                    }
                };
                let resp = format!(
                    "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    Mock { base, log }
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
    let path = head.split(' ').nth(1)?.to_owned();
    let len: usize = head
        .split("\r\n")
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
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
    Some(Captured { path, body })
}
