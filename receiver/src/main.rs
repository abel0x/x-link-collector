// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! x-link-receiver -- a loopback HTTP endpoint that appends links to a file,
//! and the panel that runs the rest of the kit from a browser tab.
//!
//! Shape of the thing: one thread parks in `poll()` on the listening socket and
//! hands each accepted connection to a short-lived worker thread. There is no
//! async runtime and no dependencies; idle cost is one blocked thread. A job
//! the panel starts -- a download, a flatten -- is a child process with a
//! thread watching it, for as long as it runs.

#[macro_use]
mod log;

mod api;
mod app;
mod config;
mod history;
mod http;
mod jobs;
mod json;
mod media;
mod panel;
mod platform;
mod settings;
mod store;
mod url;

use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use app::App;
use http::Response;
use settings::Settings;
use store::Store;

/// Bounds worker threads so a misbehaving client cannot spawn without limit.
static INFLIGHT: AtomicUsize = AtomicUsize::new(0);
const MAX_INFLIGHT: usize = 64;

/// How long a single client may take to send its request / read its response.
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Accept-loop tick; also the worst-case delay before a shutdown is noticed.
const POLL_INTERVAL_MS: i32 = 250;

type Shared = Arc<App>;

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

    let (settings, warning) = Settings::load(&cfg.config_file);
    if let Some(warning) = warning {
        elog!("{warning}");
    }

    let links_file = app::resolve_links_file(&cfg, &settings);
    let store = match Store::open(links_file.clone(), !cfg.no_fsync && settings.fsync) {
        Ok(s) => s,
        Err(e) => {
            elog!("cannot use {}: {e}", links_file.display());
            std::process::exit(1);
        }
    };
    let existing = store.total();

    let addr = cfg
        .addr
        .clone()
        .unwrap_or_else(|| format!("{}:{}", config::DEFAULT_HOST, settings.port));
    // Explorer gave us a console of our own: a double-click, or a logon start.
    let own_console = platform::double_clicked();
    let open = cfg.open || (!cfg.no_open && own_console);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) if e.kind() == io::ErrorKind::AddrInUse && open => {
            // Double-clicked while it already runs: what was wanted is the
            // panel, and the running receiver has it.
            let port = addr.rsplit_once(':').map_or("9876", |(_, p)| p);
            let _ = platform::open(format!("http://127.0.0.1:{port}/").as_ref());
            log!("already running; opened its panel");
            return;
        }
        Err(e) => {
            let hint = if e.kind() == io::ErrorKind::AddrInUse {
                " (already running?)"
            } else {
                ""
            };
            elog!("cannot bind {addr}: {e}{hint}");
            std::process::exit(1);
        }
    };
    if let Err(e) = listener.set_nonblocking(true) {
        elog!("cannot set non-blocking mode: {e}");
        std::process::exit(1);
    }
    let port = listener.local_addr().map_or(settings.port, |a| a.port());
    platform::install_shutdown_handlers();

    // The panel answers to loopback names only, whatever address is bound.
    let panel = format!("http://127.0.0.1:{port}/");
    log!("listening on http://{addr}");
    log!(
        "collecting into {} ({existing} link(s) already there)",
        links_file.display()
    );
    log!("the panel is at {panel}");

    if own_console {
        // The window would only sit there; the panel shows everything it
        // would, and can quit the receiver too.
        let log_file = cfg
            .config_file
            .parent()
            .map_or_else(|| "receiver.log".into(), |d| d.join("receiver.log"));
        log!("logging to {} from here on", log_file.display());
        if log::to_file(&log_file).is_ok() {
            platform::detach_console();
        }
    }
    let app: Shared = Arc::new(App::new(cfg, settings, store, port));
    if open {
        if let Err(e) = platform::open(panel.as_ref()) {
            elog!("cannot open a browser: {e}");
        }
    }

    while !platform::shutting_down() {
        app.tick();
        if !platform::wait_for_connection(&listener, POLL_INTERVAL_MS) {
            continue;
        }
        // Drain everything the backlog holds: middle-clicking a dozen tweets
        // arrives as a dozen near-simultaneous connections.
        loop {
            match listener.accept() {
                Ok((stream, _)) => dispatch(stream, &app),
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
    app.jobs.shutdown(Duration::from_secs(5));
    drain(Duration::from_secs(2));
    let store = app.store();
    log!(
        "stopped; {} link(s) in {}",
        store.total(),
        store.path().display()
    );
}

fn dispatch(mut stream: TcpStream, app: &Shared) {
    if INFLIGHT.load(Ordering::Relaxed) >= MAX_INFLIGHT {
        elog!("too many concurrent connections; rejecting one");
        let _ = stream.set_nonblocking(false);
        let res = Response::text(503, "busy\n");
        let _ = http::write_response(&mut stream, &res, &http::Cors::Allow(None), false);
        return;
    }
    INFLIGHT.fetch_add(1, Ordering::Relaxed);
    let app = Arc::clone(app);
    let spawned = thread::Builder::new()
        .name("conn".to_string())
        .stack_size(128 * 1024)
        .spawn(move || {
            handle_connection(stream, &app);
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

fn handle_connection(mut stream: TcpStream, app: &App) {
    // macOS and the BSDs hand out accepted sockets in the listener's
    // non-blocking mode; Linux does not. The timeouts below only mean
    // something on a blocking socket, so say which this is.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_nodelay(true);

    let req = match http::read_request(&mut stream, config::MAX_BODY) {
        Ok(req) => req,
        Err(e) => {
            elog!("bad request: {}", e.detail);
            let res = Response::text(e.status, format!("{}\n", e.detail));
            let _ = http::write_response(&mut stream, &res, &http::Cors::Allow(None), false);
            return;
        }
    };

    let origin = req.header("origin").map(str::to_string);
    let cors = http::cors_for(origin.as_deref(), app.port);
    if matches!(cors, http::Cors::Deny) {
        elog!(
            "refused {} {} from web origin {}",
            req.method,
            req.path,
            origin.unwrap_or_default()
        );
        let res = Response::text(
            403,
            "this endpoint only accepts browser extensions, its own panel and local tools\n",
        );
        let _ = http::write_response(&mut stream, &res, &cors, false);
        return;
    }
    // A capture carries the extension's Origin. Its plain GETs may not, so
    // the extension also says who it is; it only decides a setup hint.
    if origin.as_deref().map_or(false, http::is_extension)
        || req.header("x-link-collector") == Some("extension")
    {
        app.extension_contact();
    }

    let res = route(&req, app);
    if let Err(e) = http::write_response(&mut stream, &res, &cors, req.method == "HEAD") {
        elog!("response write failed: {e}");
    }
}

fn route(req: &http::Request, app: &App) -> Response {
    let (method, path) = (req.method.as_str(), req.path.as_str());
    let read = matches!(method, "GET" | "HEAD");
    // Anything that reads state answers to loopback names only; see
    // http::host_is_local for the attack this closes.
    let local = http::host_is_local(req.header("host"));
    let refused = || Response::text(421, "this only answers on 127.0.0.1 or localhost\n");
    match (method, path) {
        ("OPTIONS", _) => preflight(req),
        ("POST", "/" | "/add" | "/link" | "/links") => api::collect(req, app),
        ("GET" | "HEAD", "/health" | "/healthz") if local => health(app),
        ("GET" | "HEAD", "/health" | "/healthz") => refused(),
        ("GET" | "HEAD", "/") if !wants_html(req) => Response::text(
            200,
            "x-link-receiver: POST a URL here (text/plain or JSON), \
             or open this address in a browser for the panel.\n",
        ),
        _ if read || path.starts_with("/api/") => {
            if !local {
                return refused();
            }
            if path.starts_with("/api/") {
                api::route(req, app)
            } else if path.starts_with("/media/") && read {
                media::serve(req, &app.media_dir())
            } else {
                panel::asset(path).unwrap_or_else(|| Response::text(404, "not found\n"))
            }
        }
        _ => Response::text(404, "not found\n"),
    }
}

fn wants_html(req: &http::Request) -> bool {
    req.header("accept")
        .map_or(false, |a| a.contains("text/html"))
}

fn health(app: &App) -> Response {
    let store = app.store();
    let body = json::obj([
        ("ok", true.into()),
        ("total", store.total().into()),
        ("file", store.path().display().to_string().into()),
        ("version", env!("CARGO_PKG_VERSION").into()),
    ]);
    Response::json(200, &body)
}

fn preflight(req: &http::Request) -> Response {
    let requested = req
        .header("access-control-request-headers")
        .unwrap_or("Content-Type");
    // Echo back only what can safely go in a header value.
    let allow: String = requested
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(256)
        .collect();

    let mut res = Response::text(204, "");
    res.extra = vec![
        "Access-Control-Allow-Methods: POST, GET, OPTIONS".to_string(),
        format!("Access-Control-Allow-Headers: {allow}"),
        // A day of cache: without this every capture pays for a preflight.
        "Access-Control-Max-Age: 86400".to_string(),
        // Chromium's Private Network Access check for public -> loopback.
        "Access-Control-Allow-Private-Network: true".to_string(),
    ];
    res
}
