// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Everything the connection threads share.
//!
//! The command line and environment are fixed for the run. The settings file
//! can change underneath through the panel, so anything that comes from it --
//! the links file, the media folder, where the tools live -- is worked out
//! when it is needed, not once at startup.

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

use crate::config::{self, Config};
use crate::history::{self, History};
use crate::jobs::{self, Jobs};
use crate::json::{self, Json};
use crate::platform;
use crate::settings::{self, Settings};
use crate::store::Store;
use crate::url;

/// Quiet time after the last capture before an automatic download starts,
/// so a burst of middle-clicks turns into one run.
const AUTO_DELAY: Duration = Duration::from_secs(20);
/// How long the media folder's numbers are trusted before it is walked again.
const MEDIA_TTL: Duration = Duration::from_secs(15);
const TOOLS_TTL: Duration = Duration::from_secs(300);

/// x-download's record of what it fetched, inside the media folder.
const STATE_FILE: &str = ".x-download-state.json";
/// x-download's MAX_ATTEMPTS: a link that failed fewer times than this is
/// tried again by a plain run, without --retry-failed.
const MAX_ATTEMPTS: f64 = 3.0;
/// When a browser extension last talked to a receiver, next to the settings:
/// once it has, the panel stops showing how to install it.
const EXTENSION_SEEN: &str = "extension-seen";
const MANIFEST: &str = ".x-flatten-manifest.json";
const VIDEO_EXTS: [&str; 6] = ["mp4", "mkv", "webm", "mov", "m4v", "gif"];
const IMAGE_EXTS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

pub struct App {
    pub cfg: Config,
    /// The port actually listened on; the panel's origin depends on it.
    pub port: u16,
    pub started: u64,
    pub jobs: Jobs,
    /// Links captured since this process started.
    pub session: AtomicUsize,
    /// Unix time of the last request from a browser extension; 0 for never.
    extension_seen: AtomicU64,
    settings: Mutex<Settings>,
    store: Mutex<Store>,
    history: Mutex<History>,
    history_warned: AtomicBool,
    auto_due: Mutex<Option<Instant>>,
    downloads: Mutex<DownloadCache>,
    media: Mutex<Option<(Instant, u64, PathBuf, MediaStats)>>,
    tools: Mutex<Option<(Instant, u64, Json)>>,
}

pub struct Download {
    pub status: String,
    pub files: Vec<String>,
    pub error: String,
    pub attempts: f64,
}

#[derive(Default)]
struct DownloadCache {
    path: PathBuf,
    stamp: Option<(SystemTime, u64)>,
    entries: Arc<HashMap<String, Download>>,
}

#[derive(Clone, Copy, Default)]
pub struct MediaStats {
    pub files: u64,
    pub bytes: u64,
    pub videos: u64,
    pub images: u64,
}

/// Poisoning is recovered from: one panicking worker must not wedge the daemon.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Why a request was refused. `code` and `vars` let the panel say it in the
/// reader's language; `text` is the English, for the log and for curl.
#[derive(Debug)]
pub struct Problem {
    pub code: &'static str,
    pub vars: Vec<(&'static str, String)>,
    pub text: String,
}

impl Problem {
    pub fn new(code: &'static str, text: impl Into<String>) -> Problem {
        Problem {
            code,
            vars: Vec::new(),
            text: text.into(),
        }
    }

    /// Set a value the panel's sentence uses, replacing any earlier one.
    pub fn with(mut self, key: &'static str, value: impl Into<String>) -> Problem {
        let value = value.into();
        match self.vars.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.vars.push((key, value)),
        }
        self
    }

    /// The cause of an I/O failure: a code the panel can word, and the
    /// system's own message for everything else.
    pub fn because(self, e: &io::Error) -> Problem {
        let reason = match e.kind() {
            io::ErrorKind::PermissionDenied => "denied",
            io::ErrorKind::NotFound => "missing",
            _ => "",
        };
        self.with("reason", reason).with("detail", e.to_string())
    }

