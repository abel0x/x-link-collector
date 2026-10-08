// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The settings file behind the panel.
//!
//! One small JSON object that the receiver, x-download and x-flatten all read,
//! so a choice made in the panel holds when the tools are run from a terminal
//! too. Keys this version does not know are carried through a save untouched:
//! an older receiver never strips what a newer one wrote.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config;
use crate::json::{self, Json};
use crate::store;

pub const APP_DIR: &str = "x-link-collector";
pub const FILE_NAME: &str = "config.json";
pub const DEFAULT_PORT: u16 = 9876;

const MIRROR_MODES: [&str; 3] = ["off", "fallback", "only"];

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Empty means the default, `<desktop>/links.txt`.
    pub links_file: String,
    /// Empty means `<desktop>/x-media`.
    pub media_dir: String,
    pub port: u16,
    pub fsync: bool,
    pub jobs: u32,
    /// "off", "fallback" (only once X has refused) or "only".
    pub mirror: String,
    /// A browser name or a cookies.txt path; empty for none.
    pub cookies: String,
    pub metadata: bool,
    pub timeout: u32,
    /// Start a download by itself shortly after new links arrive.
    pub auto_download: bool,
    /// Empty means `<media dir>/x-media`, x-flatten's own default.
    pub flatten_dest: String,
    pub flatten_prefix_handle: bool,
    pub flatten_videos_only: bool,
    pub flatten_copy: bool,
    /// The folder holding downloader/ and spliter/; empty means "find it".
    pub tools_dir: String,
    /// The Python interpreter; empty means "find it".
    pub python: String,
    unknown: Vec<(String, Json)>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            links_file: String::new(),
            media_dir: String::new(),
            port: DEFAULT_PORT,
            fsync: true,
            jobs: 3,
            mirror: "off".to_string(),
            cookies: String::new(),
            metadata: false,
            timeout: 900,
            auto_download: false,
            flatten_dest: String::new(),
            flatten_prefix_handle: false,
            flatten_videos_only: false,
            flatten_copy: false,
            tools_dir: String::new(),
            python: String::new(),
            unknown: Vec::new(),
        }
    }
}

impl Settings {
    /// Read the file at `path`. A missing file is simply the defaults; a broken
    /// one is the defaults plus a warning, and stays untouched until a save.
    pub fn load(path: &Path) -> (Settings, Option<String>) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (Settings::default(), None),
            Err(e) => {
                let warning = format!("cannot read {}: {e}; using defaults", path.display());
                return (Settings::default(), Some(warning));
            }
        };
        // A byte-order mark from a Windows editor is not a reason to give up.
        let fields = match json::parse(text.trim_start_matches('\u{feff}')) {
            Ok(Json::Obj(fields)) => fields,
            Ok(_) => {
                let warning = format!("{} is not a JSON object; using defaults", path.display());
                return (Settings::default(), Some(warning));
            }
            Err(e) => {
                let warning = format!("{} is not valid JSON ({e}); using defaults", path.display());
                return (Settings::default(), Some(warning));
            }
        };

        let mut settings = Settings::default();
        let mut problems = Vec::new();
        for (key, value) in fields {
            if KEYS.contains(&key.as_str()) {
                if let Err(e) = settings.apply(&key, &value) {
                    problems.push(e);
                }
            } else {
                settings.unknown.push((key, value));
            }
        }
        let warning = (!problems.is_empty())
            .then(|| format!("{}: ignoring {}", path.display(), problems.join("; ")));
        (settings, warning)
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir)?;
            }
        }
        store::write_atomic(path, self.to_json().pretty().as_bytes(), true)
    }

    /// Set one key from a JSON value, with the same limits the panel's form has.
    pub fn apply(&mut self, key: &str, v: &Json) -> Result<(), String> {
        match key {
            "links_file" => self.links_file = text(key, v)?,
            "media_dir" => self.media_dir = text(key, v)?,
            "port" => self.port = number(key, v, 1024, 65_535)? as u16,
            "fsync" => self.fsync = boolean(key, v)?,
            "jobs" => self.jobs = number(key, v, 1, 10)?,
            "mirror" => self.mirror = choice(key, v, &MIRROR_MODES)?,
            "cookies" => self.cookies = text(key, v)?,
            "metadata" => self.metadata = boolean(key, v)?,
            "timeout" => self.timeout = number(key, v, 30, 86_400)?,
            "auto_download" => self.auto_download = boolean(key, v)?,
            "flatten_dest" => self.flatten_dest = text(key, v)?,
            "flatten_prefix_handle" => self.flatten_prefix_handle = boolean(key, v)?,
            "flatten_videos_only" => self.flatten_videos_only = boolean(key, v)?,
            "flatten_copy" => self.flatten_copy = boolean(key, v)?,
            "tools_dir" => self.tools_dir = text(key, v)?,
            "python" => self.python = text(key, v)?,
            _ => return Err(format!("unknown setting {key:?}")),
        }
        Ok(())
    }

    pub fn to_json(&self) -> Json {
        let mut out = json::obj([
            ("links_file", self.links_file.as_str().into()),
            ("media_dir", self.media_dir.as_str().into()),
            ("port", self.port.into()),
            ("fsync", self.fsync.into()),
            ("jobs", self.jobs.into()),
            ("mirror", self.mirror.as_str().into()),
            ("cookies", self.cookies.as_str().into()),
            ("metadata", self.metadata.into()),
            ("timeout", self.timeout.into()),
            ("auto_download", self.auto_download.into()),
            ("flatten_dest", self.flatten_dest.as_str().into()),
            ("flatten_prefix_handle", self.flatten_prefix_handle.into()),
            ("flatten_videos_only", self.flatten_videos_only.into()),
            ("flatten_copy", self.flatten_copy.into()),
            ("tools_dir", self.tools_dir.as_str().into()),
            ("python", self.python.as_str().into()),
        ]);
        if let Json::Obj(fields) = &mut out {
            fields.extend(self.unknown.iter().cloned());
        }
        out
    }
}

