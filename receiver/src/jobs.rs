// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Running the Python tools for the panel.
//!
//! One job at a time: a download, a flatten, installing the tools. Its output
//! goes into a ring of recent lines the panel polls with a cursor, and its
//! progress is read off the `[ n/total]` prefix x-download already prints.
//! The tools stay ordinary command line programs -- the panel starts them the
//! way a person would, and stops them the way Ctrl-C does.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::history::now;
use crate::json::{self, Json};
use crate::platform;

/// Output lines kept for the panel; older ones scroll away.
const MAX_LINES: usize = 4000;
const MAX_LINE_LEN: usize = 2000;
/// After the Ctrl-C equivalent, how long a job gets before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(8);
/// After a job exits, how long its last output may take to arrive. Only a
/// grandchild that outlives it and keeps the pipe open makes this wait.
const OUTPUT_GRACE: Duration = Duration::from_secs(3);

#[derive(Clone, Default)]
pub struct Jobs {
    inner: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    last_id: u64,
    job: Option<Job>,
    lines: VecDeque<Line>,
    last_line: u64,
    /// The job's latest stdout line, which becomes its summary.
    last_out: String,
    /// Bumped whenever a job ends, so caches built from its results know.
    finished: u64,
}

struct Line {
    n: u64,
    err: bool,
    text: String,
}

struct Job {
    id: u64,
    kind: String,
    pid: u32,
    started: u64,
    ended: Option<u64>,
    /// `None` while running, and when a signal ended it.
    code: Option<i32>,
    stopping: Arc<AtomicBool>,
    done: u32,
    total: u32,
    summary: String,
}

impl Jobs {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn running(&self) -> bool {
        self.lock()
            .job
            .as_ref()
            .map_or(false, |j| j.ended.is_none())
    }

    /// Counts finished jobs; it changes when one ends.
    pub fn generation(&self) -> u64 {
        self.lock().finished
    }

