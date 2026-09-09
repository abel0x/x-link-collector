// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! Minimal stderr logging. journald/systemd adds its own timestamps, but the
//! daemon is just as often run in a terminal, so we print our own UTC stamp.

use std::fmt::Arguments;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub static QUIET: AtomicBool = AtomicBool::new(false);

/// Routine progress; silenced by `--quiet`.
pub fn line(args: Arguments<'_>) {
    if QUIET.load(Ordering::Relaxed) {
        return;
    }
    eprintln!("[{}] {}", timestamp(), args);
}

/// Problems; always printed.
pub fn error(args: Arguments<'_>) {
    eprintln!("[{}] {}", timestamp(), args);
}

#[macro_export]
macro_rules! log {
    ($($t:tt)*) => { $crate::log::line(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! elog {
    ($($t:tt)*) => { $crate::log::error(format_args!($($t)*)) };
}

pub fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let sod = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Howard Hinnant's days-from-civil inverse; no time zone handling, UTC only.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn epoch_and_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
