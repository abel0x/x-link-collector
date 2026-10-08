// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! A minimal JSON reader and writer.
//!
//! Reading started as just enough to pull URLs out of a posted body. Accepted
//! shapes: `"url"`, `["a","b"]`, `{"url":"a"}`, `{"urls":["a","b"]}`, and
//! `{"links":[{"url":"a"}]}`. Only `url`/`urls`/`link`/`links` keys are
//! descended into, so unrelated string fields never end up in the file.
//!
//! The panel API and the settings file need the other direction too: `Display`
//! writes compact JSON, `pretty()` the indented form a person might edit.

use std::fmt;

const MAX_DEPTH: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Field lookup on an object; `None` for anything else.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Two-space indented output, for files a person may open in an editor.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }

    fn write_pretty(&self, out: &mut String, depth: usize) {
        let pad = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
        match self {
            Json::Arr(items) if !items.is_empty() => {
                out.push_str("[\n");
                for (i, v) in items.iter().enumerate() {
                    pad(out, depth + 1);
                    v.write_pretty(out, depth + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                pad(out, depth);
                out.push(']');
            }
            Json::Obj(fields) if !fields.is_empty() => {
                out.push_str("{\n");
                for (i, (k, v)) in fields.iter().enumerate() {
                    pad(out, depth + 1);
                    out.push_str(&format!("\"{}\": ", escape(k)));
                    v.write_pretty(out, depth + 1);
                    out.push_str(if i + 1 < fields.len() { ",\n" } else { "\n" });
                }
                pad(out, depth);
                out.push('}');
            }
            scalar => out.push_str(&scalar.to_string()),
        }
    }
}

impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Json::Null => f.write_str("null"),
            Json::Bool(b) => write!(f, "{b}"),
            // JSON has no NaN or infinity, and whole numbers read better
            // without a trailing `.0`.
            Json::Num(n) if !n.is_finite() => f.write_str("null"),
            Json::Num(n) if n.fract() == 0.0 && n.abs() < 9e15 => write!(f, "{}", *n as i64),
            Json::Num(n) => write!(f, "{n}"),
            Json::Str(s) => write!(f, "\"{}\"", escape(s)),
            Json::Arr(items) => {
                f.write_str("[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Json::Obj(fields) => {
                f.write_str("{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "\"{}\":{v}", escape(k))?;
                }
                f.write_str("}")
            }
        }
    }
}

/// `obj([("ok", true.into()), ("total", 3.into())])`
pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Obj(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}

impl From<f64> for Json {
    fn from(n: f64) -> Self {
        Json::Num(n)
    }
}

macro_rules! from_integer {
    ($($t:ty)*) => {$(
        impl From<$t> for Json {
            fn from(n: $t) -> Self {
                Json::Num(n as f64)
            }
        }
    )*};
}
from_integer!(u16 u32 u64 usize i32 i64);

impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(v: Option<T>) -> Self {
        v.map_or(Json::Null, Into::into)
    }
}

impl From<Vec<Json>> for Json {
    fn from(items: Vec<Json>) -> Self {
        Json::Arr(items)
    }
}

/// String escaping for JSON output.
pub fn escape(s: &str) -> String {
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

#[derive(Debug)]
pub struct Error {
    pub msg: String,
    pub at: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.msg, self.at)
    }
}

pub fn parse(input: &str) -> Result<Json, Error> {
    let mut p = Parser {
        b: input.as_bytes(),
        i: 0,
        depth: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(p.err("trailing data after JSON value"));
    }
    Ok(v)
}

