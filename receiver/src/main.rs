// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! x-link-receiver -- a loopback HTTP endpoint that appends links to a file.
//!
//! Shape of the thing: one thread parks in `poll()` on the listening socket and
//! hands each accepted connection to a short-lived worker thread. There is no
//! async runtime and no dependencies; idle cost is one blocked thread.

#[macro_use]
mod log;

mod config;
mod http;
mod json;
mod platform;
mod store;
mod url;

use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use store::Store;

/// Bounds worker threads so a misbehaving client cannot spawn without limit.
static INFLIGHT: AtomicUsize = AtomicUsize::new(0);
const MAX_INFLIGHT: usize = 64;

/// How long a single client may take to send its request / read its response.
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Accept-loop tick; also the worst-case delay before a shutdown is noticed.
const POLL_INTERVAL_MS: i32 = 250;

type Shared = Arc<Mutex<Store>>;

fn main() {
    let cfg = match config::Config::from_args() {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return, // --help / --version already printed
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };
    log::QUIET.store(cfg.quiet, Ordering::Relaxed);

    let store = match Store::open(cfg.file.clone(), cfg.fsync) {
        Ok(s) => s,
        Err(e) => {
            elog!("cannot use {}: {e}", cfg.file.display());
            std::process::exit(1);
        }
    };
    let file_path = store.path().display().to_string();
    let existing = store.total();

    let listener = match TcpListener::bind(&cfg.addr) {
        Ok(l) => l,
        Err(e) => {
            let hint = if e.kind() == io::ErrorKind::AddrInUse {
                " (already running?)"
            } else {
                ""
            };
            elog!("cannot bind {}: {e}{hint}", cfg.addr);
            std::process::exit(1);
        }
    };
    if let Err(e) = listener.set_nonblocking(true) {
        elog!("cannot set non-blocking mode: {e}");
        std::process::exit(1);
    }
    platform::install_shutdown_handlers();

    log!("listening on http://{}", cfg.addr);
    log!("collecting into {file_path} ({existing} link(s) already there)");

    let shared: Shared = Arc::new(Mutex::new(store));

    while !platform::shutting_down() {
        if !platform::wait_for_connection(&listener, POLL_INTERVAL_MS) {
            continue;
        }
        // Drain everything the backlog holds: middle-clicking a dozen tweets
        // arrives as a dozen near-simultaneous connections.
        loop {
            match listener.accept() {
                Ok((stream, _)) => dispatch(stream, &shared),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    break
                }
                Err(e) => {
                    elog!("accept failed: {e}");
                    break;
                }
            }
        }
    }

    log!("shutting down");
    drain(Duration::from_secs(2));
    let total = shared.lock().map_or(existing, |s| s.total());
    log!("stopped; {total} link(s) in {file_path}");
}

fn dispatch(mut stream: TcpStream, shared: &Shared) {
    if INFLIGHT.load(Ordering::Relaxed) >= MAX_INFLIGHT {
        elog!("too many concurrent connections; rejecting one");
        let res = http::Response::text(503, "Service Unavailable", "busy\n");
        let _ = http::write_response(&mut stream, &res, &http::Cors::Allow(None));
        return;
    }
    INFLIGHT.fetch_add(1, Ordering::Relaxed);
    let shared = Arc::clone(shared);
    let spawned = thread::Builder::new()
        .name("conn".to_string())
        .stack_size(128 * 1024)
        .spawn(move || {
            handle_connection(stream, &shared);
            INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        });
    if let Err(e) = spawned {
        INFLIGHT.fetch_sub(1, Ordering::Relaxed);
        elog!("cannot spawn worker thread: {e}");
    }
}

fn drain(limit: Duration) {
    let deadline = Instant::now() + limit;
    while INFLIGHT.load(Ordering::Relaxed) > 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
}

fn handle_connection(mut stream: TcpStream, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_nodelay(true);

    let req = match http::read_request(&mut stream, config::MAX_BODY) {
        Ok(req) => req,
        Err(e) => {
            elog!("bad request: {}", e.detail);
            let body = format!("{}\n", e.detail);
            let res = http::Response::text(e.status, e.reason, &body);
            let _ = http::write_response(&mut stream, &res, &http::Cors::Allow(None));
            return;
        }
    };

    let origin = req.header("origin").map(str::to_string);
    let cors = http::cors_for(origin.as_deref());
    if matches!(cors, http::Cors::Deny) {
        elog!(
            "refused {} {} from web origin {}",
            req.method,
            req.path,
            origin.unwrap_or_default()
        );
        let res = http::Response::text(
            403,
            "Forbidden",
            "this endpoint only accepts browser extensions and local tools\n",
        );
        let _ = http::write_response(&mut stream, &res, &cors);
        return;
    }

    let result = route(&req, shared);
    if let Err(e) = http::write_response(&mut stream, &result.as_response(), &cors) {
        elog!("response write failed: {e}");
    }
}

/// A routed reply, owning its body so it can outlive the routing call.
struct Reply {
    status: u16,
    reason: &'static str,
    body: String,
    json: bool,
    extra: Vec<String>,
}

