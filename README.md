<div align="center">

<img src="docs/logo.png" alt="x-link-collector" width="140" height="140">

# x-link-collector

**Middle-click tweets while you read. They file themselves and the tab closes.**

[![ci](https://github.com/abel0x/x-link-collector/actions/workflows/ci.yml/badge.svg)](https://github.com/abel0x/x-link-collector/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![platform](https://img.shields.io/badge/linux%20%7C%20macos%20%7C%20windows-supported-brightgreen.svg)](#platform-support)

</div>

---

Open a tweet in a new tab and three things happen before you look up: the link is
cleaned, appended to a text file on your desktop, and the tab closes itself. Later,
one command downloads the media behind every link it collected.

```
  middle-click            extension              receiver             links.txt
  a tweet        ──────>  cleans the    ──────>  writes it   ──────>  on your
                          URL, closes            once, never          desktop
                          the tab                twice                    │
                                                                          ▼
  x-media/<handle>/<tweet id>-1.mp4    <──────   downloader   <──────  x-download
```

## Quick start

<details open>
<summary><b>Linux / macOS</b></summary>

```sh
git clone https://github.com/abel0x/x-link-collector
cd x-link-collector

make build              # compile the receiver
make run                # start it (Ctrl-C to stop)
make service            # ...or run it at login: systemd on Linux, launchd on macOS

make downloader-setup   # yt-dlp + gallery-dl into downloader/.venv, no sudo
```
</details>

<details>
<summary><b>Windows</b></summary>

```powershell
git clone https://github.com/abel0x/x-link-collector
cd x-link-collector

.\scripts\setup.ps1              # builds the receiver, installs the tools
.\scripts\setup.ps1 -Autostart   # ...and starts it at every logon
```

Needs [Rust](https://rustup.rs) and [Python 3.9+](https://python.org). Or take a
prebuilt `x-link-receiver.exe` from [Releases](https://github.com/abel0x/x-link-collector/releases)
and skip the Rust part.
</details>

Then load the extension: open `vivaldi://extensions` (or `chrome://extensions`),
turn on **Developer mode**, click **Load unpacked**, and pick the `extension/`
folder. Middle-click a tweet — the toolbar badge counts what it saves.

## Downloading the media

```sh
make download            # everything new, best quality X serves
make download-all        # including age-restricted tweets
make download-status     # what has been collected so far
```

Media lands in `<desktop>/x-media/<handle>/<tweet id>-1.mp4`. Every link is
fetched exactly once, and stopping is free — Ctrl-C, `--limit`, `--batch` all
resume where they left off. See **[downloader/README.md](downloader/README.md)**.

## What is in here

| | |
|---|---|
| **[receiver/](receiver/)** | A dependency-free Rust daemon on `127.0.0.1:9876`. Appends links, deduplicates, fsyncs each write. 541 KB static binary, ~2.6 MB RSS, one idle thread. |
| **[extension/](extension/)** | Manifest V3 service worker for Vivaldi, Chrome, Brave and Edge. Cleans the URL, posts it, closes the tab once the write is confirmed. |
| **[downloader/](downloader/)** | Fetches the media behind the links, once each, via yt-dlp and gallery-dl. Resumable, deduplicating, with routes for age-restricted tweets. |
| **[spliter/](spliter/)** | Flattens the per-handle folders into one directory. Reversible. |

## Two rules the extension never breaks

**It only closes a tab it watched being created.** Clicking through your feed is a
same-tab navigation; those tabs are left completely alone. Once a new tab settles
on something that is not a tweet, it is dropped from the watch list for good.

**A tab is closed only after the receiver answers 2xx.** If the receiver is down
the tab simply stays open, nothing is lost, and the worker pauses instead of
retrying every tab.

Both are covered by tests in `extension/test/`.

## URL cleaning

```
https://mobile.twitter.com/jack/status/20/photo/1?s=20&t=TRACKING#top
                        ->  https://x.com/jack/status/20
```

Deduplication keys on the **tweet id**, because x.com serves the same tweet under
any handle in the path — `/alice/status/1` and `/bob/status/1` are one link.

## Where files go

`links.txt` and `x-media/` live on your desktop, resolved per platform:

| | |
|---|---|
| Linux | `XDG_DESKTOP_DIR` from `~/.config/user-dirs.dirs` — correct on localised installs, where the desktop is `~/Masaüstü`, `~/Escritorio`, `~/Bureau` rather than `~/Desktop` |
| macOS | `~/Desktop` |
| Windows | `%USERPROFILE%\Desktop`, or the OneDrive-redirected one when that is where it actually lives |

Override any of it: `x-link-receiver --file ~/notes/tweets.txt`,
`x-download --out ~/Videos/x`.

## Installing a release

Grab the archive for your system from
[Releases](https://github.com/abel0x/x-link-collector/releases). Each one ships
the `x-link-receiver` binary plus a `.sha256` to check it against.

<details>
<summary><b>Linux</b> — one binary, every distribution</summary>

```sh
tar xzf x-link-receiver-linux-x86_64.tar.gz
./x-link-receiver-linux-x86_64/x-link-receiver
```

Statically linked against musl, so there is no glibc version to match and no
separate Debian, Ubuntu, Fedora or Alpine build to pick between. One file, 541 KB.
</details>

<details>
<summary><b>macOS</b> — clear the quarantine flag first</summary>

```sh
tar xzf x-link-receiver-macos-arm64.tar.gz
xattr -d com.apple.quarantine x-link-receiver-macos-arm64/x-link-receiver
./x-link-receiver-macos-arm64/x-link-receiver
```

The binary is not code-signed, so Gatekeeper blocks it until that attribute is
removed. Take `macos-arm64` for Apple Silicon, `macos-x86_64` for Intel.
</details>

<details>
<summary><b>Windows</b> — SmartScreen will warn once</summary>

Unzip and run `x-link-receiver.exe`. It is not signed, so SmartScreen shows
"Windows protected your PC" — **More info → Run anyway**. The daemon binds
loopback only, so no firewall prompt should appear.
</details>

Building from source avoids both warnings, and takes about ten seconds.

## Platform support

| | receiver | extension | downloader | spliter |
|---|---|---|---|---|
| Linux | ✅ | ✅ | ✅ | ✅ |
| macOS | ✅ | ✅ | ✅ | ✅ |
| Windows | ✅ | ✅ | ✅ | ✅ |

CI builds and runs the full test suite on all three on every push, so the badge
above is the real check. The receiver uses `poll(2)` and `signal(2)` on Unix and
a console control handler on Windows; the Python tools are standard library only
and shell out to yt-dlp / gallery-dl, which run everywhere too.

Platform-specific things that are handled rather than assumed: localised desktop
folders, OneDrive-redirected Desktop, virtualenv layout (`Scripts\` vs `bin/`),
Windows reserved device names (`con`, `nul`, `com1`) appearing as an X handle, and
CRLF line endings in a hand-edited `links.txt`.

## Privacy

Everything is local. The receiver binds loopback only and **refuses any request
carrying a web-page `Origin`** — without that rule, any site you visit could
append lines to your file. Nothing is uploaded anywhere.

The one exception is opt-in and announced: `x-download --mirror` sends a tweet id
to `api.fxtwitter.com` / `api.vxtwitter.com` when X itself refuses the link.

> **Your `links.txt` and `x-media/` are personal data.** Both are in `.gitignore`,
> along with videos, images and `cookies.txt`. Check `git status` before your first
> push anyway.

## Tests

```sh
make test        # 22 Rust unit tests + 9 service worker tests
```

## Licence

[Apache 2.0](LICENSE). yt-dlp (Unlicense) and gallery-dl (GPL-2.0) are invoked as
external programs, installed separately, and are neither bundled nor linked — see
[NOTICE](NOTICE).