    fn file(code: &'static str, what: &str, path: &Path, e: &io::Error) -> Problem {
        Problem::new(code, format!("{what} {}: {e}", path.display()))
            .with("path", path_text(path))
            .because(e)
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Settings that do not validate; the panel's own forms never send them.
impl From<String> for Problem {
    fn from(text: String) -> Problem {
        Problem::new("other", text)
    }
}

pub fn busy() -> Problem {
    Problem::new(
        "busy",
        "a job is already running; stop it or let it finish first",
    )
}

impl App {
    pub fn new(cfg: Config, settings: Settings, store: Store, port: u16) -> App {
        let history_file = data_dir(&cfg).join(history::FILE_NAME);
        let seen = fs::read_to_string(data_dir(&cfg).join(EXTENSION_SEEN))
            .ok()
            .and_then(|t| t.trim().parse::<u64>().ok())
            .unwrap_or(0);
        App {
            cfg,
            port,
            started: history::now(),
            jobs: Jobs::default(),
            session: AtomicUsize::new(0),
            extension_seen: AtomicU64::new(seen),
            settings: Mutex::new(settings),
            store: Mutex::new(store),
            history: Mutex::new(History::open(history_file)),
            history_warned: AtomicBool::new(false),
            auto_due: Mutex::new(None),
            downloads: Mutex::new(DownloadCache::default()),
            media: Mutex::new(None),
            tools: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> MutexGuard<'_, Settings> {
        lock(&self.settings)
    }

    pub fn store(&self) -> MutexGuard<'_, Store> {
        lock(&self.store)
    }

    pub fn history(&self) -> MutexGuard<'_, History> {
        lock(&self.history)
    }

    pub fn links_file(&self) -> PathBuf {
        resolve_links_file(&self.cfg, &self.settings())
    }

    pub fn media_dir(&self) -> PathBuf {
        resolve_media_dir(&self.settings())
    }

    fn fsync(&self) -> bool {
        !self.cfg.no_fsync && self.settings().fsync
    }

    fn restart_needed(&self, s: &Settings) -> bool {
        self.cfg.addr.is_none() && s.port != self.port
    }

    /// A browser extension made a request. Noted on disk the first time each
    /// run, so a restart does not bring back the install instructions.
    pub fn extension_contact(&self) {
        let now = history::now();
        let before = self.extension_seen.swap(now, Ordering::Relaxed);
        if before < self.started {
            let dir = data_dir(&self.cfg);
            let _ = fs::create_dir_all(&dir);
            let _ = fs::write(dir.join(EXTENSION_SEEN), now.to_string());
        }
    }

    /// A link was just written: count it, time it, and schedule a download.
    pub fn captured(&self, link: &str) {
        self.session.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = self.history().record(link) {
            if !self.history_warned.swap(true, Ordering::Relaxed) {
                elog!("cannot write the capture log: {e}");
            }
        }
        *lock(&self.auto_due) = Some(Instant::now());
    }

    /// Called by the accept loop on every tick.
    pub fn tick(&self) {
        let mut due = lock(&self.auto_due);
        let Some(at) = *due else { return };
        if !self.settings().auto_download {
            *due = None;
            return;
        }
        if at.elapsed() < AUTO_DELAY || self.jobs.running() {
            return;
        }
        *due = None;
        drop(due);
        match self.start_download(&Json::Obj(Vec::new())) {
            Ok(_) => log!("new links arrived; downloading them"),
            Err(e) => elog!("automatic download not started: {e}"),
        }
    }

    // ------------------------------------------------------------ the tools

    /// The folder holding `downloader/` and `spliter/`.
    pub fn tools_dir(&self) -> Option<PathBuf> {
        let configured = self.settings().tools_dir.clone();
        if !configured.is_empty() {
            return Some(settings::expand(&configured));
        }
        tool_dir_candidates()
            .into_iter()
            .find(|d| downloader(d).is_file())
    }

    /// The interpreter to run the tools with, and any arguments it needs first.
    pub fn python(&self) -> Option<(PathBuf, Vec<OsString>)> {
        let configured = self.settings().python.clone();
        if !configured.is_empty() {
            return Some((settings::expand(&configured), Vec::new()));
        }
        find_python()
    }

    fn tool_command(
        &self,
        script: fn(&Path) -> PathBuf,
        args: Vec<OsString>,
    ) -> Result<Command, Problem> {
        let dir = self.tools_dir().ok_or_else(|| {
            Problem::new(
                "no_tools",
                "cannot find the downloader/ and spliter/ folders; set the tools folder in Settings",
            )
        })?;
        let script = script(&dir);
        if !script.is_file() {
            return Err(Problem::new(
                "script_missing",
                format!(
                    "{} is missing; check the tools folder in Settings",
                    script.display()
                ),
            )
            .with("path", path_text(&script)));
        }
        let (python, leading) = self.python().ok_or_else(|| {
            Problem::new(
                "no_python",
                "Python 3 was not found; install it, or set its path in Settings",
            )
        })?;
        // A path typed into Settings is checked here, where the panel can say
        // what is wrong; a bare name is left for the PATH search to find.
        let has_dir = python.parent().map_or(false, |p| !p.as_os_str().is_empty());
        if has_dir && !python.is_file() {
            return Err(Problem::new(
                "python_missing",
                format!("there is no Python at {}", python.display()),
            )
            .with("path", path_text(&python)));
        }
        let mut cmd = Command::new(python);
        cmd.args(leading).arg(script).args(args).current_dir(&dir);
        // The service manager's PATH is minimal; yt-dlp still has to find ffmpeg.
        if let Ok(path) = env::join_paths(search_path()) {
            cmd.env("PATH", path);
        }
        Ok(cmd)
    }

    /// One job at a time; refused in words the panel can translate.
    fn start_job(&self, kind: &str, cmd: Command) -> Result<u64, Problem> {
        if self.jobs.running() {
            return Err(busy());
        }
        Ok(self.jobs.start(kind, cmd)?)
    }

    pub fn start_download(&self, req: &Json) -> Result<u64, Problem> {
        // One run may differ from the saved defaults; the request says how.
        let mut run = self.settings().clone();
        for key in ["jobs", "mirror", "cookies", "metadata", "timeout"] {
            if let Some(v) = req.get(key) {
                run.apply(key, v)?;
            }
        }
        // Everything is passed explicitly, so the settings file cannot pull
        // x-download in a different direction from what the panel showed.
        let mut args: Vec<OsString> = vec![
            "--no-config".into(),
            "--links".into(),
            resolve_links_file(&self.cfg, &run).into(),
            "--out".into(),
            resolve_media_dir(&run).into(),
            "--jobs".into(),
            run.jobs.to_string().into(),
            "--timeout".into(),
            run.timeout.to_string().into(),
        ];
        match run.mirror.as_str() {
            "fallback" => args.push("--mirror".into()),
            "only" => args.push("--mirror-only".into()),
            _ => {}
        }
        if !run.cookies.is_empty() {
            args.push("--cookies".into());
            args.push(settings::expand(&run.cookies).into());
        }
        if run.metadata {
            args.push("--metadata".into());
        }
        if flag(req, "retry_failed") {
            args.push("--retry-failed".into());
        }
        if flag(req, "dry_run") {
            args.push("--dry-run".into());
        }
        // A row's own Download button: just that link.
        if let Some(Json::Arr(items)) = req.get("only") {
            for link in items.iter().take(100).filter_map(Json::as_str) {
                if let Some(link) = url::sanitize(link) {
                    args.push("--only".into());
                    args.push(link.into());
                }
            }
        }
        if let Some(n) = req
            .get("limit")
            .and_then(Json::as_f64)
            .filter(|n| *n >= 1.0 && n.fract() == 0.0)
        {
            args.push("--limit".into());
            args.push(format!("{n}").into());
        }
        let id = self.start_job("download", self.tool_command(downloader, args)?)?;
        log!("download started");
        Ok(id)
    }

    pub fn start_flatten(&self, req: &Json, undo: bool) -> Result<u64, Problem> {
        let s = self.settings().clone();
        let mut args: Vec<OsString> = vec![
            "--no-config".into(),
            "--src".into(),
            resolve_media_dir(&s).into(),
        ];
        let dest = match req.get("dest").and_then(Json::as_str).map(str::trim) {
            Some(d) if !d.is_empty() => d.to_string(),
            _ => s.flatten_dest.clone(),
        };
        if !dest.is_empty() {
            args.push("--dest".into());
            args.push(settings::expand(&dest).into());
        }
        if undo {
            args.push("--undo".into());
        } else {
            for (key, option, default) in [
                ("copy", "--copy", s.flatten_copy),
                ("prefix_handle", "--prefix-handle", s.flatten_prefix_handle),
                ("videos_only", "--videos-only", s.flatten_videos_only),
            ] {
                if req.get(key).and_then(Json::as_bool).unwrap_or(default) {
                    args.push(option.into());
                }
            }
        }
        if flag(req, "dry_run") {
            args.push("--dry-run".into());
        }
        let kind = if undo { "unflatten" } else { "flatten" };
        let id = self.start_job(kind, self.tool_command(flattener, args)?)?;
        log!("{kind} started");
        Ok(id)
    }

    /// yt-dlp and gallery-dl into downloader/.venv, or upgraded there.
    pub fn start_setup(&self) -> Result<u64, Problem> {
        let cmd = self.tool_command(downloader, vec!["--setup".into()])?;
        let id = self.start_job("setup", cmd)?;
        log!("installing the download tools");
        Ok(id)
    }

    /// What the panel's Tools card shows. x-download is asked directly,
    /// because it is the one that has to find yt-dlp and gallery-dl.
    pub fn tools(&self, refresh: bool) -> Json {
        let generation = self.jobs.generation();
        if !refresh {
            if let Some((at, seen, info)) = lock(&self.tools).as_ref() {
                if at.elapsed() < TOOLS_TTL && *seen == generation {
                    return info.clone();
                }
            }
        }
        let dir = self.tools_dir();
        let python = self.python();
        let mut fields: Vec<(String, Json)> = vec![
            ("tools_dir".into(), dir.as_deref().map(path_text).into()),
            (
                "downloader".into(),
                dir.as_deref()
                    .map_or(false, |d| downloader(d).is_file())
                    .into(),
            ),
            (
                "flattener".into(),
                dir.as_deref()
                    .map_or(false, |d| flattener(d).is_file())
                    .into(),
            ),
            (
                "python".into(),
                python.as_ref().map(|(p, _)| path_text(p)).into(),
            ),
        ];
        if let Ok(cmd) = self.tool_command(downloader, vec!["--check".into(), "--json".into()]) {
            let answer = jobs::capture(cmd, Duration::from_secs(30))
                .and_then(|out| json::parse(out.trim()).map_err(|e| e.to_string()));
            match answer {
                Ok(Json::Obj(found)) => fields.extend(found),
                Ok(_) => fields.push(("error".into(), "x-download --check gave no answer".into())),
                Err(e) => fields.push(("error".into(), format!("x-download --check: {e}").into())),
            }
        }
        let info = Json::Obj(fields);
        *lock(&self.tools) = Some((Instant::now(), generation, info.clone()));
        info
    }

    pub fn open(&self, what: &str) -> Result<(), Problem> {
        let target = match what {
            "media" => self.media_dir(),
            // The file's folder, as the button says; the file itself would
            // open in whatever happens to handle .txt.
            "links" => self
                .links_file()
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            "config" => data_dir(&self.cfg),
            "tools" => self
                .tools_dir()
                .ok_or_else(|| Problem::new("no_tools", "the tools folder was not found"))?,
            _ => return Err(format!("nothing called {what:?} to open").into()),
        };
        // A folder that does not exist yet opens as an error dialog; an empty
        // one is what the person expects to see before the first download.
        if what == "media" || what == "config" {
            let _ = fs::create_dir_all(&target);
        }
        platform::open(target.as_os_str()).map_err(|e| {
            let problem = Problem::file("open", "cannot open", &target, &e);
            // What could not be started is the opener -- xdg-open, open or
            // explorer -- not the folder.
            if e.kind() == io::ErrorKind::NotFound {
                problem.with("reason", "no_opener")
            } else {
                problem
            }
        })
    }

    // ------------------------------------------------------------- settings

    pub fn settings_view(&self) -> Json {
        let s = self.settings().clone();
        let media = resolve_media_dir(&s);
        let desktop = config::desktop_dir();
        let pinned: Vec<Json> = self.cfg.pinned().into_iter().map(Json::from).collect();
        json::obj([
            ("settings", s.to_json()),
            (
                "effective",
                json::obj([
                    (
                        "links_file",
                        path_text(&resolve_links_file(&self.cfg, &s)).into(),
                    ),
                    ("media_dir", path_text(&media).into()),
                    ("flatten_dest", path_text(&flatten_dest(&s, &media)).into()),
                    ("port", self.port.into()),
                    ("fsync", (!self.cfg.no_fsync && s.fsync).into()),
                ]),
            ),
            (
                "defaults",
                json::obj([
                    (
                        "links_file",
                        path_text(&desktop.join(config::FILE_NAME)).into(),
                    ),
                    ("media_dir", path_text(&desktop.join("x-media")).into()),
                ]),
            ),
            ("pinned", pinned.into()),
            // null where the system has its own way: systemd, launchd.
            ("autostart", platform::autostart().into()),
            ("config_file", path_text(&self.cfg.config_file).into()),
            ("restart_needed", self.restart_needed(&s).into()),
        ])
    }

    /// Validate and save a partial update from the panel, then put it into
    /// effect. A new links file is opened before anything is saved, so a path
    /// that cannot be used is refused instead of written down.
    pub fn update_settings(&self, changes: &[(String, Json)]) -> Result<(), Problem> {
        let mut next = self.settings().clone();
        for (key, value) in changes {
            next.apply(key, value)?;
        }
        let links = resolve_links_file(&self.cfg, &next);
        let current = self.store().path().to_path_buf();
        let reopened = if links == current {
            None
        } else {
            let fsync = !self.cfg.no_fsync && next.fsync;
            Some(
                Store::open(links.clone(), fsync)
                    .map_err(|e| Problem::file("links_file", "cannot use", &links, &e))?,
            )
        };
        next.save(&self.cfg.config_file)
            .map_err(|e| Problem::file("save", "cannot save", &self.cfg.config_file, &e))?;
        *self.settings() = next;

        let fsync = self.fsync();
        let mut store = self.store();
        if let Some(fresh) = reopened {
            *store = fresh;
            log!(
                "collecting into {} ({} link(s) already there)",
                store.path().display(),
                store.total()
            );
        }
        store.set_fsync(fsync);
        Ok(())
    }

    // ----------------------------------------------------------- the views

    /// What the panel polls. With `strip`, also one letter per link in file
    /// order -- d(one), n(o media), f(ailed), p(ending) -- for the overview's
    /// one-square-per-link picture of the whole collection.
    pub fn status(&self, offset: i64, strip: bool) -> Json {
        let links = self.store().links().unwrap_or_default();
        let downloads = self.downloads();
        let (mut done, mut no_media, mut failed) = (0usize, 0usize, 0usize);
        // What a plain run would take on: the new links, and the failed ones
        // it has not given up on yet.
        let mut retrying = 0usize;
        let mut codes = String::with_capacity(if strip { links.len() } else { 0 });
        for link in &links {
            let download = downloads.get(&url::dedupe_key(link));
            let code = match download.map(|d| d.status.as_str()) {
                Some("done") => {
                    done += 1;
                    'd'
                }
                Some("no-media") => {
                    no_media += 1;
                    'n'
                }
                Some("failed") => {
                    failed += 1;
                    if download.map_or(0.0, |d| d.attempts) < MAX_ATTEMPTS {
                        retrying += 1;
                    }
                    'f'
                }
                _ => 'p',
            };
            if strip {
                codes.push(code);
            }
        }
        let pending = links.len() - done - no_media - failed;
        let daily = self.history().daily(14, offset);
        let media = self.media();
        let seen = self.extension_seen.load(Ordering::Relaxed);
        let s = self.settings().clone();
        let job = self.jobs.snapshot(u64::MAX).get("job").cloned();

        json::obj([
            ("version", env!("CARGO_PKG_VERSION").into()),
            ("now", history::now().into()),
            ("started", self.started.into()),
            ("port", self.port.into()),
            (
                "links_file",
                path_text(&resolve_links_file(&self.cfg, &s)).into(),
            ),
            ("media_dir", path_text(&resolve_media_dir(&s)).into()),
            ("total", links.len().into()),
            ("session", self.session.load(Ordering::Relaxed).into()),
            ("today", daily.last().copied().unwrap_or(0).into()),
            (
                "daily",
                daily.into_iter().map(Json::from).collect::<Vec<_>>().into(),
            ),
            (
                "downloads",
                json::obj([
                    ("done", done.into()),
                    ("no_media", no_media.into()),
                    ("failed", failed.into()),
                    ("pending", pending.into()),
                    ("queued", (pending + retrying).into()),
                ]),
            ),
            (
                "media",
                json::obj([
                    ("files", media.files.into()),
                    ("bytes", media.bytes.into()),
                    ("videos", media.videos.into()),
                    ("images", media.images.into()),
                    ("flattened", self.flattened(&s).into()),
                ]),
            ),
            ("extension_seen", (seen > 0).then_some(seen).into()),
            ("auto_download", s.auto_download.into()),
            ("job", job.into()),
            ("restart_needed", self.restart_needed(&s).into()),
            ("strip", strip.then_some(codes).into()),
        ])
    }

    /// One page of links, newest first, each with what x-download made of it.
    pub fn links_page(&self, query: &str, status: &str, offset: usize, limit: usize) -> Json {
        let links = self.store().links().unwrap_or_default();
        let downloads = self.downloads();
        let history = self.history();
        let query = query.trim().to_lowercase();
        let mut matched = 0usize;
        let mut items = Vec::new();
        for (index, link) in links.iter().enumerate().rev() {
            let download = downloads.get(&url::dedupe_key(link));
            let state = match download.map(|d| d.status.as_str()) {
                Some("done") => "done",
                Some("no-media") => "no-media",
                Some("failed") => "failed",
                _ => "pending",
            };
            if !query.is_empty() && !link.to_lowercase().contains(&query) {
                continue;
            }
            if !status.is_empty() && status != "all" && status != state {
                continue;
            }
            matched += 1;
            if matched <= offset || items.len() >= limit {
                continue;
            }
            let tweet = url::tweet_ref(link);
            let files: Vec<Json> = download
                .map(|d| d.files.iter().map(|f| f.as_str().into()).collect())
                .unwrap_or_default();
            items.push(json::obj([
                // Its place in links.txt, so a row can be found in the file.
                ("line", (index + 1).into()),
                ("url", link.as_str().into()),
                ("id", tweet.as_ref().map(|t| t.id.clone()).into()),
                ("handle", tweet.and_then(|t| t.handle).into()),
                ("captured", history.captured(link).into()),
                ("status", state.into()),
                ("files", files.into()),
                ("error", download.map_or("", |d| d.error.as_str()).into()),
            ]));
        }
        json::obj([
            ("total", links.len().into()),
            ("matched", matched.into()),
            ("items", items.into()),
        ])
    }

    pub fn remove_link(&self, link: &str) -> Result<bool, Problem> {
        let removed = self.store().remove(link).map_err(|e| {
            Problem::new("rewrite", format!("cannot rewrite the links file: {e}")).because(&e)
        })?;
        if removed {
            log!("- {link} (removed in the panel)");
        }
        Ok(removed)
    }

    /// x-download's state file, keyed the way the store dedupes. It is read
    /// again only when it has changed.
    fn downloads(&self) -> Arc<HashMap<String, Download>> {
        let path = self.media_dir().join(STATE_FILE);
        let stamp = fs::metadata(&path)
            .ok()
            .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len()));
        let mut cache = lock(&self.downloads);
        if cache.path != path || cache.stamp != stamp {
            cache.entries = Arc::new(read_downloads(&path));
            cache.path = path;
            cache.stamp = stamp;
        }
        Arc::clone(&cache.entries)
    }

    fn media(&self) -> MediaStats {
        let dir = self.media_dir();
        let generation = self.jobs.generation();
        let mut cache = lock(&self.media);
        if let Some((at, seen, cached_dir, stats)) = cache.as_ref() {
            if at.elapsed() < MEDIA_TTL && *seen == generation && *cached_dir == dir {
                return *stats;
            }
        }
        let stats = scan_media(&dir);
        *cache = Some((Instant::now(), generation, dir, stats));
        stats
    }

    /// Whether x-flatten left a manifest that `--undo` can walk back.
    fn flattened(&self, s: &Settings) -> bool {
        flatten_dest(s, &resolve_media_dir(s))
            .join(MANIFEST)
            .is_file()
    }
}

