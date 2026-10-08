// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! URL validation and canonicalisation.
//!
//! The extension already sends clean URLs, but the receiver is a plain HTTP
//! endpoint that anything can post to, so it re-does the work itself:
//!
//! * anything that is not an `http(s)://` URL, or that contains whitespace or
//!   control characters, is rejected (a newline would corrupt the file format);
//! * x.com / twitter.com status URLs are rewritten to one canonical form;
//! * every other URL is stored exactly as received -- this is a general link
//!   collector, and stripping query strings off arbitrary links would break them.

/// Long enough for any real link, short enough that a junk POST can't bloat the file.
pub const MAX_URL_LEN: usize = 2048;

pub struct TweetRef {
    /// `None` for `/i/web/status/<id>` style links, where the handle is unknown.
    pub handle: Option<String>,
    pub id: String,
}

/// Validate and canonicalise one candidate line. `None` means "not a URL".
pub fn sanitize(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t.len() > MAX_URL_LEN {
        return None;
    }
    if t.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    // Byte comparison, not string slicing: a multi-byte char at index 7 would
    // panic a `&t[..7]`.
    let b = t.as_bytes();
    let scheme_ok = (b.len() > 7 && b[..7].eq_ignore_ascii_case(b"http://"))
        || (b.len() > 8 && b[..8].eq_ignore_ascii_case(b"https://"));
    if !scheme_ok {
        return None;
    }
    Some(canonicalize(t))
}

/// `https://mobile.twitter.com/User/status/123?s=20&t=x#foo` ->
/// `https://x.com/User/status/123`. Non-tweet URLs pass through untouched.
pub fn canonicalize(url: &str) -> String {
    match tweet_ref(url) {
        Some(TweetRef {
            handle: Some(h),
            id,
        }) => format!("https://x.com/{h}/status/{id}"),
        Some(TweetRef { handle: None, id }) => format!("https://x.com/i/web/status/{id}"),
        None => url.to_string(),
    }
}

/// Key used for deduplication.
///
/// For tweets this is the status id alone: x.com serves the same tweet under
/// *any* handle in the path, so `/alice/status/1` and `/bob/status/1` are the
/// same link and must not both land in the file.
pub fn dedupe_key(url: &str) -> String {
    match tweet_ref(url) {
        Some(t) => format!("tweet:{}", t.id),
        None => url.to_string(),
    }
}

/// Split `scheme://user@host:port/path?q#f` into (host, path). Host is returned
/// without userinfo or port; the path keeps its query and fragment.
fn split_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.split_once("://")?.1;
    let (authority, path) = match rest.find(['/', '?', '#']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some((host, path))
}

/// The handle and status id of a tweet permalink; `None` for anything else.
pub fn tweet_ref(url: &str) -> Option<TweetRef> {
    let (host_raw, path_raw) = split_url(url)?;

    let lowered = host_raw.to_ascii_lowercase();
    let mut host = lowered.as_str();
    for prefix in ["www.", "mobile.", "m."] {
        if let Some(stripped) = host.strip_prefix(prefix) {
            host = stripped;
            break;
        }
    }
    if host != "x.com" && host != "twitter.com" {
        return None;
    }

    let path = path_raw.split(['?', '#']).next().unwrap_or(path_raw);
    let mut segs = path.split('/').filter(|s| !s.is_empty());

    let first = segs.next()?;
    if first.eq_ignore_ascii_case("i") {
        // /i/status/<id> and /i/web/status/<id>
        let mut next = segs.next()?;
        if next.eq_ignore_ascii_case("web") {
            next = segs.next()?;
        }
        if !next.eq_ignore_ascii_case("status") {
            return None;
        }
        let id = segs.next()?;
        return is_status_id(id).then(|| TweetRef {
            handle: None,
            id: id.to_string(),
        });
    }

    if !is_handle(first) {
        return None;
    }
    let kind = segs.next()?;
    if !kind.eq_ignore_ascii_case("status") && !kind.eq_ignore_ascii_case("statuses") {
        return None;
    }
    let id = segs.next()?;
    // Trailing segments (/photo/1, /video/2, /analytics) are dropped here.
    is_status_id(id).then(|| TweetRef {
        handle: Some(first.to_string()),
        id: id.to_string(),
    })
}

