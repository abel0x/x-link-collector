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

pub fn asset(path: &str) -> Option<Response> {
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
    let mut res = Response::asset(content_type, body);
    res.extra = vec![
        CSP.to_string(),
        "X-Frame-Options: DENY".to_string(),
        "Referrer-Policy: no-referrer".to_string(),
    ];
    Some(res)
}