impl Reply {
    fn json(status: u16, reason: &'static str, body: String) -> Self {
        Reply {
            status,
            reason,
            body,
            json: true,
            extra: Vec::new(),
        }
    }

    fn text(status: u16, reason: &'static str, body: &str) -> Self {
        Reply {
            status,
            reason,
            body: body.to_string(),
            json: false,
            extra: Vec::new(),
        }
    }

    fn as_response(&self) -> http::Response<'_> {
        let mut res = if self.json {
            http::Response::json(self.status, self.reason, &self.body)
        } else {
            http::Response::text(self.status, self.reason, &self.body)
        };
        res.extra = self.extra.clone();
        res
    }
}

fn route(req: &http::Request, shared: &Shared) -> Reply {
    match (req.method.as_str(), req.path.as_str()) {
        ("OPTIONS", _) => preflight(req),
        ("POST", "/" | "/add" | "/link" | "/links") => collect(req, shared),
        ("GET" | "HEAD", "/health" | "/healthz") => {
            let (total, path) = shared.lock().map_or((0, String::new()), |s| {
                (s.total(), s.path().display().to_string())
            });
            Reply::json(
                200,
                "OK",
                format!(
                    "{{\"ok\":true,\"total\":{total},\"file\":\"{}\"}}",
                    escape(&path)
                ),
            )
        }
        ("GET" | "HEAD", "/") => Reply::text(
            200,
            "OK",
            "x-link-receiver: POST a URL here (text/plain or JSON).\n",
        ),
        _ => Reply::text(404, "Not Found", "not found\n"),
    }
}

fn preflight(req: &http::Request) -> Reply {
    let requested = req
        .header("access-control-request-headers")
        .unwrap_or("Content-Type");
    // Echo back only what can safely go in a header value.
    let allow: String = requested
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(256)
        .collect();

    let mut reply = Reply::text(204, "No Content", "");
    reply.extra = vec![
        "Access-Control-Allow-Methods: POST, GET, OPTIONS".to_string(),
        format!("Access-Control-Allow-Headers: {allow}"),
        // A day of cache: without this every capture pays for a preflight.
        "Access-Control-Max-Age: 86400".to_string(),
        // Chromium's Private Network Access check for public -> loopback.
        "Access-Control-Allow-Private-Network: true".to_string(),
    ];
    reply
}

fn collect(req: &http::Request, shared: &Shared) -> Reply {
    let text = String::from_utf8_lossy(&req.body);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return bad_request("empty body");
    }

    let content_type = req
        .header("content-type")
        .unwrap_or("")
        .to_ascii_lowercase();
    let looks_json = content_type.contains("json") || trimmed.starts_with(['{', '[', '"']);

    let candidates: Vec<String> = if looks_json {
        match json::parse(trimmed) {
            Ok(value) => {
                let mut out = Vec::new();
                json::collect_urls(&value, &mut out);
                out
            }
            Err(e) => return bad_request(&format!("invalid JSON: {e}")),
        }
    } else {
        trimmed.lines().map(str::to_string).collect()
    };

    let (mut added, mut duplicate, mut rejected) = (0u32, 0u32, 0u32);
    // Poisoning is recovered from: one panicking worker must not wedge the daemon.
    let mut store = shared.lock().unwrap_or_else(|e| e.into_inner());

    for candidate in &candidates {
        if candidate.trim().is_empty() {
            continue;
        }
        let Some(clean) = url::sanitize(candidate) else {
            rejected += 1;
            log!("rejected {}", preview(candidate));
            continue;
        };
        match store.add(&clean) {
            Ok(true) => {
                added += 1;
                log!("+ {clean}");
            }
            Ok(false) => {
                duplicate += 1;
                log!("= {clean} (already collected)");
            }
            Err(e) => {
                elog!("write to {} failed: {e}", store.path().display());
                return Reply::json(
                    500,
                    "Internal Server Error",
                    format!("{{\"ok\":false,\"error\":\"{}\"}}", escape(&e.to_string())),
                );
            }
        }
    }
    let total = store.total();
    drop(store);

    if added == 0 && duplicate == 0 {
        return bad_request("no usable URL in request body");
    }
    Reply::json(
        200,
        "OK",
        format!(
            "{{\"ok\":true,\"added\":{added},\"duplicate\":{duplicate},\
             \"rejected\":{rejected},\"total\":{total}}}"
        ),
    )
}

fn bad_request(msg: &str) -> Reply {
    Reply::json(
        400,
        "Bad Request",
        format!("{{\"ok\":false,\"error\":\"{}\"}}", escape(msg)),
    )
}

/// Minimal JSON string escaping for the short messages we emit.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Trim untrusted text before it reaches the log.
fn preview(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| !c.is_control()).take(80).collect();
    format!("{:?}", cleaned)
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_json_strings() {
        assert_eq!(escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(escape("line\nbreak"), "line\\nbreak");
        assert_eq!(escape("bell\u{7}"), "bell\\u0007");
        assert_eq!(escape("plain"), "plain");
    }
}
