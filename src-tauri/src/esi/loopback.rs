//! A short-lived, loopback-only HTTP listener that receives exactly one
//! OAuth redirect from the system browser, validates `state`, and
//! returns the authorization `code`. Per docs/ESI_INTEGRATION.md §2
//! step 3. Deliberately minimal — no HTTP server crate, just enough raw
//! TCP + string parsing to handle one GET request shaped like
//! `GET /callback?code=...&state=...#_ HTTP/1.1`.
//!
//! Binds only to 127.0.0.1, never 0.0.0.0 — this must never be reachable
//! from outside the local machine.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

pub struct CallbackResult {
    pub code: String,
}

/// Binds to `127.0.0.1:{port}`, accepts one connection, and blocks until
/// either a valid callback arrives or `timeout` elapses. `expected_state`
/// must match the `state` query parameter exactly, or the callback is
/// refused — this is the CSRF protection for the whole flow.
pub fn listen_for_callback(port: u16, expected_state: &str, timeout: Duration) -> Result<CallbackResult, String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("failed to bind loopback listener on 127.0.0.1:{port}: {e}"))?;
    listener
        .set_nonblocking(false)
        .map_err(|e| format!("failed to configure listener: {e}"))?;

    let (mut stream, _addr) = accept_with_timeout(&listener, timeout)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| format!("failed to set read timeout: {e}"))?;

    let request_line = read_request_line(&mut stream)?;
    let query = extract_query_string(&request_line)?;
    let params = parse_query_params(&query);

    let state = params.get("state").cloned().unwrap_or_default();
    if state != expected_state {
        respond(&mut stream, 400, "Login could not be verified (state mismatch). Close this window and try again.");
        return Err("callback state did not match the value sent in the authorize request — possible CSRF or stale redirect".into());
    }

    if let Some(err) = params.get("error") {
        let desc = params.get("error_description").cloned().unwrap_or_default();
        respond(&mut stream, 400, "Login was not completed. Close this window and try again.");
        return Err(format!("EVE SSO returned an error: {err} — {desc}"));
    }

    let code = params
        .get("code")
        .cloned()
        .ok_or_else(|| "callback had no 'code' parameter".to_string())?;

    respond(&mut stream, 200, "Login successful — you can close this window and return to RenderNorth Industrial.");
    Ok(CallbackResult { code })
}

fn accept_with_timeout(listener: &TcpListener, timeout: Duration) -> Result<(TcpStream, std::net::SocketAddr), String> {
    // std::net::TcpListener has no built-in accept timeout; a short poll
    // loop on a listener already in blocking mode is the simplest
    // correct way to bound the wait without pulling in an async runtime
    // for one connection.
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to set non-blocking mode: {e}"))?;
    let start = std::time::Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                return Ok((stream, addr));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() > timeout {
                    return Err("timed out waiting for the browser to redirect back — login was not completed in time".into());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("failed to accept loopback connection: {e}")),
        }
    }
}

fn read_request_line(stream: &mut TcpStream) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).map_err(|e| format!("failed reading callback request: {e}"))?;
        if n == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
        if buf.len() > 8192 {
            return Err("callback request line was unexpectedly large — refusing".into());
        }
    }
    Ok(String::from_utf8_lossy(&buf).trim_end_matches('\r').to_string())
}

fn extract_query_string(request_line: &str) -> Result<String, String> {
    // Expected shape: "GET /callback?code=...&state=... HTTP/1.1"
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    if method != "GET" {
        return Err(format!("unexpected HTTP method on callback: '{method}'"));
    }
    let path_and_query = parts.next().unwrap_or("");
    match path_and_query.split_once('?') {
        Some((_path, query)) => Ok(query.to_string()),
        None => Err("callback request had no query string".into()),
    }
}

fn parse_query_params(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            Some((url_decode(k), url_decode(v)))
        })
        .collect()
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn respond(stream: &mut TcpStream, status: u16, message: &str) {
    let status_text = if status == 200 { "OK" } else { "Bad Request" };
    let body = format!(
        "<html><body style=\"font-family:sans-serif;padding:2rem;\"><h2>{message}</h2></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;

    #[test]
    fn valid_callback_with_matching_state_returns_the_code() {
        let port = 38471;
        let expected_state = "test-state-abc";
        let handle = thread::spawn(move || listen_for_callback(port, expected_state, Duration::from_secs(5)));

        // Give the listener a moment to bind before the "browser" connects.
        thread::sleep(Duration::from_millis(200));
        let mut client = TcpStream::connect(("127.0.0.1", port)).expect("client should connect");
        client
            .write_all(b"GET /callback?code=abc123&state=test-state-abc HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("200 OK"), "response was: {response}");

        let result = handle.join().unwrap().expect("listener should succeed");
        assert_eq!(result.code, "abc123");
    }

    #[test]
    fn mismatched_state_is_refused() {
        let port = 38472;
        let handle = thread::spawn(move || listen_for_callback(port, "expected-state", Duration::from_secs(5)));

        thread::sleep(Duration::from_millis(200));
        let mut client = TcpStream::connect(("127.0.0.1", port)).expect("client should connect");
        client
            .write_all(b"GET /callback?code=abc123&state=WRONG-state HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("400"), "response was: {response}");

        let result = handle.join().unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("state"));
    }

    #[test]
    fn sso_error_response_is_surfaced() {
        let port = 38473;
        let handle = thread::spawn(move || listen_for_callback(port, "s", Duration::from_secs(5)));

        thread::sleep(Duration::from_millis(200));
        let mut client = TcpStream::connect(("127.0.0.1", port)).expect("client should connect");
        client
            .write_all(b"GET /callback?error=access_denied&error_description=User+declined&state=s HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("400"));

        let result = handle.join().unwrap();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("access_denied"));
        assert!(err.contains("User declined"), "url-decoded '+' should become a space: {err}");
    }

    #[test]
    fn timeout_fires_if_nothing_ever_connects() {
        let port = 38474;
        let result = listen_for_callback(port, "s", Duration::from_millis(500));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn url_decode_handles_percent_and_plus_encoding() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a+b+c"), "a b c");
        assert_eq!(url_decode("esi-assets.read_assets.v1"), "esi-assets.read_assets.v1");
    }
}