/// The settings file's folder, which also holds the capture log.
fn data_dir(cfg: &Config) -> PathBuf {
    cfg.config_file
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

pub fn resolve_links_file(cfg: &Config, s: &Settings) -> PathBuf {
    if let Some(file) = &cfg.file {
        return file.clone();
    }
    if !s.links_file.is_empty() {
        return settings::expand(&s.links_file);
    }
    config::desktop_dir().join(config::FILE_NAME)
}

pub fn resolve_media_dir(s: &Settings) -> PathBuf {
    if !s.media_dir.is_empty() {
        return settings::expand(&s.media_dir);
    }
    config::desktop_dir().join("x-media")
}

fn flatten_dest(s: &Settings, media: &Path) -> PathBuf {
    if s.flatten_dest.is_empty() {
        media.join("x-media")
    } else {
        settings::expand(&s.flatten_dest)
    }
}

fn downloader(dir: &Path) -> PathBuf {
    dir.join("downloader").join("x-download")
}

fn flattener(dir: &Path) -> PathBuf {
    dir.join("spliter").join("x-flatten")
}

fn flag(req: &Json, key: &str) -> bool {
    req.get(key).and_then(Json::as_bool).unwrap_or(false)
}

fn path_text(p: &Path) -> String {
    p.display().to_string()
}

fn tool_dir_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = env::current_exe().ok().as_deref().and_then(Path::parent) {
        // A release archive: the tools sit next to the binary.
        out.push(dir.to_path_buf());
        // A build tree: <repo>/receiver/target/[<triple>/]release/.
        out.extend(dir.ancestors().skip(3).take(2).map(Path::to_path_buf));
    }
    // The checkout this binary was built from -- where the tools are for
    // anyone who ran `make install` or `make service`.
    if let Some(repo) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() {
        out.push(repo.to_path_buf());
    }
    out
}

