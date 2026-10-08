// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! A very small HTTP/1.1 request reader and response writer.
//!
//! Only what a browser extension and the panel need: a request line, headers,
//! an optional `Content-Length` body, and a `Connection: close` response.

use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::TcpStream;

use crate::json::Json;

/// Request line + headers must fit in this; a browser sends well under 1 KiB.
const MAX_HEAD: usize = 16 * 1024;
const READ_CHUNK: usize = 4096;

pub struct Request {
    pub method: String,
    pub path: String,
    /// Raw query string, without the `?`.
    pub query: String,
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

    /// One query-string parameter, percent-decoded.
    pub fn param(&self, name: &str) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(k) == name).then(|| decode(v))
        })
    }
}

/// A request we refused before routing; `status` is what to send back.
pub struct RequestError {
    pub status: u16,
    pub detail: String,
}

impl RequestError {
    fn new(status: u16, detail: impl Into<String>) -> Self {
        RequestError {
            status,
            detail: detail.into(),
        }
    }
}

impl From<io::Error> for RequestError {
    fn from(e: io::Error) -> Self {
        RequestError::new(400, format!("read error: {e}"))
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
            return Err(RequestError::new(431, "headers too large"));
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(RequestError::new(
                400,
                "connection closed before headers ended",
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");

    let start = lines
        .next()
        .ok_or_else(|| RequestError::new(400, "empty request"))?;
    let mut parts = start.split(' ');
    let method = parts
        .next()
        .filter(|m| !m.is_empty())
        .ok_or_else(|| RequestError::new(400, "missing method"))?
        .to_ascii_uppercase();
    let target = parts
        .next()
        .ok_or_else(|| RequestError::new(400, "missing request target"))?;
    let target = target.split('#').next().unwrap_or("");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let (path, query) = (path.to_string(), query.to_string());

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            return Err(RequestError::new(400, "malformed header"));
        };
        headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
    }

    let mut req = Request {
        method,
        path,
        query,
        headers,
        body: Vec::new(),
    };

    if req.header("transfer-encoding").is_some() {
        return Err(RequestError::new(
            411,
            "chunked bodies are not supported; send Content-Length",
        ));
    }

    let len = match req.header("content-length") {
        None => 0,
        Some(v) => v
            .trim()
            .parse::<usize>()
            .map_err(|_| RequestError::new(400, "invalid Content-Length"))?,
    };
    if len > max_body {
        return Err(RequestError::new(
            413,
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

/// Only browser extensions, the panel this daemon serves itself, and
/// non-browser clients (curl, scripts, which send no `Origin`) may talk to it.
/// Without this, any web page you visit could quietly append lines to your
/// links file -- or, now that there is a panel, start a download.
pub fn cors_for(origin: Option<&str>, port: u16) -> Cors {
    match origin {
        None => Cors::Allow(None),
        Some(o) if is_extension(o) || is_panel(o, port) => Cors::Allow(Some(o.to_string())),
        Some(_) => Cors::Deny,
    }
}

pub fn is_extension(origin: &str) -> bool {
    origin.starts_with("chrome-extension://")
        || origin.starts_with("moz-extension://")
        || origin.starts_with("safari-web-extension://")
}

/// The panel's own origin, under any of the names loopback goes by. Another
/// port is another origin: a dev server on localhost:3000 is not the panel.
fn is_panel(origin: &str, port: u16) -> bool {
    ["127.0.0.1", "localhost", "[::1]"]
        .iter()
        .any(|host| origin.eq_ignore_ascii_case(&format!("http://{host}:{port}")))
}

/// True when the `Host` header names this machine's loopback interface.
///
/// The panel and its API check this. Without it a page on evil.example could
/// re-resolve its own name to 127.0.0.1 (DNS rebinding) and read the panel as
/// if it were same-origin -- the browser would see nothing cross-site about it.
pub fn host_is_local(host: Option<&str>) -> bool {
    let Some(host) = host else { return false };
    let name = match host.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((inner, _)) => inner,
            None => return false,
        },
        None => match host.rsplit_once(':') {
            Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
            _ => host,
        },
    };
    name.eq_ignore_ascii_case("localhost") || name == "127.0.0.1" || name == "::1"
}

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Cow<'static, [u8]>,
    /// Extra headers, already formatted as `Name: value`.
    pub extra: Vec<String>,
    /// A body streamed from disk instead: the file, already at its first
    /// byte, and how many bytes to send. A video never sits in memory whole.
    pub file: Option<(File, u64)>,
}

impl Response {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Response {
            status,
            content_type: "text/plain; charset=utf-8",
            body: Cow::Owned(body.into().into_bytes()),
            extra: Vec::new(),
            file: None,
        }
    }

    pub fn json(status: u16, body: &Json) -> Self {
        Response {
            status,
            content_type: "application/json; charset=utf-8",
            body: Cow::Owned(body.to_string().into_bytes()),
            extra: Vec::new(),
            file: None,
        }
    }

    /// A file compiled into the binary.
    pub fn asset(content_type: &'static str, body: &'static [u8]) -> Self {
        Response {
            status: 200,
            content_type,
            body: Cow::Borrowed(body),
            extra: Vec::new(),
            file: None,
        }
    }

    /// `len` bytes of `file`, read from where it is positioned now.
    pub fn stream(status: u16, content_type: &'static str, file: File, len: u64) -> Self {
        Response {
            status,
            content_type,
            body: Cow::Borrowed(b""),
            extra: Vec::new(),
            file: Some((file, len)),
        }
    }
}

pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        206 => "Partial Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        421 => "Misdirected Request",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// `head_only` answers a HEAD request: same headers, no body.
pub fn write_response(
    stream: &mut TcpStream,
    res: &Response,
    cors: &Cors,
    head_only: bool,
) -> io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n",
        res.status,
        reason(res.status),
        res.file
            .as_ref()
            .map_or(res.body.len() as u64, |(_, len)| *len)
    );
    if !res.body.is_empty() || res.file.is_some() {
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
    if !head_only {
        out.extend_from_slice(&res.body);
    }
    stream.write_all(&out)?;
    if let (Some((file, len)), false) = (&res.file, head_only) {
        io::copy(&mut file.take(*len), stream)?;
    }
    stream.flush()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Percent-decoding for query strings, `+` included.
pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' => match b.get(i + 1..i + 3) {
                Some(hex) if hex.iter().all(u8::is_ascii_hexdigit) => {
                    let s = std::str::from_utf8(hex).unwrap_or("0");
                    out.push(u8::from_str_radix(s, 16).unwrap_or(b'?'));
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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
    fn cors_policy_allows_only_extensions_the_panel_and_cli() {
        assert!(matches!(cors_for(None, 9876), Cors::Allow(None)));
        assert!(matches!(
            cors_for(Some("chrome-extension://abcdef"), 9876),
            Cors::Allow(Some(_))
        ));
        assert!(matches!(
            cors_for(Some("http://127.0.0.1:9876"), 9876),
            Cors::Allow(Some(_))
        ));
        assert!(matches!(
            cors_for(Some("http://localhost:9876"), 9876),
            Cors::Allow(Some(_))
        ));
        assert!(matches!(cors_for(Some("https://x.com"), 9876), Cors::Deny));
        assert!(matches!(cors_for(Some("null"), 9876), Cors::Deny));
        assert!(matches!(
            cors_for(Some("http://localhost:3000"), 9876),
            Cors::Deny
        ));
        assert!(matches!(
            cors_for(Some("http://127.0.0.1:9876.evil.tld"), 9876),
            Cors::Deny
        ));
    }

    #[test]
    fn host_check_rejects_rebound_names() {
        for ok in [
            "127.0.0.1:9876",
            "localhost:9876",
            "LOCALHOST",
            "[::1]:9876",
            "127.0.0.1",
        ] {
            assert!(host_is_local(Some(ok)), "{ok}");
        }
        for bad in [
            "evil.example:9876",
            "127.0.0.1.nip.io:9876",
            "localhost.evil.tld",
            "[::2]:9876",
            "[::1",
            "",
        ] {
            assert!(!host_is_local(Some(bad)), "{bad}");
        }
        assert!(!host_is_local(None));
    }

    #[test]
    fn decodes_query_parameters() {
        let req = Request {
            method: "GET".into(),
            path: "/api/links".into(),
            query: "q=a%20b+c&limit=5&empty=&bad=%zz%4".into(),
            headers: Vec::new(),
            body: Vec::new(),
        };
        assert_eq!(req.param("q").as_deref(), Some("a b c"));
        assert_eq!(req.param("limit").as_deref(), Some("5"));
        assert_eq!(req.param("empty").as_deref(), Some(""));
        assert_eq!(req.param("bad").as_deref(), Some("%zz%4"));
        assert_eq!(req.param("missing"), None);
    }
}
