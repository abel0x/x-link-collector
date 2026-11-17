# Changelog

## [1.4.0] - 2026-11-17

### Security

- **In Chromium, the receiver takes links from our extension only.** The
  extension now has a fixed id, from a public key in its manifest:
  `jnedknmekfknjaajolahghmpigmmfell`, unpacked and (once published) from the
  Chrome Web Store alike. Any other Chromium extension is refused. Firefox
  gives each install an id at random, so there any extension may still hand
  over a link, and nothing more. `X_LINK_EXTENSION_IDS` adds ids by hand.

  **After updating, remove the old unpacked extension and load `extension/`
  again**: a copy loaded before this has another id, and its links are refused
  (its tabs stay open, so nothing is lost).
- **The panel's API takes changes only from the panel.** Each install has a
  key of its own (`panel-key`, next to the settings, readable by you alone);
  the receiver writes it into the panel page and every change the panel asks
  for carries it, which a page elsewhere cannot read. Browsers also say where
  a request comes from, and the API now answers only the panel's own pages,
  so another site cannot set off even a GET with an image tag. curl and
  scripts, which send neither, work as before.

### Added

- **Build attestations.** Every release archive and extension package comes
  with GitHub's signed record of the repository, commit and workflow that
  built it; `gh attestation verify <file> --repo abel0x/x-link-collector`
  checks it.
- `make store-zip` and [docs/chrome-web-store.md](docs/chrome-web-store.md):
  the package and the listing, in English and Turkish, for publishing the
  extension on the Chrome Web Store. Releases carry that package too.

## [1.3.0] - 2026-11-17

### Added

- **Gallery:** a page of every photo and video in the media folder, newest
  first, as a grid. Show only videos or only photos, or one account (each
  with how many it has); click one to open it in the viewer, which steps
  through that tweet's files.
- **Export:** the Links page's Export menu saves what the list shows, with its
  search and filter, as CSV for a spreadsheet or JSON for a program: line,
  link, account, tweet id, state, when it was collected, its files and why it
  failed. The same is at `/api/export?format=csv|json`.
- **Import from an X archive:** in Add links, pick `data/like.js` or
  `data/bookmark.js` from the archive X sends you and every tweet in it is
  added. The tweets they quote are left out, and an imported link does not
  count towards the day it was read in.
- **A video posted again is kept once.** When a download comes out byte for
  byte the same as a file already in the media folder, it becomes a hard link
  to it: both names stay where they are and the disk holds the file once. The
  run says what it saved. On by default; *Keep one copy of a video posted
  again* in Settings, or `--keep-duplicates`, turns it off.
- `make panel-test` opens every page of the panel in English and Turkish in a
  headless Chrome and fails on an untranslated key, a page in the wrong
  language or an empty heading; CI runs it. `make screenshots` takes the
  README's pictures again from a made-up collection.

### Changed

- The panel's words live in `receiver/panel/i18n/en.json` and `tr.json`:
  English is the source, Turkish follows it key for key. A test in CI fails
  when a key, a `{placeholder}` or a plural form is missing from either, for
  the panel and the extension alike.
- **One macOS download for Intel and Apple silicon.** The release builds both
  on one Mac and joins them (`x-link-collector-macos-universal.tar.gz`), in
  place of the separate `macos-arm64` and `macos-x86_64` archives. 1.2.1 has
  no Intel build: GitHub had no macOS runner free for it.

## [1.2.1] - 2026-11-17

### Security

- **Other browser extensions can no longer use the panel's API.** Any
  installed extension could call x-link-receiver with its own origin, and
  without a single permission of its own it could change the settings, point
  the Python setting at a program of its choosing and start a "download" to
  run it. Extensions may now only hand over a link (`POST /`) and ask whether
  the receiver is up (`GET /health`), which is all ours does; everything under
  `/api/` answers the panel and local tools only.
- **Only tweets on x.com and twitter.com are downloaded.** Any other link in
  links.txt reached yt-dlp's generic extractor, so a run could be sent to any
  address, the local network included, and with a login chosen the browser's
  cookies for that site went along. Such links stay in the file but are now
  marked failed for good, as "Not an X link".
- The mirror route only fetches media over https. A mirror's answer could name
  a `file://` address, which would have copied a file from this disk into the
  media folder.

### Fixed

- Why a link failed is shown in Turkish too, for every reason x-download
  gives itself; a tool's own output is still shown as it came.
- "1 videos and 1 images" on the Folder page.
- The Turkish for starting at Windows sign-in said "when Windows starts".

### Changed

- **A new look, shared with abel0x/toolkit.** The panel and the extension's
  page drop the indigo, cyan and violet for near-black surfaces (or white in
  the light theme), one light accent and flat bordered cards. Colour now only
  means something: green for "running", red for what failed. The strip tells
  its states apart by lightness, so it still reads without colour vision.
- The logo is redrawn flat light grey on a dark tile, for the extension
  icons, the panel and the README.

## [1.2.0] - 2026-11-06

### Fixed

