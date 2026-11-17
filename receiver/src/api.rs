// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The panel's JSON API, and the capture endpoint the extension posts to.
//!
//! Everything under /api/ answers only on a loopback `Host`, and to the panel's
//! own origin or a local tool that sends none (see `http::host_is_local`,
//! `http::cors_for` and `http::extension_may`). Requests that
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
        "/api/export" if read => export(req, app),
        "/api/gallery" if read => gallery(req, app),
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
        "/api/status" | "/api/links" | "/api/export" | "/api/gallery" | "/api/links/remove"
        | "/api/settings" | "/api/job" | "/api/job/stop" | "/api/tools" | "/api/open"
        | "/api/autostart" | "/api/quit" => error(405, "method not allowed"),
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

/// The media folder's photos and videos, newest first, for the Gallery page.
fn gallery(req: &Request, app: &App) -> Response {
    let number = |name: &str, default: usize| {
        req.param(name)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(default)
    };
    Response::json(
        200,
        &crate::media::gallery(
            &app.media_dir(),
            &req.param("handle").unwrap_or_default(),
            &req.param("kind").unwrap_or_default(),
            number("offset", 0),
            number("limit", 60).clamp(1, 500),
        ),
    )
}

/// Every link the Links page would show for this search and filter, oldest
/// first, as a file to keep: `?format=csv` (the default) or `?format=json`.
fn export(req: &Request, app: &App) -> Response {
    let page = app.links_page(
        &req.param("q").unwrap_or_default(),
        &req.param("status").unwrap_or_default(),
        0,
        usize::MAX,
    );
    let mut rows = match page.get("items") {
        Some(Json::Arr(items)) => items.clone(),
        _ => Vec::new(),
    };
    rows.reverse(); // the page lists the newest first; a file reads in order
    let day = crate::log::iso8601(crate::history::now() as i64);
    let day = &day[..10];
    let (content_type, name, body) = match req.param("format").as_deref() {
        Some("json") => (
            "application/json; charset=utf-8",
            format!("x-link-collector-links-{day}.json"),
            Json::Arr(rows.iter().map(export_json).collect())
                .pretty()
                .into_bytes(),
        ),
        None | Some("csv") => (
            "text/csv; charset=utf-8",
            format!("x-link-collector-links-{day}.csv"),
            export_csv(&rows).into_bytes(),
        ),
        Some(_) => return error(400, "format is csv or json"),
    };
    let mut res = Response::text(200, "");
    res.content_type = content_type;
    res.body = std::borrow::Cow::Owned(body);
    res.extra.push(format!(
        "Content-Disposition: attachment; filename=\"{name}\""
    ));
    res
}

const EXPORT_COLUMNS: [&str; 8] = [
    "line",
    "url",
    "handle",
    "tweet_id",
    "status",
    "collected_at",
    "files",
    "error",
];

/// One link as it goes into the file: the time as ISO 8601, not seconds.
fn export_json(row: &Json) -> Json {
    let field = |k: &str| row.get(k).cloned().unwrap_or(Json::Null);
    json::obj([
        ("line", field("line")),
        ("url", field("url")),
        ("handle", field("handle")),
        ("tweet_id", field("id")),
        ("status", field("status")),
        (
            "collected_at",
            collected(row).map_or(Json::Null, Json::from),
        ),
        ("files", field("files")),
        ("error", field("error")),
    ])
}

fn collected(row: &Json) -> Option<String> {
    row.get("captured")
        .and_then(Json::as_f64)
        .map(|secs| crate::log::iso8601(secs as i64))
}

/// RFC 4180, with a byte-order mark so spreadsheet programs read it as UTF-8.
fn export_csv(rows: &[Json]) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str(&EXPORT_COLUMNS.join(","));
    out.push_str("\r\n");
    for row in rows {
        let text = |k: &str| match row.get(k) {
            Some(Json::Str(s)) => s.clone(),
            Some(Json::Num(n)) => format!("{n}"),
            _ => String::new(),
        };
        let files = match row.get("files") {
            Some(Json::Arr(files)) => files
                .iter()
                .filter_map(Json::as_str)
                .collect::<Vec<_>>()
                .join(" | "),
            _ => String::new(),
        };
        let cells = [
            text("line"),
            text("url"),
            text("handle"),
            text("id"),
            text("status"),
            collected(row).unwrap_or_default(),
            files,
            text("error"),
        ];
        let cells: Vec<String> = cells.iter().map(|c| csv_cell(c)).collect();
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    out
}

/// Quoted when it must be, and never read as a formula: a spreadsheet would
/// run a cell that starts with = + - or @.
fn csv_cell(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
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

    // Links read from an X archive were collected long ago, on no known day:
    // they go into the file, but not into the per-day history. Links marked
    // "downloaded" are written into the download history as such, too.
    let mut imported = false;
    let mut downloaded = false;
    let candidates: Vec<String> = if looks_json {
        match json::parse(trimmed) {
            Ok(value) => {
                imported = value.get("imported").and_then(Json::as_bool) == Some(true);
                downloaded = value.get("downloaded").and_then(Json::as_bool) == Some(true);
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
            if !imported {
                app.captured(link);
            }
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
        let from = if imported {
            "an X archive"
        } else {
            "a pasted list"
        };
        log!("+ {added} link(s) from {from}, {duplicate} already collected");
    }

    if added == 0 && duplicate == 0 {
        return refused(
            400,
            &Problem::new("no_url", "no usable URL in request body"),
        );
    }
    let mut fields = vec![
        ("added".into(), added.into()),
        ("duplicate".into(), duplicate.into()),
        ("rejected".into(), rejected.into()),
        ("total".into(), total.into()),
    ];
    if downloaded {
        // The links are in the file either way; marking them is the part a
        // running job could get in the way of.
        match app.mark_downloaded(&clean) {
            Ok((marked, already)) => {
                fields.push(("marked".into(), marked.into()));
                fields.push(("already_downloaded".into(), already.into()));
            }
            Err(e) if e.code == "busy" => return refused(409, &e),
            Err(e) => return refused(500, &e),
        }
    }
    ok(fields)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_cells_are_quoted_and_never_formulas() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("=HYPERLINK(1)"), "'=HYPERLINK(1)");
        assert_eq!(csv_cell("-1"), "'-1");
    }

    #[test]
    fn csv_rows_follow_the_header() {
        let row = json::obj([
            ("line", 3.into()),
            ("url", "https://x.com/a/status/1".into()),
            ("id", "1".into()),
            ("handle", "a".into()),
            ("captured", Json::Num(0.0)),
            ("status", "done".into()),
            (
                "files",
                vec![Json::from("a/1-1.mp4"), Json::from("a/1-2.jpg")].into(),
            ),
            ("error", "".into()),
        ]);
        let csv = export_csv(&[row]);
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(
            lines[0],
            "\u{feff}line,url,handle,tweet_id,status,collected_at,files,error"
        );
        assert_eq!(
            lines[1],
            "3,https://x.com/a/status/1,a,1,done,1970-01-01T00:00:00Z,a/1-1.mp4 | a/1-2.jpg,"
        );
    }
}
