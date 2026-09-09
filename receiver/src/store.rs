// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Append-only link file with in-memory deduplication.
//!
//! Design notes:
//! * The file is opened per write (`O_APPEND`) rather than held open, so the
//!   daemon keeps working if you delete, move or rotate `links.txt` while it
//!   runs -- the next write simply recreates it.
//! * Each line is written with a single `write` call, which `O_APPEND` makes
//!   atomic, then flushed (and `fsync`ed unless `--no-fsync`), so a link is on
//!   disk before the extension is told it may close the tab.
//! * If the file changes size behind our back (you edited or emptied it), the
//!   dedupe set is rebuilt from disk before the next write.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::url;

pub struct Store {
    path: PathBuf,
    seen: HashSet<String>,
    known_len: u64,
    fsync: bool,
    total: usize,
}

impl Store {
    pub fn open(path: PathBuf, fsync: bool) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir)?;
            }
        }
        let mut store = Store {
            path,
            seen: HashSet::new(),
            known_len: 0,
            fsync,
            total: 0,
        };
        store.reload()?;
        // Touch the file so an empty collection is visible on the desktop and
        // permission problems surface at startup instead of on first capture.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&store.path)?;
        store.known_len = file_len(&store.path);
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn total(&self) -> usize {
        self.total
    }

    /// Append `url` unless an equivalent link is already recorded.
    /// Returns `true` when a new line was written.
    pub fn add(&mut self, url: &str) -> io::Result<bool> {
        self.refresh_if_changed();

        let key = url::dedupe_key(url);
        if self.seen.contains(&key) {
            return Ok(false);
        }

        let mut line = String::with_capacity(url.len() + 1);
        line.push_str(url);
        line.push('\n');

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(line.as_bytes())?;
        f.flush()?;
        if self.fsync {
            f.sync_data()?;
        }

        self.seen.insert(key);
        self.total += 1;
        self.known_len = f
            .metadata()
            .map_or(self.known_len + line.len() as u64, |m| m.len());
        Ok(true)
    }

    /// Rebuild the dedupe set from disk (used at startup and after external edits).
    fn reload(&mut self) -> io::Result<()> {
        self.seen.clear();
        self.total = 0;
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                for line in text.lines() {
                    let l = line.trim();
                    if l.is_empty() || l.starts_with('#') {
                        continue;
                    }
                    self.total += 1;
                    self.seen.insert(url::dedupe_key(l));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        self.known_len = file_len(&self.path);
        Ok(())
    }

    fn refresh_if_changed(&mut self) {
        if file_len(&self.path) != self.known_len {
            let _ = self.reload();
        }
    }
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map_or(0, |m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("x-link-receiver-{tag}-{n}/links.txt"))
    }

    fn cleanup(p: &Path) {
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn creates_file_and_appends() {
        let p = temp_path("append");
        let mut s = Store::open(p.clone(), false).unwrap();
        assert!(p.exists(), "file should be created eagerly");
        assert!(s.add("https://x.com/a/status/1").unwrap());
        assert!(s.add("https://x.com/b/status/2").unwrap());
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "https://x.com/a/status/1\nhttps://x.com/b/status/2\n"
        );
        assert_eq!(s.total(), 2);
        cleanup(&p);
    }

    #[test]
    fn rejects_duplicates_including_other_handles() {
        let p = temp_path("dupes");
        let mut s = Store::open(p.clone(), false).unwrap();
        assert!(s.add("https://x.com/a/status/1").unwrap());
        assert!(!s.add("https://x.com/a/status/1").unwrap());
        // Same tweet id, different handle in the path.
        assert!(!s.add("https://x.com/someone_else/status/1").unwrap());
        assert_eq!(fs::read_to_string(&p).unwrap().lines().count(), 1);
        cleanup(&p);
    }

    #[test]
    fn dedupes_against_preexisting_file() {
        let p = temp_path("existing");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "https://x.com/a/status/1\n\nhttps://example.com/x\n").unwrap();
        let mut s = Store::open(p.clone(), false).unwrap();
        assert_eq!(s.total(), 2);
        assert!(!s.add("https://x.com/a/status/1").unwrap());
        assert!(!s.add("https://example.com/x").unwrap());
        assert!(s.add("https://example.com/y").unwrap());
        cleanup(&p);
    }

    #[test]
    fn recovers_when_file_is_deleted_or_truncated() {
        let p = temp_path("truncate");
        let mut s = Store::open(p.clone(), false).unwrap();
        s.add("https://x.com/a/status/1").unwrap();
        fs::remove_file(&p).unwrap();
        // The link is gone from disk, so it must be writable again.
        assert!(s.add("https://x.com/a/status/1").unwrap());
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "https://x.com/a/status/1\n"
        );
        assert_eq!(s.total(), 1);
        cleanup(&p);
    }
}