- **Moving the media folder no longer makes the downloader forget.** The
  record of what was downloaded used to live inside the media folder, so
  moving `x-media` somewhere else (to cloud storage, say) took it along, and
  the next run downloaded every link again. It now lives next to the settings
  file as `download-history.json`. The old `.x-download-state.json` is still
  read and taken over by the first run; nothing has to be done by hand.
- A tweet is found in the record by its id, so the same tweet under another
  handle is not downloaded twice.

### Added

- **A setting for it:** *Remember downloads after their files are moved*, on
  by default. Turned off, a link whose files are no longer in the media
  folder is downloaded again by the next run.
- **Download again**, a section of its own on the Download page. It counts the
  downloaded links whose files have left the media folder, downloads them all
  again with one button, or downloads any links pasted into it. The Links page
  gains a *Moved away* filter, and a *Download again* button on those rows.
- `x-download --again URL`, `--again-missing`, `--look-in DIR` and
  `--history FILE`. `--status` shows how many links have moved away.

## [1.1.1] - 2026-10-10

### Added

- **A signed Firefox add-on.** The release now carries
  `x-link-collector-firefox.xpi`, signed by Mozilla, so Firefox 128 and newer
  keep the extension installed across restarts. 1.1.0 shipped without it.

## [1.1.0] - 2026-10-08

### Added

- **The panel.** The receiver now serves a page at <http://127.0.0.1:9876> that
  runs the whole kit from a browser tab:
  - an overview with one square per collected link, coloured by what the
    downloader made of it, and how many links came in per day;
  - the links, each with a thumbnail of what was downloaded; a click shows the
    photo or plays the video. Search, filter by state, see why a download
    failed, fetch or retry one link, paste links in, take one out;
  - downloads started, followed and stopped from the page, with the choices the
    command line has: mirrors, your own login, parallel downloads, a limit,
    retrying failed links, a dry run;
  - the one-folder flatten, and its undo;
  - every setting, saved to one file that x-download and x-flatten read too;
  - installing or updating yt-dlp and gallery-dl with one button.

  It is in English and Turkish, light and dark, and fits a phone-width window.
  It is compiled into the binary and loads nothing from anywhere else. It can
  notify you when a download finishes, and says when yt-dlp is old enough
  that X has probably broken it.
- **Firefox.** The same extension folder runs in Firefox 128 and newer. The
  release workflow signs it as an unlisted add-on when the AMO API key is set
  as a repository secret.
- The extension's right-click menu collects a tweet link, or the tweet on the
  page, without opening or closing a tab.
- Automatic downloads, if you want them: new links are fetched 20 seconds
  after the last one arrives.
- `history.tsv` next to the settings file records when each link was
  captured. `links.txt` stays one URL per line.
- `x-download --setup` installs the tools, `--check` shows what a download
  would use, `--only URL` fetches just that link, and `--no-config` (also on
  x-flatten) ignores the settings file.
  A dry run ends with how many links it would fetch.
- Clicking the extension's toolbar icon opens the panel. When the receiver does
  not answer, it opens the extension's new options page instead, which says how
  to start the receiver and on which port to look for it.
- On Windows, double-clicking `x-link-receiver.exe` opens the panel and then
  runs without a console window, logging to a file next to the settings.
  Double-clicking it again opens the panel of the one already running. The
  panel can start it at every sign-in, and quit it. `--open` opens the panel
  on any system, and `--no-open` never does.
- A smoke test that starts the built receiver and uses it end to end, run by
  CI on Linux, macOS and Windows.
- Release archives carry the whole kit: the receiver, the downloader, the
  flattener and the extension.
- Tests for the downloader and the flattener.

### Changed

- The extension leaves alone a tweet opened from the panel.
- The panel and `/health` answer only to `127.0.0.1` and `localhost`, so a web
  page cannot read them by pointing its own domain at your machine.
- A pasted list of links is written with one fsync instead of one per line,
  and may be up to 1 MiB.
- `make downloader-setup` and `make downloader-update` go through
  `x-download --setup`, the same code the panel's button runs.
- `setup.ps1 -Autostart` uses the same sign-in entry as the panel instead of a
  scheduled task, and removes a task an older setup left.
- The systemd unit lets the downloads the panel starts reach X over IPv6 and
  write under your home directory; the rest of the system stays read-only.
- The launchd agent finds Python and ffmpeg where Homebrew puts them.
- `make service` restarts a receiver that is already running, so an upgrade
  takes effect.
- The files are arranged by what they are: `receiver/`, `extension/`,
  `tools/` (x-download and x-flatten, which were in `downloader/` and
  `spliter/`), `packaging/` (systemd, launchd, Windows setup), `tests/` and
  `docs/`. The tools' virtualenv moves to `tools/.venv`: run
  `make downloader-setup` once.
- The extension needs Chrome 121 or newer, for a manifest Firefox can read too.
- `Cargo.toml` states the licence the project has always had, Apache-2.0.

## [1.0.0] - 2026-09-09

First release: the receiver, the extension, the downloader and the flattener.
