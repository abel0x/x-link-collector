// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The panel's JSON API, and the capture endpoint the extension posts to.
//!
//! Everything under /api/ answers only on a loopback `Host` and to the panel's
//! own origin (see `http::host_is_local` and `http::cors_for`). Requests that
//! change something must also carry a JSON body, which a cross-site form
//! cannot send without a preflight -- and the preflight is refused.

use crate::app::{self, App, Problem};
use crate::http::{Request, Response};
use crate::json::{self, Json};
use crate::platform;
use crate::url;

pub fn route(req: &Request, app: &App) -> Response {
    let read = matches!(req.method.as_str(), "GET" | "HEAD");
    let write = req.method == "POST";
    if write && !is_json(req) {
        return error(415, "send the body as application/json");
    }
    match req.path.as_str() {
        "/api/status" if read => status(req, app),
        "/api/links" if read => links(req, app),
        "/api/links" if write => collect(req, app),
        "/api/links/remove" if write => remove(req, app),
        "/api/settings" if read => Response::json(200, &app.settings_view()),
        "/api/settings" if write => save_settings(req, app),
        "/api/job" if read => job(req, app),
        "/api/job" if write => start_job(req, app),
        "/api/job/stop" if write => stop_job(app),
        "/api/tools" if read => Response::json(200, &app.tools(req.param("refresh").is_some())),
        "/api/open" if write => open(req, app),
        "/api/autostart" if write => autostart(req),
        "/api/quit" if write => quit(app),
        "/api/status" | "/api/links" | "/api/links/remove" | "/api/settings" | "/api/job"
        | "/api/job/stop" | "/api/tools" | "/api/open" | "/api/autostart" | "/api/quit" => {
            error(405, "method not allowed")
        }
        _ => error(404, "no such endpoint"),
    }
}

fn is_json(req: &Request) -> bool {
    req.header("content-type").map_or(false, |t| {
        t.to_ascii_lowercase().starts_with("application/json")
    })
}

fn error(status: u16, msg: &str) -> Response {
    Response::json(
        status,
        &json::obj([("ok", false.into()), ("error", msg.into())]),
    )
}

/// A refusal the panel can put into the reader's language; `error` keeps the
/// English for anything else that calls the API.
fn refused(status: u16, problem: &Problem) -> Response {
    let vars = problem
        .vars
        .iter()
        .map(|(k, v)| (k.to_string(), Json::from(v.as_str())))
        .collect();
    Response::json(
        status,
        &json::obj([
            ("ok", false.into()),
            ("error", problem.text.as_str().into()),
            ("code", problem.code.into()),
            ("vars", Json::Obj(vars)),
        ]),
    )
}

fn ok(fields: Vec<(String, Json)>) -> Response {
    let mut all = vec![("ok".to_string(), Json::Bool(true))];
    all.extend(fields);
    Response::json(200, &Json::Obj(all))
}

fn body(req: &Request) -> Result<Json, Response> {
    let text = std::str::from_utf8(&req.body).map_err(|_| error(400, "the body is not UTF-8"))?;
    if text.trim().is_empty() {
        return Ok(Json::Obj(Vec::new()));
    }
    json::parse(text.trim()).map_err(|e| error(400, &format!("invalid JSON: {e}")))
}

fn status(req: &Request, app: &App) -> Response {
    // JavaScript's getTimezoneOffset(): minutes *behind* UTC, so UTC+3 is -180.
    let offset = req
        .param("tz")
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|m| m.abs() <= 24 * 60)
        .map_or(0, |m| -m * 60);
    Response::json(200, &app.status(offset, req.param("strip").is_some()))
}

fn links(req: &Request, app: &App) -> Response {
    let number = |name: &str, default: usize| {
        req.param(name)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(default)
    };
    let page = app.links_page(
        &req.param("q").unwrap_or_default(),
        &req.param("status").unwrap_or_default(),
        number("offset", 0),
        number("limit", 50).clamp(1, 500),
    );
    Response::json(200, &page)
}

