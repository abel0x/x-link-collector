// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Command line / environment configuration.
//!
//! All of it is optional. What is not given here comes from the settings file
//! the panel edits, then from the defaults. What is given wins, and the panel
//! shows those settings as fixed rather than pretending an edit would apply.

use std::env;
use std::ffi::OsString;
/// Only the XDG path reads a config file; Windows resolves the desktop directly.
#[cfg(not(windows))]
use std::fs;
use std::path::PathBuf;

use crate::settings;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_ADDR: &str = "127.0.0.1:9876";
pub const FILE_NAME: &str = "links.txt";
/// Room for a pasted list of several thousand links; anything larger is not a
/// link list.
pub const MAX_BODY: usize = 1024 * 1024;

pub struct Config {
    /// `--addr` / `--port`, or the same from the environment.
    pub addr: Option<String>,
    /// `--file` or `LINKS_FILE`.
    pub file: Option<PathBuf>,
    pub no_fsync: bool,
    pub quiet: bool,
    pub config_file: PathBuf,
    /// `--open`: show the panel in the default browser once listening.
    pub open: bool,
    /// `--no-open`: never do that, not even after a double-click.
    pub no_open: bool,
}

pub const USAGE: &str = "\
x-link-receiver -- collects links posted to a loopback HTTP endpoint, and
serves the x-link-collector panel at the same address.

USAGE:
    x-link-receiver [OPTIONS]

    Then open http://127.0.0.1:9876 in a browser. On Windows, double-clicking
    x-link-receiver.exe opens it for you.

OPTIONS:
    -a, --addr <HOST:PORT>   Listen address           [default: 127.0.0.1:9876]
    -p, --port <PORT>        Port only, host stays 127.0.0.1
    -f, --file <PATH>        Output file              [default: <desktop>/links.txt]
    -c, --config <PATH>      Settings file            [default: see below]
        --no-fsync           Flush without fsync (faster, less durable)
        --open               Open the panel in your browser once listening
        --no-open            Do not, even when started by a double-click
    -q, --quiet              Only log errors
    -h, --help               Show this help
    -V, --version            Show version

ENVIRONMENT:
    LINKS_FILE               Same as --file
    LINK_RECEIVER_ADDR       Same as --addr
    LINK_RECEIVER_PORT       Same as --port
    X_LINK_COLLECTOR_CONFIG  Same as --config

SETTINGS:
    The panel saves its settings to one JSON file, which x-download and
    x-flatten read too:
        Linux     ~/.config/x-link-collector/config.json
        macOS     ~/Library/Application Support/x-link-collector/config.json
        Windows   %APPDATA%\\x-link-collector\\config.json
    An option given above, or in the environment, wins over that file.

The default output directory is your XDG desktop (XDG_DESKTOP_DIR from
~/.config/user-dirs.dirs, which is localised -- e.g. ~/Masaustu, ~/Escritorio),
falling back to ~/Desktop.

ENDPOINTS:
    GET  /                   the panel, in a browser
    POST /                   text/plain (one URL per line) or JSON
                             ({\"url\":...} / {\"urls\":[...]} / [...])
    OPTIONS /                CORS preflight
    GET  /health             liveness + link count
    /api/...                 the panel's JSON API, loopback only
";

impl Config {
    pub fn from_args() -> Result<Option<Config>, String> {
        let mut cfg = Config {
            addr: env_value("LINK_RECEIVER_ADDR").map(|v| v.to_string_lossy().into_owned()),
            file: env_value("LINKS_FILE").map(PathBuf::from),
            no_fsync: false,
            quiet: false,
            config_file: settings::default_path(),
            open: false,
            no_open: false,
        };
        if let Some(p) = env_value("LINK_RECEIVER_PORT") {
            let base = cfg.addr.as_deref().unwrap_or(DEFAULT_ADDR);
            cfg.addr = Some(with_port(base, p.to_string_lossy().trim())?);
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
                "-a" | "--addr" => cfg.addr = Some(next(&a)?.to_string_lossy().into_owned()),
                "-p" | "--port" => {
                    let p = next(&a)?.to_string_lossy().into_owned();
                    let base = cfg.addr.as_deref().unwrap_or(DEFAULT_ADDR);
                    cfg.addr = Some(with_port(base, p.trim())?);
                }
                "-f" | "--file" => cfg.file = Some(PathBuf::from(next(&a)?)),
                "-c" | "--config" => cfg.config_file = PathBuf::from(next(&a)?),
                "--no-fsync" => cfg.no_fsync = true,
                "--open" => cfg.open = true,
                "--no-open" => cfg.no_open = true,
                "-q" | "--quiet" => cfg.quiet = true,
                other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
            }
        }
        Ok(Some(cfg))
    }

    /// The settings the command line or environment has already decided.
    pub fn pinned(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.addr.is_some() {
            out.push("port");
        }
        if self.file.is_some() {
            out.push("links_file");
        }
        if self.no_fsync {
            out.push("fsync");
        }
        out
    }
}

fn env_value(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|v| !v.is_empty())
}

fn with_port(addr: &str, port: &str) -> Result<String, String> {
    port.parse::<u16>()
        .map_err(|_| format!("invalid port: {port}"))?;
    let host = addr.rsplit_once(':').map_or(addr, |(h, _)| h);
    Ok(format!("{host}:{port}"))
}

pub fn home() -> PathBuf {
    // HOME on Unix, USERPROFILE on Windows.
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// Where `links.txt` and `x-media/` go unless told otherwise.
pub fn desktop_dir() -> PathBuf {
    xdg_desktop_dir()
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| home().join("Desktop"))
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
    use super::*;

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

    #[test]
    fn only_what_was_given_is_pinned() {
        let mut cfg = Config {
            addr: None,
            file: None,
            no_fsync: false,
            quiet: false,
            config_file: PathBuf::from("config.json"),
            open: false,
            no_open: false,
        };
        assert!(cfg.pinned().is_empty());
        cfg.file = Some(PathBuf::from("/tmp/links.txt"));
        cfg.no_fsync = true;
        assert_eq!(cfg.pinned(), ["links_file", "fsync"]);
    }
}
