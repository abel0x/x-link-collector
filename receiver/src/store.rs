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
//! * Removing a link from the panel is the one rewrite: the file is written
//!   whole to a sibling and renamed over, so a reader never sees half of it.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
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

    pub fn set_fsync(&mut self, fsync: bool) {
        self.fsync = fsync;
    }

    /// `add` for a batch -- a pasted list can run to thousands of lines -- with
    /// one fsync for the lot instead of one per line. Reports each link the way
    /// `add` would.
    pub fn add_all(&mut self, urls: &[String]) -> io::Result<Vec<bool>> {
        let fsync = std::mem::replace(&mut self.fsync, false);
        let added: io::Result<Vec<bool>> = urls.iter().map(|u| self.add(u)).collect();
        self.fsync = fsync;
        let added = added?;
        if fsync && added.contains(&true) {
            // Opened for writing: Windows refuses to flush a read-only handle.
            OpenOptions::new()
                .append(true)
                .open(&self.path)?
                .sync_data()?;
        }
        Ok(added)
    }

    /// Every link in file order, read from disk so hand edits show up.
    pub fn links(&mut self) -> io::Result<Vec<String>> {
        self.refresh_if_changed();
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Drop every line that is the same link as `url` -- for a tweet, any line
    /// with its id. Comments and all other lines are kept byte for byte.
    /// Returns `true` when something was removed.
    pub fn remove(&mut self, url: &str) -> io::Result<bool> {
        let key = url::dedupe_key(url.trim());
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        let mut kept = String::with_capacity(text.len());
        let mut removed = false;
        for line in text.split_inclusive('\n') {
            let l = line.trim();
            if !l.is_empty() && !l.starts_with('#') && url::dedupe_key(l) == key {
                removed = true;
                continue;
            }
            kept.push_str(line);
        }
        if removed {
            write_atomic(&self.path, kept.as_bytes(), self.fsync)?;
            self.reload()?;
        }
        Ok(removed)
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

/// Replace `path` in one step: write a hidden sibling, then rename it over.
/// Whoever reads the file meanwhile -- x-download, an editor -- gets the old
/// version or the new one, never a mix.
pub fn write_atomic(path: &Path, data: &[u8], sync: bool) -> io::Result<()> {
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy());
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    let result = write_file(&tmp, data, sync).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_file(path: &Path, data: &[u8], sync: bool) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(data)?;
    if sync {
        f.sync_all()?;
    }
    Ok(())
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

    #[test]
    fn lists_and_removes_links() {
        let p = temp_path("remove");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(
            &p,
            "# my list\nhttps://x.com/a/status/1\nhttps://example.com/x\r\nhttps://x.com/b/status/2\n",
        )
        .unwrap();
        let mut s = Store::open(p.clone(), false).unwrap();
        assert_eq!(
            s.links().unwrap(),
            [
                "https://x.com/a/status/1",
                "https://example.com/x",
                "https://x.com/b/status/2"
            ]
        );

        // Same tweet id under another handle is the same link.
        assert!(s.remove("https://twitter.com/zz/status/1?s=20").unwrap());
        assert!(!s.remove("https://x.com/a/status/1").unwrap());
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "# my list\nhttps://example.com/x\r\nhttps://x.com/b/status/2\n"
        );
        assert_eq!(s.total(), 2);
        // Gone from the dedupe set too, so it can be collected again.
        assert!(s.add("https://x.com/a/status/1").unwrap());
        cleanup(&p);
    }

    #[test]
    fn adds_a_batch() {
        let p = temp_path("batch");
        let mut s = Store::open(p.clone(), true).unwrap();
        s.add("https://x.com/a/status/1").unwrap();
        let batch: Vec<String> = [
            "https://x.com/b/status/2",
            "https://x.com/a/status/1",
            "https://example.com/x",
            "https://x.com/other/status/2",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(s.add_all(&batch).unwrap(), [true, false, true, false]);
        assert_eq!(s.total(), 3);
        assert_eq!(fs::read_to_string(&p).unwrap().lines().count(), 3);
        cleanup(&p);
    }

    #[test]
    fn atomic_write_leaves_no_temp_file() {
        let p = temp_path("atomic");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        write_atomic(&p, b"one\n", false).unwrap();
        write_atomic(&p, b"two\n", true).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "two\n");
        let names: Vec<_> = fs::read_dir(p.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["links.txt"]);
        cleanup(&p);
    }
}
