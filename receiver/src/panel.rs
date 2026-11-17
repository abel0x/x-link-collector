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

/// The page's place for the panel key; filled in as it is served.
const KEY_SLOT: &str = r#"<meta name="panel-key" content="">"#;

pub fn asset(path: &str, key: &str) -> Option<Response> {
    if path == "/" || path == "/index.html" {
        let page = include_str!("../panel/index.html").replacen(
            KEY_SLOT,
            &format!(r#"<meta name="panel-key" content="{key}">"#),
            1,
        );
        let mut res = Response::asset("text/html; charset=utf-8", b"");
        res.body = std::borrow::Cow::Owned(page.into_bytes());
        return Some(secured(res));
    }
    if path == "/words.js" {
        let mut res = Response::asset("text/javascript; charset=utf-8", b"");
        res.body = std::borrow::Cow::Owned(words());
        return Some(secured(res));
    }
    let (content_type, body): (&'static str, &'static [u8]) = match path {
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
        let res = asset("/words.js", "k").expect("served");
        let text = String::from_utf8(res.body.to_vec()).unwrap();
        assert!(text.starts_with("'use strict';\nconst WORDS = {"));
        assert!(text.contains("\"en\": {") && text.contains("\"tr\": {"));
        assert!(text.contains("\"nav.links\": \"Bağlantılar\""));
    }

    #[test]
    fn the_page_carries_the_key() {
        assert!(include_str!("../panel/index.html").contains(KEY_SLOT));
        let res = asset("/", "0123abcd").expect("served");
        let page = String::from_utf8(res.body.to_vec()).unwrap();
        assert!(page.contains(r#"<meta name="panel-key" content="0123abcd">"#));
    }
}