pub const KEYS: [&str; 16] = [
    "links_file",
    "media_dir",
    "port",
    "fsync",
    "jobs",
    "mirror",
    "cookies",
    "metadata",
    "timeout",
    "auto_download",
    "flatten_dest",
    "flatten_prefix_handle",
    "flatten_videos_only",
    "flatten_copy",
    "tools_dir",
    "python",
];

fn boolean(key: &str, v: &Json) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("{key} must be true or false"))
}

fn number(key: &str, v: &Json, min: u32, max: u32) -> Result<u32, String> {
    match v.as_f64() {
        Some(n) if n.fract() == 0.0 && n >= f64::from(min) && n <= f64::from(max) => Ok(n as u32),
        _ => Err(format!("{key} must be a whole number from {min} to {max}")),
    }
}

fn choice(key: &str, v: &Json, options: &[&str]) -> Result<String, String> {
    match v.as_str() {
        Some(s) if options.contains(&s) => Ok(s.to_string()),
        _ => Err(format!("{key} must be one of: {}", options.join(", "))),
    }
}

/// A path or a name typed into a form: trimmed, and nothing a file name or a
/// command line would choke on.
fn text(key: &str, v: &Json) -> Result<String, String> {
    let s = v
        .as_str()
        .ok_or_else(|| format!("{key} must be text"))?
        .trim();
    if s.len() > 4096 || s.chars().any(char::is_control) {
        return Err(format!("{key} is not a usable value"));
    }
    Ok(s.to_string())
}

/// `$X_LINK_COLLECTOR_CONFIG`, else the per-user config folder:
/// `~/.config` on Linux, `~/Library/Application Support` on macOS,
/// `%APPDATA%` on Windows.
pub fn default_path() -> PathBuf {
    if let Some(v) = env::var_os("X_LINK_COLLECTOR_CONFIG") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    config_dir().join(APP_DIR).join(FILE_NAME)
}

fn config_dir() -> PathBuf {
    let home = config::home();
    if cfg!(windows) {
        env::var_os("APPDATA")
            .filter(|v| !v.is_empty())
            .map_or_else(|| home.join("AppData").join("Roaming"), PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map_or_else(|| home.join(".config"), PathBuf::from)
    }
}

/// `~/x` -> `<home>/x`. Anything else is taken as written.
pub fn expand(raw: &str) -> PathBuf {
    match raw.strip_prefix('~') {
        Some("") => config::home(),
        Some(rest) if rest.starts_with(['/', '\\']) => config::home().join(&rest[1..]),
        _ => PathBuf::from(raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("x-link-settings-{tag}-{n}/config.json"))
    }

    #[test]
    fn missing_file_means_defaults() {
        let (s, warning) = Settings::load(Path::new("/nonexistent/x-link/config.json"));
        assert_eq!(s, Settings::default());
        assert!(warning.is_none());
    }

    #[test]
    fn round_trips_and_keeps_unknown_keys() {
        let p = temp_file("roundtrip");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(
            &p,
            "\u{feff}{\"jobs\": 4, \"mirror\": \"only\", \"from_the_future\": [1, 2]}",
        )
        .unwrap();
        let (mut s, warning) = Settings::load(&p);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!((s.jobs, s.mirror.as_str()), (4, "only"));

        s.apply("cookies", &Json::from("  vivaldi ")).unwrap();
        s.save(&p).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("\"from_the_future\": [\n"), "{text}");
        let (again, _) = Settings::load(&p);
        assert_eq!(again, s);
        assert_eq!(again.cookies, "vivaldi");
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }

    #[test]
    fn bad_values_are_refused_one_by_one() {
        let mut s = Settings::default();
        assert!(s.apply("jobs", &Json::Num(0.0)).is_err());
        assert!(s.apply("jobs", &Json::Num(2.5)).is_err());
        assert!(s.apply("port", &Json::Num(80.0)).is_err());
        assert!(s.apply("mirror", &Json::from("sometimes")).is_err());
        assert!(s.apply("fsync", &Json::from("yes")).is_err());
        assert!(s.apply("media_dir", &Json::from("a\nb")).is_err());
        assert!(s.apply("nope", &Json::Null).is_err());
        assert_eq!(s, Settings::default(), "nothing half-applied");

        let p = temp_file("partial");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, r#"{"jobs": 99, "timeout": 120}"#).unwrap();
        let (s, warning) = Settings::load(&p);
        assert_eq!((s.jobs, s.timeout), (3, 120), "the good key still loads");
        assert!(warning.unwrap().contains("jobs"));
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }

    #[test]
    fn expands_home() {
        let home = config::home();
        assert_eq!(expand("~"), home);
        assert_eq!(expand("~/x-media"), home.join("x-media"));
        assert_eq!(expand("/srv/x"), PathBuf::from("/srv/x"));
        assert_eq!(expand("~user/x"), PathBuf::from("~user/x"));
    }
}
