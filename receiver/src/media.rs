// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The downloaded media, for the panel's previews: `GET /media/<handle>/<file>`.
//!
//! Only files inside the media folder, and only photos and videos -- not the
//! state file, not anything a `..` could reach. Ranges are honoured, so a
//! video can be scrubbed through, and the file is streamed rather than read
//! into memory. Other sites cannot embed these: a request a page elsewhere
//! starts is refused, and the response says it is for this origin only.

use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::{Component, Path};

use crate::http::{self, Request, Response};

pub fn serve(req: &Request, root: &Path) -> Response {
    // A cross-site <img> or <video> sends no Origin, but browsers do say where
    // a request came from. Only the panel's own pages, and typed addresses.
    if let Some(site) = req.header("sec-fetch-site") {
        if site != "same-origin" && site != "none" {
            return Response::text(403, "the media is only shown in the panel\n");
        }
    }
    let rel = http::decode(req.path.strip_prefix("/media/").unwrap_or(""));
    let Some((file, content_type)) = open(root, &rel) else {
        return Response::text(404, "not found\n");
    };
    let size = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return Response::text(404, "not found\n"),
    };

    let (status, start, len) = match req.header("range").map(|r| range(r, size)) {
        None => (200, 0, size),
        Some(Some((start, end))) => (206, start, end - start + 1),
        Some(None) => {
            let mut res = Response::text(416, "range not satisfiable\n");
            res.extra.push(format!("Content-Range: bytes */{size}"));
            return res;
        }
    };
    let mut file = file;
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Response::text(500, "cannot read the file\n");
    }
    let mut res = Response::stream(status, content_type, file, len);
    res.extra = vec![
        "Accept-Ranges: bytes".to_string(),
        "Cross-Origin-Resource-Policy: same-origin".to_string(),
    ];
    if status == 206 {
        res.extra.push(format!(
            "Content-Range: bytes {start}-{}/{size}",
            start + len - 1
        ));
    }
    res
}

/// The file at `rel` under `root`, if it is a photo or a video that really
/// lives there -- after symlinks, not just by its spelling.
fn open(root: &Path, rel: &str) -> Option<(File, &'static str)> {
    let rel = Path::new(rel);
    let plain = rel.components().all(|c| match c {
        Component::Normal(part) => !part.to_string_lossy().starts_with('.'),
        _ => false,
    });
    if !plain || rel.as_os_str().is_empty() {
        return None;
    }
    let content_type = media_type(rel)?;
    let root = root.canonicalize().ok()?;
    let path = root.join(rel).canonicalize().ok()?;
    if !path.starts_with(&root) || !path.is_file() {
        return None;
    }
    Some((File::open(path).ok()?, content_type))
}

fn media_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    Some(match ext.as_str() {
        "mp4" => "video/mp4",
        "m4v" => "video/x-m4v",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => return None,
    })
}

/// One `bytes=` range, as inclusive offsets; `None` when it cannot be met.
/// Several ranges at once are answered with the first: no browser asks for
/// more to play a video.
fn range(header: &str, size: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    let spec = spec.split(',').next()?.trim();
    let (from, to) = spec.split_once('-')?;
    let (start, end) = match (from.trim(), to.trim()) {
        ("", "") => return None,
        // bytes=-500: the last 500 bytes
        ("", n) => {
            let n: u64 = n.parse().ok()?;
            (size.checked_sub(n.min(size))?, size.checked_sub(1)?)
        }
        (a, "") => (a.parse().ok()?, size.checked_sub(1)?),
        (a, b) => (
            a.parse().ok()?,
            b.parse::<u64>().ok()?.min(size.checked_sub(1)?),
        ),
    };
    (start <= end && end < size).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reads_byte_ranges() {
        assert_eq!(range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(range("bytes=8-100", 10), Some((8, 9)));
        assert_eq!(range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(range("bytes=-30", 10), Some((0, 9)));
        assert_eq!(range("bytes=0-1, 5-6", 10), Some((0, 1)));
        for bad in [
            "bytes=10-",
            "bytes=5-2",
            "bytes=-",
            "items=0-1",
            "bytes=x-1",
        ] {
            assert_eq!(range(bad, 10), None, "{bad}");
        }
        assert_eq!(range("bytes=0-", 0), None);
    }

    #[test]
    fn serves_only_media_inside_the_folder() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("x-link-media-{n}/x-media"));
        fs::create_dir_all(root.join("alice")).unwrap();
        fs::write(root.join("alice/1-1.mp4"), b"video").unwrap();
        fs::write(root.join("alice/notes.txt"), b"text").unwrap();
        fs::write(root.join(".x-download-state.json"), b"{}").unwrap();
        fs::write(root.parent().unwrap().join("secret.jpg"), b"outside").unwrap();

        assert_eq!(open(&root, "alice/1-1.mp4").unwrap().1, "video/mp4");
        for refused in [
            "alice/notes.txt",
            ".x-download-state.json",
            "../secret.jpg",
            "alice/../../secret.jpg",
            "/etc/passwd",
            "",
            "alice/missing.mp4",
        ] {
            assert!(open(&root, refused).is_none(), "{refused}");
        }
        #[cfg(unix)]
        {
            // A link that points out of the folder is followed, then refused.
            std::os::unix::fs::symlink(
                root.parent().unwrap().join("secret.jpg"),
                root.join("alice/2-1.jpg"),
            )
            .unwrap();
            assert!(open(&root, "alice/2-1.jpg").is_none());
        }
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