fn find_python() -> Option<(PathBuf, Vec<OsString>)> {
    // On Windows the py launcher goes first: a bare python.exe may be the
    // Store's placeholder, which only offers to install Python. On Unix,
    // python3 first -- plain `python` is still Python 2 on some systems.
    let candidates: &[(&str, &[&str])] = if cfg!(windows) {
        &[
            ("py.exe", &["-3"]),
            ("python.exe", &[]),
            ("python3.exe", &[]),
        ]
    } else {
        &[("python3", &[]), ("python", &[])]
    };
    let dirs = search_path();
    candidates.iter().find_map(|(name, leading)| {
        dirs.iter()
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .map(|p| (p, leading.iter().map(OsString::from).collect()))
    })
}

/// `$PATH`, plus the places a service manager's minimal PATH leaves out but
/// pip, pipx and Homebrew put programs.
fn search_path() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();
    if !cfg!(windows) {
        for extra in [
            config::home().join(".local").join("bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ] {
            if !dirs.contains(&extra) {
                dirs.push(extra);
            }
        }
    }
    dirs
}

fn read_downloads(path: &Path) -> HashMap<String, Download> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(state) = json::parse(&text) else {
        return HashMap::new();
    };
    let Some(Json::Obj(entries)) = state.get("entries") else {
        return HashMap::new();
    };
    entries
        .iter()
        .map(|(link, entry)| {
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(Json::as_str)
                    .unwrap_or("")
                    .to_string()
            };
            let files = match entry.get("files") {
                Some(Json::Arr(items)) => items
                    .iter()
                    .filter_map(Json::as_str)
                    .map(str::to_string)
                    .collect(),
                _ => Vec::new(),
            };
            let download = Download {
                status: text("status"),
                files,
                error: text("error"),
                attempts: entry.get("attempts").and_then(Json::as_f64).unwrap_or(0.0),
            };
            (url::dedupe_key(link), download)
        })
        .collect()
}