/// Collect every string reachable through URL-ish keys, in document order.
pub fn collect_urls(v: &Json, out: &mut Vec<String>) {
    match v {
        Json::Str(s) => out.push(s.clone()),
        Json::Arr(items) => items.iter().for_each(|i| collect_urls(i, out)),
        Json::Obj(fields) => {
            for (k, val) in fields {
                if matches!(
                    k.to_ascii_lowercase().as_str(),
                    "url" | "urls" | "link" | "links"
                ) {
                    collect_urls(val, out);
                }
            }
        }
        _ => {}
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> Error {
        Error {
            msg: msg.to_string(),
            at: self.i,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &[u8]) -> bool {
        if self.b[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Json, Error> {
        if self.depth > MAX_DEPTH {
            return Err(self.err("JSON nested too deeply"));
        }
        match self
            .peek()
            .ok_or_else(|| self.err("unexpected end of input"))?
        {
            b'"' => self.string().map(Json::Str),
            b'{' => self.object(),
            b'[' => self.array(),
            b't' if self.eat(b"true") => Ok(Json::Bool(true)),
            b'f' if self.eat(b"false") => Ok(Json::Bool(false)),
            b'n' if self.eat(b"null") => Ok(Json::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.err("unexpected character")),
        }
    }

    fn array(&mut self) -> Result<Json, Error> {
        self.i += 1; // '['
        self.depth += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Json::Arr(items));
        }
        loop {
            self.ws();
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, Error> {
        self.i += 1; // '{'
        self.depth += 1;
        let mut fields = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Json::Obj(fields));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected object key"));
            }
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(self.err("expected ':'"));
            }
            self.i += 1;
            self.ws();
            let val = self.value()?;
            fields.push((key, val));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Obj(fields));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn number(&mut self) -> Result<Json, Error> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[start..self.i])
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .map(Json::Num)
            .ok_or_else(|| self.err("invalid number"))
    }

    fn string(&mut self) -> Result<String, Error> {
        self.i += 1; // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            self.i += 1;
            match c {
                b'"' => {
                    return String::from_utf8(out).map_err(|_| self.err("invalid UTF-8 in string"))
                }
                b'\\' => {
                    let e = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0C),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let ch = self.unicode_escape()?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
                0x00..=0x1F => return Err(self.err("control character in string")),
                _ => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        let end = self.i + 4;
        if end > self.b.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let s =
            std::str::from_utf8(&self.b[self.i..end]).map_err(|_| self.err("bad \\u escape"))?;
        let v = u32::from_str_radix(s, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.i = end;
        Ok(v)
    }

    fn unicode_escape(&mut self) -> Result<char, Error> {
        let hi = self.hex4()?;
        // Surrogate pair, e.g. 😀
        if (0xD800..0xDC00).contains(&hi) {
            if self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b'u') {
                self.i += 2;
                let lo = self.hex4()?;
                if (0xDC00..0xE000).contains(&lo) {
                    let cp = 0x1_0000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                    return char::from_u32(cp).ok_or_else(|| self.err("invalid code point"));
                }
            }
            return Ok('\u{FFFD}');
        }
        if (0xDC00..0xE000).contains(&hi) {
            return Ok('\u{FFFD}'); // lone low surrogate
        }
        char::from_u32(hi).ok_or_else(|| self.err("invalid code point"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        collect_urls(&parse(src).expect("parse"), &mut out);
        out
    }

    #[test]
    fn extracts_supported_shapes() {
        assert_eq!(urls(r#""https://a""#), ["https://a"]);
        assert_eq!(
            urls(r#"["https://a","https://b"]"#),
            ["https://a", "https://b"]
        );
        assert_eq!(urls(r#"{"url":"https://a"}"#), ["https://a"]);
        assert_eq!(
            urls(r#"{"urls":["https://a","https://b"],"note":"ignored"}"#),
            ["https://a", "https://b"]
        );
        assert_eq!(urls(r#"{"links":[{"url":"https://a"}]}"#), ["https://a"]);
    }

    #[test]
    fn ignores_unrelated_keys() {
        assert!(urls(r#"{"title":"https://not-a-link","n":5}"#).is_empty());
    }

    #[test]
    fn handles_escapes_and_unicode() {
        assert_eq!(
            urls(r#"{"url":"https://a?q=1&b=2"}"#),
            ["https://a?q=1&b=2"]
        );
        assert_eq!(urls(r#"{"url":"😀"}"#), ["\u{1F600}"]);
        assert_eq!(urls(r#"{"url":"a\/b\"c"}"#), [r#"a/b"c"#]);
    }

    #[test]
    fn parses_scalars_and_whitespace() {
        assert_eq!(parse("  null ").unwrap(), Json::Null);
        assert_eq!(parse("true").unwrap(), Json::Bool(true));
        assert_eq!(parse("-1.5e3").unwrap(), Json::Num(-1500.0));
        assert_eq!(parse("{}").unwrap(), Json::Obj(vec![]));
        assert_eq!(parse("[]").unwrap(), Json::Arr(vec![]));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "",
            "{",
            "[1,",
            r#"{"a" 1}"#,
            r#"{"a":}"#,
            "tru",
            r#""unterminated"#,
            "{} extra",
            "[1,,2]",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn rejects_deep_nesting() {
        let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(escape("line\nbreak"), "line\\nbreak");
        assert_eq!(escape("bell\u{7}"), "bell\\u0007");
        assert_eq!(escape("plain"), "plain");
    }

    #[test]
    fn writes_what_it_reads() {
        let v = obj([
            ("ok", true.into()),
            ("n", 3u32.into()),
            ("half", 0.5.into()),
            ("s", "a\"b\n".into()),
            ("none", Json::Null),
            ("list", vec![1u32.into(), "x".into()].into()),
            ("empty", Json::Obj(vec![])),
        ]);
        let compact = v.to_string();
        assert_eq!(
            compact,
            r#"{"ok":true,"n":3,"half":0.5,"s":"a\"b\n","none":null,"list":[1,"x"],"empty":{}}"#
        );
        assert_eq!(parse(&compact).unwrap(), v);
        assert_eq!(parse(&v.pretty()).unwrap(), v);
        assert!(v.pretty().contains("\n  \"ok\": true,\n"));
        assert_eq!(Json::Num(f64::NAN).to_string(), "null");
    }

    #[test]
    fn looks_up_fields() {
        let v = parse(r#"{"a":"x","b":false,"c":2}"#).unwrap();
        assert_eq!(v.get("a").and_then(Json::as_str), Some("x"));
        assert_eq!(v.get("b").and_then(Json::as_bool), Some(false));
        assert_eq!(v.get("c").and_then(Json::as_f64), Some(2.0));
        assert!(v.get("missing").is_none());
    }
}
