// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! A very small HTTP/1.1 request reader and response writer.
//!
//! Only what a browser extension posting a URL needs: a request line, headers,
//! an optional `Content-Length` body, and a `Connection: close` response.

use std::io::{self, Read, Write};
use std::net::TcpStream;

/// Request line + headers must fit in this; a browser sends well under 1 KiB.
const MAX_HEAD: usize = 16 * 1024;
const READ_CHUNK: usize = 4096;

pub struct Request {
    pub method: String,
    pub path: String,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A request we refused before routing; `status` is what to send back.
pub struct RequestError {
    pub status: u16,
    pub reason: &'static str,
    pub detail: String,
}

impl RequestError {
    fn new(status: u16, reason: &'static str, detail: impl Into<String>) -> Self {
        RequestError {
            status,
            reason,
            detail: detail.into(),
        }
    }
}

impl From<io::Error> for RequestError {
    fn from(e: io::Error) -> Self {
        RequestError::new(400, "Bad Request", format!("read error: {e}"))
    }
}

pub fn read_request(stream: &mut TcpStream, max_body: usize) -> Result<Request, RequestError> {
    let mut buf: Vec<u8> = Vec::with_capacity(READ_CHUNK);
    let mut chunk = [0u8; READ_CHUNK];
    let mut searched = 0usize;

    let head_end = loop {
        if let Some(p) = find(&buf[searched..], b"\r\n\r\n") {
            break searched + p;
        }
        searched = buf.len().saturating_sub(3);
        if buf.len() > MAX_HEAD {
            return Err(RequestError::new(
                431,
                "Request Header Fields Too Large",
                "headers too large",
            ));
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(RequestError::new(
                400,
                "Bad Request",
                "connection closed before headers ended",
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");

    let start = lines
        .next()
        .ok_or_else(|| RequestError::new(400, "Bad Request", "empty request"))?;
    let mut parts = start.split(' ');
    let method = parts
        .next()
        .filter(|m| !m.is_empty())
        .ok_or_else(|| RequestError::new(400, "Bad Request", "missing method"))?
        .to_ascii_uppercase();
    let target = parts
        .next()
        .ok_or_else(|| RequestError::new(400, "Bad Request", "missing request target"))?;
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(RequestError::new(400, "Bad Request", "malformed header"));
        };
        headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
    }

    let mut req = Request {
        method,
        path,
        headers,
        body: Vec::new(),
    };

    if req.header("transfer-encoding").is_some() {
        return Err(RequestError::new(
            411,
            "Length Required",
            "chunked bodies are not supported; send Content-Length",
        ));
    }

    let len = match req.header("content-length") {
        None => 0,
        Some(v) => v
            .trim()
            .parse::<usize>()
            .map_err(|_| RequestError::new(400, "Bad Request", "invalid Content-Length"))?,
    };
    if len > max_body {
        return Err(RequestError::new(
            413,
            "Payload Too Large",
            format!("body of {len} bytes exceeds the {max_body} byte limit"),
        ));
    }

    let mut body = buf.split_off((head_end + 4).min(buf.len()));
    body.truncate(len);
    if body.len() < len {
        let mut rest = vec![0u8; len - body.len()];
        stream.read_exact(&mut rest)?;
        body.extend_from_slice(&rest);
    }
    req.body = body;
    Ok(req)
}

/// What CORS headers a response should carry, decided from the `Origin` header.
pub enum Cors {
    /// `Some(origin)` echoes that origin back; `None` sends `*`.
    Allow(Option<String>),
    /// A regular web page tried to talk to us.
    Deny,
}

/// Only browser extensions and non-browser clients (curl, scripts, which send
/// no `Origin`) may write. Without this, any web page you visit could quietly
/// append lines to your links file.
pub fn cors_for(origin: Option<&str>) -> Cors {
    match origin {
        None => Cors::Allow(None),
        Some(o)
            if o.starts_with("chrome-extension://")
                || o.starts_with("moz-extension://")
                || o.starts_with("safari-web-extension://") =>
        {
            Cors::Allow(Some(o.to_string()))
        }
        Some(_) => Cors::Deny,
    }
}

pub struct Response<'a> {
    pub status: u16,
    pub reason: &'a str,
    pub content_type: &'a str,
    pub body: &'a [u8],
    /// Extra headers, already formatted as `Name: value`.
    pub extra: Vec<String>,
}

impl<'a> Response<'a> {
    pub fn text(status: u16, reason: &'a str, body: &'a str) -> Self {
        Response {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            body: body.as_bytes(),
            extra: Vec::new(),
        }
    }

    pub fn json(status: u16, reason: &'a str, body: &'a str) -> Self {
        Response {
            status,
            reason,
            content_type: "application/json; charset=utf-8",
            body: body.as_bytes(),
            extra: Vec::new(),
        }
    }
}

pub fn write_response(stream: &mut TcpStream, res: &Response<'_>, cors: &Cors) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
        res.status,
        res.reason,
        res.body.len()
    );
    if !res.body.is_empty() {
        head.push_str(&format!("Content-Type: {}\r\n", res.content_type));
    }
    match cors {
        Cors::Allow(Some(origin)) => {
            head.push_str(&format!("Access-Control-Allow-Origin: {origin}\r\n"));
            head.push_str("Vary: Origin\r\n");
        }
        Cors::Allow(None) => head.push_str("Access-Control-Allow-Origin: *\r\n"),
        Cors::Deny => head.push_str("Vary: Origin\r\n"),
    }
    for h in &res.extra {
        head.push_str(h);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    // One write for the head, one for the body: fewer packets, and the browser
    // sees the whole response in a single read.
    let mut out = head.into_bytes();
    out.extend_from_slice(res.body);
    stream.write_all(&out)?;
    stream.flush()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_header_terminator() {
        assert_eq!(find(b"abc\r\n\r\ndef", b"\r\n\r\n"), Some(3));
        assert_eq!(find(b"abc", b"\r\n\r\n"), None);
        assert_eq!(find(b"", b"x"), None);
    }

    #[test]
    fn cors_policy_allows_only_extensions_and_cli() {
        assert!(matches!(cors_for(None), Cors::Allow(None)));
        assert!(matches!(
            cors_for(Some("chrome-extension://abcdef")),
            Cors::Allow(Some(_))
        ));
        assert!(matches!(cors_for(Some("https://x.com")), Cors::Deny));
        assert!(matches!(cors_for(Some("null")), Cors::Deny));
        assert!(matches!(
            cors_for(Some("http://localhost:3000")),
            Cors::Deny
        ));
    }
}