fn is_status_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 25 && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_handle(s: &str) -> bool {
    !s.is_empty() && s.len() <= 15 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalises_tweet_urls() {
        let cases = [
            "https://x.com/someone/status/1234567890",
            "http://twitter.com/someone/status/1234567890",
            "https://mobile.twitter.com/someone/status/1234567890?s=20&t=abcd",
            "https://www.x.com/someone/status/1234567890/photo/1",
            "https://x.com/someone/status/1234567890#anchor",
            "https://x.com/someone/statuses/1234567890",
            "https://x.com/someone/status/1234567890/",
        ];
        for c in cases {
            assert_eq!(
                canonicalize(c),
                "https://x.com/someone/status/1234567890",
                "{c}"
            );
        }
    }

    #[test]
    fn handles_i_web_status_form() {
        assert_eq!(
            canonicalize("https://twitter.com/i/web/status/12345"),
            "https://x.com/i/web/status/12345"
        );
        assert_eq!(
            canonicalize("https://x.com/i/status/12345?foo=1"),
            "https://x.com/i/web/status/12345"
        );
    }

    #[test]
    fn leaves_other_urls_intact() {
        let other = "https://example.com/a/b?keep=this#frag";
        assert_eq!(canonicalize(other), other);
        // Not a status URL, so the query survives.
        assert_eq!(
            canonicalize("https://x.com/someone"),
            "https://x.com/someone"
        );
        assert_eq!(
            canonicalize("https://x.com/search?q=rust"),
            "https://x.com/search?q=rust"
        );
    }

    #[test]
    fn dedupes_same_tweet_under_any_handle() {
        assert_eq!(
            dedupe_key("https://x.com/alice/status/999"),
            dedupe_key("https://twitter.com/bob/status/999?s=20")
        );
        assert_eq!(
            dedupe_key("https://x.com/alice/status/999"),
            dedupe_key("https://x.com/i/web/status/999")
        );
        assert_ne!(
            dedupe_key("https://x.com/alice/status/999"),
            dedupe_key("https://x.com/alice/status/998")
        );
    }

    #[test]
    fn rejects_non_urls_and_injection() {
        assert_eq!(sanitize(""), None);
        assert_eq!(sanitize("   "), None);
        assert_eq!(sanitize("javascript:alert(1)"), None);
        assert_eq!(sanitize("file:///etc/passwd"), None);
        assert_eq!(sanitize("ftp://example.com/x"), None);
        assert_eq!(sanitize("https://ex.com/a\nhttps://ex.com/b"), None);
        assert_eq!(sanitize("https://ex.com/a b"), None);
        assert_eq!(
            sanitize(&format!("https://ex.com/{}", "a".repeat(4096))),
            None
        );
        assert_eq!(sanitize("https://"), None);
    }

    #[test]
    fn accepts_and_trims_valid_input() {
        assert_eq!(
            sanitize("  https://x.com/someone/status/1234567890?s=46  "),
            Some("https://x.com/someone/status/1234567890".to_string())
        );
        assert_eq!(
            sanitize("HTTPS://example.com/x"),
            Some("HTTPS://example.com/x".to_string())
        );
    }

    #[test]
    fn rejects_malformed_status_paths() {
        for bad in [
            "https://x.com/someone/status/notanumber",
            "https://x.com/someone/status/",
            "https://x.com/waytoolongahandlehere/status/1234567890",
            "https://notx.com/someone/status/1234567890",
            "https://x.com.evil.tld/someone/status/1234567890",
        ] {
            assert_eq!(canonicalize(bad), bad, "{bad} should not be rewritten");
        }
    }
}