/// Count what is in the media folder. Hidden files -- the state file, the
/// flatten manifest -- and half-finished downloads are not media.
fn scan_media(root: &Path) -> MediaStats {
    let mut stats = MediaStats::default();
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name.ends_with(".part") {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() && depth < 4 {
                pending.push((entry.path(), depth + 1));
            } else if kind.is_file() {
                stats.files += 1;
                stats.bytes += entry.metadata().map_or(0, |m| m.len());
                let ext = Path::new(&*name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if VIDEO_EXTS.contains(&ext.as_str()) {
                    stats.videos += 1;
                } else if IMAGE_EXTS.contains(&ext.as_str()) {
                    stats.images += 1;
                }
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("x-link-app-{tag}-{n}"))
    }

    #[test]
    fn reads_the_downloaders_state_by_tweet_id() {
        let dir = temp_dir("state");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(STATE_FILE);
        fs::write(
            &path,
            r#"{"version": 1, "entries": {
                "https://x.com/a/status/1": {"status": "done", "files": ["a/1-1.mp4"], "attempts": 1},
                "https://x.com/b/status/2": {"status": "failed", "error": "rate limited by X"}
            }}"#,
        )
        .unwrap();
        let states = read_downloads(&path);
        let done = &states[&url::dedupe_key("https://twitter.com/other/status/1")];
        assert_eq!((done.status.as_str(), done.files.len()), ("done", 1));
        assert_eq!(
            states[&url::dedupe_key("https://x.com/b/status/2")].error,
            "rate limited by X"
        );
        assert!(read_downloads(&dir.join("missing.json")).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn problems_carry_what_the_panel_needs_to_word_them() {
        let e = io::Error::new(io::ErrorKind::PermissionDenied, "nope");
        let p = Problem::file("save", "cannot save", Path::new("/x/config.json"), &e);
        assert_eq!(
            (p.code, p.text.as_str()),
            ("save", "cannot save /x/config.json: nope")
        );
        let var = |p: &Problem, key: &str| {
            p.vars
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(var(&p, "path").as_deref(), Some("/x/config.json"));
        assert_eq!(var(&p, "reason").as_deref(), Some("denied"));
        assert_eq!(var(&p, "detail").as_deref(), Some("nope"));

        // A later value replaces an earlier one rather than adding a second.
        let p = p.with("reason", "no_opener");
        assert_eq!(var(&p, "reason").as_deref(), Some("no_opener"));
        assert_eq!(p.vars.len(), 3);

        let other = Problem::from("jobs must be a whole number".to_string());
        assert_eq!(
            (other.code, other.to_string().as_str()),
            ("other", "jobs must be a whole number")
        );
    }

    #[test]
    fn counts_media_but_not_bookkeeping() {
        let dir = temp_dir("media");
        fs::create_dir_all(dir.join("alice")).unwrap();
        fs::write(dir.join("alice/1-1.mp4"), b"0123456789").unwrap();
        fs::write(dir.join("alice/2-1.jpg"), b"01234").unwrap();
        fs::write(dir.join("alice/3-1.mp4.part"), b"partial").unwrap();
        fs::write(dir.join(STATE_FILE), b"{}").unwrap();
        let stats = scan_media(&dir);
        assert_eq!(
            (stats.files, stats.bytes, stats.videos, stats.images),
            (2, 15, 1, 1)
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
