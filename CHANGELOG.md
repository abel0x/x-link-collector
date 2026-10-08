# Changelog

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