    /// Start `cmd` as the new job, unless one is still running.
    pub fn start(&self, kind: &str, mut cmd: Command) -> Result<u64, String> {
        let mut state = self.lock();
        if state.job.as_ref().map_or(false, |j| j.ended.is_none()) {
            return Err("another job is still running".to_string());
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Line by line, and UTF-8 whatever the console would have been:
            // a non-ASCII file name must not end a run on Windows.
            .env("PYTHONUNBUFFERED", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1");
        platform::detach(&mut cmd);
        let shown = describe(&cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", cmd.get_program().to_string_lossy()))?;

        state.last_id += 1;
        let id = state.last_id;
        let stopping = Arc::new(AtomicBool::new(false));
        state.job = Some(Job {
            id,
            kind: kind.to_string(),
            pid: child.id(),
            started: now(),
            ended: None,
            code: None,
            stopping: Arc::clone(&stopping),
            done: 0,
            total: 0,
            summary: String::new(),
        });
        state.lines.clear();
        state.last_out.clear();
        push(&mut state, false, format!("$ {shown}"));
        drop(state);

        // Output readers still open; the job's last line waits for them.
        let open = Arc::new(AtomicUsize::new(0));
        if let Some(out) = child.stdout.take() {
            self.pump(out, false, &open);
        }
        if let Some(err) = child.stderr.take() {
            self.pump(err, true, &open);
        }
        let jobs = self.clone();
        thread::Builder::new()
            .name("job".to_string())
            .spawn(move || jobs.supervise(id, child, &stopping, &open))
            .map_err(|e| format!("cannot watch the job: {e}"))?;
        Ok(id)
    }

    /// Ask the running job to stop. Returns `false` when nothing was running.
    pub fn stop(&self) -> bool {
        match self.lock().job.as_ref() {
            Some(job) if job.ended.is_none() => {
                job.stopping.store(true, Ordering::Relaxed);
                true
            }
            _ => false,
        }
    }

    /// On the way out: stop a running job and wait a little, so a receiver
    /// stopped with Ctrl-C does not leave a download running behind it.
    pub fn shutdown(&self, limit: Duration) {
        if !self.stop() {
            return;
        }
        let deadline = Instant::now() + limit;
        while self.running() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
        if let Some(job) = self.lock().job.as_ref().filter(|j| j.ended.is_none()) {
            platform::kill_tree(job.pid);
        }
    }

    /// The current or last job, plus the output lines after `since`.
    pub fn snapshot(&self, since: u64) -> Json {
        let state = self.lock();
        let job = state.job.as_ref().map(|j| {
            let phase = match (j.ended, j.code) {
                (None, _) if j.stopping.load(Ordering::Relaxed) => "stopping",
                (None, _) => "running",
                (Some(_), _) if j.stopping.load(Ordering::Relaxed) => "stopped",
                (Some(_), Some(0)) => "done",
                (Some(_), _) => "failed",
            };
            json::obj([
                ("id", j.id.into()),
                ("kind", j.kind.as_str().into()),
                ("state", phase.into()),
                ("started", j.started.into()),
                ("ended", j.ended.into()),
                ("code", j.code.into()),
                ("done", j.done.into()),
                ("total", j.total.into()),
                ("summary", j.summary.as_str().into()),
            ])
        });
        let lines: Vec<Json> = state
            .lines
            .iter()
            .filter(|l| l.n > since)
            .map(|l| {
                json::obj([
                    ("n", l.n.into()),
                    ("err", l.err.into()),
                    ("text", l.text.as_str().into()),
                ])
            })
            .collect();
        json::obj([
            ("job", job.into()),
            ("lines", lines.into()),
            ("next", state.last_line.into()),
        ])
    }

    fn pump<R: Read + Send + 'static>(&self, stream: R, err: bool, open: &Arc<AtomicUsize>) {
        let jobs = self.clone();
        let open = Arc::clone(open);
        open.fetch_add(1, Ordering::SeqCst);
        let still_open = Arc::clone(&open);
        let spawned = thread::Builder::new()
            .name("job-output".to_string())
            .spawn(move || {
                let mut reader = BufReader::new(stream);
                let mut buf = Vec::new();
                loop {
                    buf.clear();
                    match reader.read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let text = String::from_utf8_lossy(&buf);
                            // A progress bar redraws itself with \r; keep the
                            // final state of the line, not every frame.
                            let text = text.trim_end_matches(['\n', '\r']);
                            let text = text.rsplit('\r').next().unwrap_or(text);
                            jobs.line(err, text);
                        }
                    }
                }
                still_open.fetch_sub(1, Ordering::SeqCst);
            });
        if spawned.is_err() {
            open.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn line(&self, err: bool, text: &str) {
        let mut state = self.lock();
        if let Some(job) = state.job.as_mut() {
            if let Some((done, total)) = progress(text) {
                job.done = done;
                job.total = total;
            }
        }
        if !err && !text.trim().is_empty() {
            state.last_out = text.trim().to_string();
        }
        push(&mut state, err, text.to_string());
    }

    fn supervise(&self, id: u64, mut child: Child, stopping: &AtomicBool, open: &AtomicUsize) {
        let pid = child.id();
        let mut signalled: Option<Instant> = None;
        let mut killed = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {}
                Err(e) => break Err(e),
            }
            if stopping.load(Ordering::Relaxed) {
                match signalled {
                    None => {
                        platform::interrupt_tree(pid);
                        signalled = Some(Instant::now());
                    }
                    Some(at) if !killed && at.elapsed() > STOP_GRACE => {
                        platform::kill_tree(pid);
                        let _ = child.kill();
                        killed = true;
                    }
                    _ => {}
                }
            }
            thread::sleep(Duration::from_millis(100));
        };
        let drained = Instant::now() + OUTPUT_GRACE;
        while open.load(Ordering::SeqCst) > 0 && Instant::now() < drained {
            thread::sleep(Duration::from_millis(20));
        }

        let mut state = self.lock();
        let (code, note) = match status {
            Ok(s) => match s.code() {
                Some(0) => (Some(0), "finished".to_string()),
                Some(c) => (Some(c), format!("exited with code {c}")),
                None => (None, "ended by a signal".to_string()),
            },
            Err(e) => (None, format!("lost track of the process: {e}")),
        };
        let stopped = stopping.load(Ordering::Relaxed);
        let summary = state.last_out.clone();
        if let Some(job) = state.job.as_mut().filter(|j| j.id == id) {
            job.ended = Some(now());
            job.code = code;
            job.summary = summary;
        }
        state.finished += 1;
        let note = if stopped { "stopped".to_string() } else { note };
        push(
            &mut state,
            code != Some(0) && !stopped,
            format!("-- {note}"),
        );
    }
}

fn push(state: &mut State, err: bool, mut text: String) {
    if text.len() > MAX_LINE_LEN {
        let mut cut = MAX_LINE_LEN;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push('…');
    }
    state.last_line += 1;
    let n = state.last_line;
    state.lines.push_back(Line { n, err, text });
    while state.lines.len() > MAX_LINES {
        state.lines.pop_front();
    }
}

