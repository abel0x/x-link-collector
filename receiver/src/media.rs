// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The downloaded media, for the panel's previews: `GET /media/<handle>/<file>`.
//!
//! Only files inside the media folder, and only photos and videos -- not the
//! state file, not anything a `..` could reach. Ranges are honoured, so a
//! video can be scrubbed through, and the file is streamed rather than read
//! into memory. Other sites cannot embed these: a request a page elsewhere
//! starts is refused, and the response says it is for this origin only.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::app::media_tweet_id;
use crate::http::{self, Request, Response};
use crate::json::{self, Json};

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

pub(crate) fn media_type(path: &Path) -> Option<&'static str> {
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

/// One photo or video in the media folder, for the gallery.
struct Item {
    /// Relative to the media folder, with `/`: what `/media/` serves.
    path: String,
    handle: String,
    id: Option<String>,
    video: bool,
    modified: u64,
}

/// Every photo and video under `root`, newest first, a page at a time, and
/// how many each account has. `handle` and `kind` ("video", "image") narrow
/// the page; the account counts always cover the whole folder.
pub fn gallery(root: &Path, handle: &str, kind: &str, offset: usize, limit: usize) -> Json {
    let mut items = Vec::new();
    collect(root, &mut Vec::new(), &mut items);
    newest_first(&mut items);

    let mut handles: BTreeMap<String, usize> = BTreeMap::new();
    for item in &items {
        *handles.entry(item.handle.clone()).or_default() += 1;
    }
    let wanted: Vec<&Item> = items
        .iter()
        .filter(|i| handle.is_empty() || i.handle == handle)
        .filter(|i| match kind {
            "video" => i.video,
            "image" => !i.video,
            _ => true,
        })
        .collect();
    let page: Vec<Json> = wanted
        .iter()
        .skip(offset)
        .take(limit)
        .map(|i| {
            json::obj([
                ("path", i.path.as_str().into()),
                ("handle", i.handle.as_str().into()),
                ("id", i.id.clone().into()),
                ("kind", if i.video { "video" } else { "image" }.into()),
            ])
        })
        .collect();
    let mut by_count: Vec<(String, usize)> = handles.into_iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    json::obj([
        ("total", items.len().into()),
        ("matched", wanted.len().into()),
        (
            "handles",
            Json::Arr(
                by_count
                    .into_iter()
                    .map(|(h, n)| json::obj([("handle", h.into()), ("count", n.into())]))
                    .collect(),
            ),
        ),
        ("items", Json::Arr(page)),
    ])
}

fn newest_first(items: &mut [Item]) {
    items.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
}

/// Walk the folder the way the media server would allow: no hidden names,
/// no half-finished downloads, a few levels deep, symlinks not followed.
fn collect(dir: &Path, rel: &mut Vec<String>, out: &mut Vec<Item>) {
    if rel.len() > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.ends_with(".part") {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            rel.push(name);
            collect(&entry.path(), rel, out);
            rel.pop();
        } else if kind.is_file() {
            let Some(content_type) = media_type(&PathBuf::from(&name)) else {
                continue;
            };
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            let mut parts = rel.clone();
            parts.push(name.clone());
            out.push(Item {
                path: parts.join("/"),
                // x-download files a tweet's media under the account's folder.
                handle: rel.first().cloned().unwrap_or_default(),
                id: media_tweet_id(&name).map(str::to_string),
                video: content_type.starts_with("video/"),
                modified,
            });
        }
    }
}

#[cfg(test)]
mod gallery_tests {
    use super::*;
    use std::time::SystemTime;

    fn temp_dir() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("x-link-gallery-{n}"))
    }

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(&path).unwrap();
    }

    fn paths(page: &Json) -> Vec<String> {
        match page.get("items") {
            Some(Json::Arr(items)) => items
                .iter()
                .filter_map(|i| i.get("path").and_then(Json::as_str))
                .map(str::to_string)
                .collect(),
            _ => vec![],
        }
    }

    #[test]
    fn photos_and_videos_by_account_and_kind() {
        let root = temp_dir();
        touch(&root, "alice/100-1.mp4");
        touch(&root, "alice/100-2.jpg");
        touch(&root, "bob/200-1.jpg");
        touch(&root, "bob/.x-state.json");
        touch(&root, "bob/300-1.mp4.part");
        touch(&root, "notes.txt");

        let all = gallery(&root, "", "", 0, 10);
        let mut found = paths(&all);
        found.sort();
        assert_eq!(
            found,
            ["alice/100-1.mp4", "alice/100-2.jpg", "bob/200-1.jpg"]
        );
        assert_eq!(all.get("total"), Some(&Json::from(3usize)));
        assert_eq!(
            all.get("handles").cloned(),
            Some(Json::Arr(vec![
                json::obj([("handle", "alice".into()), ("count", 2usize.into())]),
                json::obj([("handle", "bob".into()), ("count", 1usize.into())]),
            ]))
        );

        let videos = gallery(&root, "alice", "video", 0, 10);
        assert_eq!(paths(&videos), ["alice/100-1.mp4"]);
        let first = match videos.get("items") {
            Some(Json::Arr(items)) => items[0].clone(),
            _ => Json::Null,
        };
        assert_eq!(first.get("id").and_then(Json::as_str), Some("100"));
        assert_eq!(first.get("kind").and_then(Json::as_str), Some("video"));
        assert_eq!(paths(&gallery(&root, "", "image", 1, 1)).len(), 1);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn newest_first_then_by_path() {
        let item = |path: &str, modified: u64| Item {
            path: path.to_string(),
            handle: String::new(),
            id: None,
            video: false,
            modified,
        };
        let mut items = vec![item("b", 1), item("c", 5), item("a", 5)];
        newest_first(&mut items);
        let order: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(order, ["a", "c", "b"]);
    }
}
