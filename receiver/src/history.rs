// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! When each link was captured.
//!
//! `links.txt` stays one URL per line -- other tools read it and people edit it
//! by hand -- so capture times go to a log of their own next to the settings
//! file, one `<unix seconds>\t<url>` per line. The panel's activity chart and
//! its "captured" column come from here. Links collected before the log
//! existed simply have no time.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::url;

pub const FILE_NAME: &str = "history.tsv";

pub struct History {
    path: PathBuf,
    /// dedupe key -> when that link was first captured
    first: HashMap<String, u64>,
    /// every capture, in the order logged
    times: Vec<u64>,
}

impl History {
    pub fn open(path: PathBuf) -> History {
        let mut history = History {
            path,
            first: HashMap::new(),
            times: Vec::new(),
        };
        // Unreadable means empty: the log is a nicety, never a reason to stop.
        if let Ok(text) = fs::read_to_string(&history.path) {
            for line in text.lines() {
                let Some((ts, link)) = line.split_once('\t') else {
                    continue;
                };
                if let Ok(ts) = ts.trim().parse::<u64>() {
                    history.note(ts, link.trim());
                }
            }
        }
        history
    }

    fn note(&mut self, ts: u64, link: &str) {
        self.first.entry(url::dedupe_key(link)).or_insert(ts);
        self.times.push(ts);
    }

    /// Log a capture that has already been written to the links file.
    pub fn record(&mut self, link: &str) -> io::Result<()> {
        let ts = now();
        self.note(ts, link);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(format!("{ts}\t{link}\n").as_bytes())
    }

    pub fn captured(&self, link: &str) -> Option<u64> {
        self.first.get(&url::dedupe_key(link)).copied()
    }

    /// Captures per day for the last `days` days, oldest first, ending with
    /// today. `offset` is the viewer's UTC offset in seconds, so the days are
    /// the viewer's days.
    pub fn daily(&self, days: usize, offset: i64) -> Vec<u32> {
        let day_of = |ts: u64| (ts as i64 + offset).div_euclid(86_400);
        let first = day_of(now()) - days as i64 + 1;
        let mut counts = vec![0u32; days];
        for &ts in &self.times {
            let day = day_of(ts);
            if day >= first {
                if let Some(slot) = counts.get_mut((day - first) as usize) {
                    *slot += 1;
                }
            }
        }
        counts
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("x-link-history-{tag}-{n}/history.tsv"))
    }

    #[test]
    fn records_and_reloads() {
        let p = temp_file("reload");
        let mut h = History::open(p.clone());
        assert_eq!(h.captured("https://x.com/a/status/1"), None);
        h.record("https://x.com/a/status/1").unwrap();
        h.record("https://example.com/x").unwrap();

        let again = History::open(p.clone());
        let at = again.captured("https://twitter.com/zz/status/1").unwrap();
        assert!(now() - at < 60);
        assert_eq!(again.daily(7, 0), [0, 0, 0, 0, 0, 0, 2]);
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }

    #[test]
    fn buckets_by_the_viewers_day_and_skips_junk() {
        let p = temp_file("days");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let t = now();
        fs::write(
            &p,
            format!(
                "{}\thttps://x.com/a/status/1\n\
                 garbage line\n\
                 {}\thttps://x.com/b/status/2\n\
                 {}\thttps://x.com/c/status/3\n",
                t - 86_400,
                t - 40 * 86_400,
                t
            ),
        )
        .unwrap();
        let h = History::open(p.clone());
        assert_eq!(h.daily(3, 0), [0, 1, 1]);
        assert_eq!(h.daily(1, 3 * 3600).iter().sum::<u32>(), 1);
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
}