/// `POST /` from the extension (text/plain or JSON), and `POST /api/links`
/// from the panel's "add links" box.
pub fn collect(req: &Request, app: &App) -> Response {
    let text = String::from_utf8_lossy(&req.body);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return refused(400, &Problem::new("empty", "empty body"));
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
                // The panel's box sends whatever was pasted as one string;
                // treat it like a text body, a link per line.
                out.iter()
                    .flat_map(|s| s.lines().map(str::to_string).collect::<Vec<_>>())
                    .collect()
            }
            Err(e) => return error(400, &format!("invalid JSON: {e}")),
        }
    } else {
        trimmed.lines().map(str::to_string).collect()
    };

    let mut rejected = 0u32;
    let mut clean = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        if candidate.trim().is_empty() {
            continue;
        }
        match url::sanitize(candidate) {
            Some(link) => clean.push(link),
            None => {
                rejected += 1;
                log!("rejected {}", preview(candidate));
            }
        }
    }

    let mut store = app.store();
    let written = match store.add_all(&clean) {
        Ok(written) => written,
        Err(e) => {
            elog!("write to {} failed: {e}", store.path().display());
            return refused(500, &Problem::new("write", e.to_string()).because(&e));
        }
    };
    let total = store.total();
    drop(store);

    // A link at a time from the extension; a pasted list gets one line.
    let one_by_one = clean.len() <= 20;
    let (mut added, mut duplicate) = (0u32, 0u32);
    for (link, new) in clean.iter().zip(written) {
        if new {
            added += 1;
            app.captured(link);
            if one_by_one {
                log!("+ {link}");
            }
        } else {
            duplicate += 1;
            if one_by_one {
                log!("= {link} (already collected)");
            }
        }
    }
    if !one_by_one {
        log!("+ {added} link(s) from a pasted list, {duplicate} already collected");
    }

    if added == 0 && duplicate == 0 {
        return refused(
            400,
            &Problem::new("no_url", "no usable URL in request body"),
        );
    }
    ok(vec![
        ("added".into(), added.into()),
        ("duplicate".into(), duplicate.into()),
        ("rejected".into(), rejected.into()),
        ("total".into(), total.into()),
    ])
}

fn remove(req: &Request, app: &App) -> Response {
    let body = match body(req) {
        Ok(body) => body,
        Err(res) => return res,
    };
    let Some(link) = body.get("url").and_then(Json::as_str) else {
        return error(400, r#"send the link as {"url": "..."}"#);
    };
    match app.remove_link(link) {
        Ok(removed) => ok(vec![
            ("removed".into(), removed.into()),
            ("total".into(), app.store().total().into()),
        ]),
        Err(e) => refused(500, &e),
    }
}

fn save_settings(req: &Request, app: &App) -> Response {
    let changes = match body(req) {
        Ok(Json::Obj(changes)) => changes,
        Ok(_) => return error(400, "send the settings as a JSON object"),
        Err(res) => return res,
    };
    match app.update_settings(&changes) {
        Ok(()) => {
            log!("settings saved");
            Response::json(200, &app.settings_view())
        }
        Err(e) => refused(400, &e),
    }
}

fn job(req: &Request, app: &App) -> Response {
    let since = req
        .param("since")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    Response::json(200, &app.jobs.snapshot(since))
}

fn start_job(req: &Request, app: &App) -> Response {
    let body = match body(req) {
        Ok(body) => body,
        Err(res) => return res,
    };
    if app.jobs.running() {
        return refused(409, &app::busy());
    }
    let started = match body.get("kind").and_then(Json::as_str) {
        Some("download") => app.start_download(&body),
        Some("flatten") => app.start_flatten(&body, false),
        Some("unflatten") => app.start_flatten(&body, true),
        Some("setup") => app.start_setup(),
        _ => return error(400, "kind must be download, flatten, unflatten or setup"),
    };
    match started {
        Ok(id) => ok(vec![("id".into(), id.into())]),
        Err(e) if e.code == "busy" => refused(409, &e),
        Err(e) => refused(400, &e),
    }
}

fn stop_job(app: &App) -> Response {
    let stopping = app.jobs.stop();
    if stopping {
        log!("stopping the running job");
    }
    ok(vec![("stopping".into(), stopping.into())])
}

fn open(req: &Request, app: &App) -> Response {
    let body = match body(req) {
        Ok(body) => body,
        Err(res) => return res,
    };
    match app.open(body.get("what").and_then(Json::as_str).unwrap_or("")) {
        Ok(()) => ok(Vec::new()),
        Err(e) => refused(400, &e),
    }
}

/// Start at logon, or stop doing so. Only where `platform::autostart` answers.
fn autostart(req: &Request) -> Response {
    let body = match body(req) {
        Ok(body) => body,
        Err(res) => return res,
    };
    let Some(on) = body.get("on").and_then(Json::as_bool) else {
        return error(400, r#"send {"on": true} or {"on": false}"#);
    };
    match platform::set_autostart(on) {
        Ok(()) => {
            log!("start at logon: {}", if on { "on" } else { "off" });
            ok(vec![("autostart".into(), on.into())])
        }
        Err(e) => refused(
            500,
            &Problem::new("autostart", format!("cannot change the logon start: {e}")).because(&e),
        ),
    }
}

/// The panel's Quit button: the same clean stop a Ctrl-C gives, including a
/// running job, after this answer has gone out.
fn quit(app: &App) -> Response {
    log!("quit from the panel");
    if app.jobs.running() {
        app.jobs.stop();
    }
    platform::request_shutdown();
    ok(Vec::new())
}

/// Trim untrusted text before it reaches the log.
fn preview(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| !c.is_control()).take(80).collect();
    format!("{cleaned:?}")
}
