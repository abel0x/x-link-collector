// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Command line / environment configuration.

use std::env;
use std::ffi::OsString;
/// Only the XDG path reads a config file; Windows resolves the desktop directly.
#[cfg(not(windows))]
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_ADDR: &str = "127.0.0.1:9876";
pub const FILE_NAME: &str = "links.txt";
/// Plenty for a batch of links; anything larger is not a link list.
pub const MAX_BODY: usize = 64 * 1024;

pub struct Config {
    pub addr: String,
    pub file: PathBuf,
    pub fsync: bool,
    pub quiet: bool,
}

pub const USAGE: &str = "\
x-link-receiver -- collects links posted to a loopback HTTP endpoint.

USAGE:
    x-link-receiver [OPTIONS]

OPTIONS:
    -a, --addr <HOST:PORT>   Listen address           [default: 127.0.0.1:9876]
    -p, --port <PORT>        Port only, host stays 127.0.0.1
    -f, --file <PATH>        Output file              [default: <desktop>/links.txt]
        --no-fsync           Flush without fsync (faster, less durable)
    -q, --quiet              Only log errors
    -h, --help               Show this help
    -V, --version            Show version

ENVIRONMENT:
    LINKS_FILE               Same as --file
    LINK_RECEIVER_ADDR       Same as --addr
    LINK_RECEIVER_PORT       Same as --port

The default output directory is your XDG desktop (XDG_DESKTOP_DIR from
~/.config/user-dirs.dirs, which is localised -- e.g. ~/Masaustu, ~/Escritorio),
falling back to ~/Desktop.

ENDPOINTS:
    POST /                   text/plain (one URL per line) or JSON
                             ({\"url\":...} / {\"urls\":[...]} / [...])
    OPTIONS /                CORS preflight
    GET  /health             liveness + link count
";

impl Config {
    pub fn from_args() -> Result<Option<Config>, String> {
        let mut cfg = Config {
            addr: env::var("LINK_RECEIVER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string()),
            file: default_links_file(),
            fsync: true,
            quiet: false,
        };
        if let Ok(p) = env::var("LINK_RECEIVER_PORT") {
            cfg.addr = with_port(&cfg.addr, p.trim())?;
        }

        let mut args = env::args_os().skip(1);
        while let Some(arg) = args.next() {
            let a = arg.to_string_lossy().into_owned();
            let mut next = |flag: &str| -> Result<OsString, String> {
                args.next().ok_or_else(|| format!("{flag} needs a value"))
            };
            match a.as_str() {
                "-h" | "--help" => {
                    print!("{USAGE}");
                    return Ok(None);
                }
                "-V" | "--version" => {
                    println!("x-link-receiver {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "-a" | "--addr" => cfg.addr = next(&a)?.to_string_lossy().into_owned(),
                "-p" | "--port" => {
                    let p = next(&a)?.to_string_lossy().into_owned();
                    cfg.addr = with_port(&cfg.addr, p.trim())?;
                }
                "-f" | "--file" => cfg.file = PathBuf::from(next(&a)?),
                "--no-fsync" => cfg.fsync = false,
                "-q" | "--quiet" => cfg.quiet = true,
                other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
            }
        }
        Ok(Some(cfg))
    }
}

fn with_port(addr: &str, port: &str) -> Result<String, String> {
    port.parse::<u16>()
        .map_err(|_| format!("invalid port: {port}"))?;
    let host = addr.rsplit_once(':').map_or(addr, |(h, _)| h);
    Ok(format!("{host}:{port}"))
}

fn home() -> PathBuf {
    // HOME on Unix, USERPROFILE on Windows.
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// `$LINKS_FILE`, else `<desktop>/links.txt`.
pub fn default_links_file() -> PathBuf {
    if let Some(v) = env::var_os("LINKS_FILE") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    let dir = xdg_desktop_dir()
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| home().join("Desktop"));
    dir.join(FILE_NAME)
}

/// Resolve the desktop directory per platform.
///
/// * Linux/BSD: `XDG_DESKTOP_DIR` from `~/.config/user-dirs.dirs`, which on a
///   localised install is not `~/Desktop` -- e.g. `"$HOME/Masaustu"`, `"$HOME/Escritorio"`.
/// * macOS: always `~/Desktop`; there is no XDG file, so the fallback applies.
/// * Windows: `~/Desktop`, or the OneDrive-redirected one when that is where it
///   actually lives.
fn xdg_desktop_dir() -> Option<PathBuf> {
    if let Some(v) = env::var_os("XDG_DESKTOP_DIR") {
        if !v.is_empty() {
            return Some(PathBuf::from(v));
        }
    }

    #[cfg(windows)]
    {
        // Redirected Desktop is common enough on Windows to be worth checking
        // before falling back to the plain profile path.
        if let Some(onedrive) = env::var_os("OneDrive") {
            let candidate = PathBuf::from(onedrive).join("Desktop");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
        Some(home().join("Desktop"))
    }

    #[cfg(not(windows))]
    {
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".config"));
        let text = fs::read_to_string(config_home.join("user-dirs.dirs")).ok()?;

        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || !line.starts_with("XDG_DESKTOP_DIR") {
                continue;
            }
            let Some(eq) = line.find('=') else { continue };
            let value = line[eq + 1..].trim().trim_matches('"');
            let expanded = match value.strip_prefix("$HOME") {
                Some(rest) => home().join(rest.trim_start_matches('/')),
                None => PathBuf::from(value),
            };
            if !expanded.as_os_str().is_empty() {
                return Some(expanded);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::with_port;

    #[test]
    fn port_override_keeps_host() {
        assert_eq!(
            with_port("127.0.0.1:9876", "1234").unwrap(),
            "127.0.0.1:1234"
        );
        assert_eq!(with_port("[::1]:9876", "80").unwrap(), "[::1]:80");
        assert!(with_port("127.0.0.1:9876", "nope").is_err());
        assert!(with_port("127.0.0.1:9876", "99999").is_err());
    }
}