/// `[ 12/300] ok ...` -> `(12, 300)`
fn progress(line: &str) -> Option<(u32, u32)> {
    let inner = line.strip_prefix('[')?.split_once(']')?.0;
    let (done, total) = inner.split_once('/')?;
    Some((done.trim().parse().ok()?, total.trim().parse().ok()?))
}

/// The command as a person would type it, for the top of the log.
fn describe(cmd: &Command) -> String {
    let mut out = cmd.get_program().to_string_lossy().into_owned();
    for arg in cmd.get_args() {
        let arg = arg.to_string_lossy();
        if arg.is_empty() || arg.contains([' ', '"', '\'']) {
            out.push_str(&format!(" {arg:?}"));
        } else {
            out.push(' ');
            out.push_str(&arg);
        }
    }
    out
}

/// Run a short command to completion and return its stdout, or give up after
/// `limit`. Used for questions like "which tools are installed".
pub fn capture(mut cmd: Command, limit: Duration) -> Result<String, String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1");
    platform::detach(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut out = child.stdout.take().ok_or("no stdout")?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let _ = out.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("no answer within {}s", limit.as_secs()));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    rx.recv_timeout(Duration::from_secs(2))
        .map_err(|_| "output did not arrive".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_progress_prefixes() {
        assert_eq!(progress("[ 3/12] ok  a/1-1.mp4  (2.0 MB)"), Some((3, 12)));
        assert_eq!(
            progress("[12/12] --  no media in https://x"),
            Some((12, 12))
        );
        assert_eq!(progress("[x/12] ok"), None);
        assert_eq!(progress("done: 3 downloaded"), None);
        assert_eq!(progress("[noslash] hi"), None);
    }

    #[test]
    fn long_lines_are_cut_on_a_char_boundary() {
        let mut state = State::default();
        push(&mut state, false, "ü".repeat(MAX_LINE_LEN));
        let line = &state.lines[0].text;
        assert!(line.len() <= MAX_LINE_LEN + '…'.len_utf8());
        assert!(line.ends_with('…'));
    }

    #[test]
    fn ring_keeps_only_recent_lines() {
        let mut state = State::default();
        for i in 0..MAX_LINES + 10 {
            push(&mut state, false, i.to_string());
        }
        assert_eq!(state.lines.len(), MAX_LINES);
        assert_eq!(state.lines[0].text, "10");
        assert_eq!(state.last_line, (MAX_LINES + 10) as u64);
    }

    #[test]
    fn quotes_arguments_that_need_it() {
        let mut cmd = Command::new("python3");
        cmd.args(["x-download", "--out", "/home/a b/x-media", "--cookies", ""]);
        assert_eq!(
            describe(&cmd),
            r#"python3 x-download --out "/home/a b/x-media" --cookies """#
        );
    }

    #[cfg(unix)]
    #[test]
    fn runs_stops_and_reports_a_job() {
        let jobs = Jobs::default();
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "echo '[ 1/2] ok first'; echo oops >&2; echo '[ 2/2] ok second'; echo 'done: 2'",
        ]);
        jobs.start("download", cmd).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while jobs.running() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        // No settling time: once the job reads as ended, all of its output is in.
        let snap = jobs.snapshot(0);
        let text = snap.to_string();
        assert!(text.contains(r#""state":"done""#), "{text}");
        assert!(text.contains(r#""done":2,"total":2"#), "{text}");
        assert!(text.contains(r#""summary":"done: 2""#), "{text}");
        assert!(text.contains(r#""err":true,"text":"oops""#), "{text}");
        let Some(Json::Arr(lines)) = snap.get("lines") else {
            panic!("no lines in {text}")
        };
        assert_eq!(lines.len(), 6, "{text}");
        let last = lines
            .last()
            .and_then(|l| l.get("text"))
            .and_then(Json::as_str);
        assert_eq!(last, Some("-- finished"));
        assert_eq!(jobs.generation(), 1);

        let mut slow = Command::new("sh");
        slow.args(["-c", "sleep 30"]);
        jobs.start("download", slow).unwrap();
        assert!(
            jobs.start("flatten", Command::new("true")).is_err(),
            "one at a time"
        );
        assert!(jobs.stop());
        let deadline = Instant::now() + Duration::from_secs(10);
        while jobs.running() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(!jobs.running(), "SIGINT should end sleep promptly");
        assert!(jobs
            .snapshot(0)
            .to_string()
            .contains(r#""state":"stopped""#));
    }
}
