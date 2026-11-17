//! The one-shot loopback listener that receives the browser's OAuth callback.

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{AuthError, CALLBACK_PATH, parse_query};

/// A connection must send its request line within this time.
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REQUEST: usize = 16 * 1024;

const DONE_PAGE: &str = "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><title>PlexusTyping</title><body style=\"font-family:sans-serif;margin:3em\"><h1>PlexusTyping</h1><p>Ya puedes cerrar esta pestaña y volver a PlexusTyping.</p></body></html>";
const REJECTED_PAGE: &str = "<!doctype html><html lang=\"es\"><meta charset=\"utf-8\"><title>PlexusTyping</title><body style=\"font-family:sans-serif;margin:3em\"><h1>PlexusTyping</h1><p>No se pudo iniciar sesión con esta respuesta. Vuelve a PlexusTyping e inténtalo de nuevo.</p></body></html>";

/// Binds `127.0.0.1:port`, or any free loopback port when it is taken. Never
/// another interface.
pub(super) async fn bind(port: u16) -> Result<TcpListener, AuthError> {
    match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => Ok(listener),
        Err(_) => TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|err| AuthError::Listener {
                message: err.to_string(),
            }),
    }
}

/// Waits for the browser's callback with `state`; other requests are answered and ignored.
pub(super) async fn wait(
    listener: &TcpListener,
    state: &str,
) -> Result<HashMap<String, String>, AuthError> {
    // Each connection is read on its own task, so one that sends nothing (a browser
    // preconnect) never holds up the callback.
    let (found_tx, mut found) = tokio::sync::mpsc::channel(1);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (socket, _) = accepted.map_err(|err| AuthError::Listener {
                    message: err.to_string(),
                })?;
                tokio::spawn(answer(socket, state.to_string(), found_tx.clone()));
            }
            Some(params) = found.recv() => return Ok(params),
        }
    }
}

/// Equal-time comparison, so `state` cannot be guessed byte by byte.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

/// Answers one connection; sends the parameters of the right callback.
async fn answer(
    mut socket: TcpStream,
    state: String,
    found: tokio::sync::mpsc::Sender<HashMap<String, String>>,
) {
    let Ok(Some(target)) = tokio::time::timeout(READ_TIMEOUT, read_target(&mut socket)).await
    else {
        return;
    };
    let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
    if path != CALLBACK_PATH {
        reply(&mut socket, "404 Not Found", "").await;
        return;
    }
    let params = parse_query(query);
    if !params.get("state").is_some_and(|s| same(s, &state)) {
        reply(&mut socket, "400 Bad Request", REJECTED_PAGE).await;
        return;
    }
    reply(&mut socket, "200 OK", DONE_PAGE).await;
    let _ = found.send(params).await;
}

/// The request target of a `GET` request line.
async fn read_target(socket: &mut TcpStream) -> Option<String> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 2048];
    while !raw.windows(2).any(|w| w == b"\r\n") {
        let n = socket.read(&mut buf).await.ok()?;
        if n == 0 || raw.len() + n > MAX_REQUEST {
            return None;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    let line = String::from_utf8_lossy(&raw);
    let mut parts = line.lines().next()?.split(' ');
    (parts.next()? == "GET").then_some(())?;
    parts.next().map(str::to_string)
}

async fn reply(socket: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
}
