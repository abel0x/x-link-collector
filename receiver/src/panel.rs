// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The panel: one page, one stylesheet, one script, compiled into the binary
//! so the receiver stays a single file and the panel needs no network at all.
//! Its icon is the extension's, so there is one copy of the artwork.

use crate::http::Response;

/// No inline script, nothing loaded from anywhere else, never inside a frame.
const CSP: &str = "Content-Security-Policy: default-src 'none'; script-src 'self'; \
                   style-src 'self'; img-src 'self' data:; media-src 'self'; \
                   connect-src 'self'; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// The panel's words, one JSON file per language: English is the source,
/// Turkish follows it key for key (tests/i18n.test.mjs keeps them in step).
const EN: &str = include_str!("../panel/i18n/en.json");
const TR: &str = include_str!("../panel/i18n/tr.json");

/// Both languages as one classic script, so the panel has them before its own
/// script runs, with no request it would have to wait for.
fn words() -> Vec<u8> {
    format!(
        "'use strict';\nconst WORDS = {{\n\"en\": {},\n\"tr\": {}\n}};\n",
        EN.trim(),
        TR.trim()
    )
    .into_bytes()
}

pub fn asset(path: &str) -> Option<Response> {
    if path == "/words.js" {
        let mut res = Response::asset("text/javascript; charset=utf-8", b"");
        res.body = std::borrow::Cow::Owned(words());
        return Some(secured(res));
    }
    let (content_type, body): (&'static str, &'static [u8]) = match path {
        "/" | "/index.html" => (
            "text/html; charset=utf-8",
            include_bytes!("../panel/index.html"),
        ),
        "/panel.css" => (
            "text/css; charset=utf-8",
            include_bytes!("../panel/panel.css"),
        ),
        "/panel.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!("../panel/panel.js"),
        ),
        "/icon.png" | "/favicon.ico" => (
            "image/png",
            include_bytes!("../../extension/icons/icon-128.png"),
        ),
        _ => return None,
    };
    Some(secured(Response::asset(content_type, body)))
}

fn secured(mut res: Response) -> Response {
    res.extra = vec![
        CSP.to_string(),
        "X-Frame-Options: DENY".to_string(),
        "Referrer-Policy: no-referrer".to_string(),
    ];
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_is_one_script_with_both_languages() {
        let res = asset("/words.js").expect("served");
        let text = String::from_utf8(res.body.to_vec()).unwrap();
        assert!(text.starts_with("'use strict';\nconst WORDS = {"));
        assert!(text.contains("\"en\": {") && text.contains("\"tr\": {"));
        assert!(text.contains("\"nav.links\": \"Bağlantılar\""));
    }
}
