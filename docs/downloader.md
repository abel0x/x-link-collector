# x-download

Downloads the media behind every link the extension collected, at the best
quality X serves, and never fetches the same tweet twice.

The panel's **Download** page runs it for you, with every choice below as a
button or a field. From the repository root:

```sh
make downloader-setup     # once: yt-dlp + gallery-dl into tools/.venv, no sudo
make download             # download everything new
make download-all         # including age-restricted tweets
make download-status      # what has been collected
```

Or call it directly, from anywhere, for the options below:

```sh
tools/x-download --watch
tools/x-download --mirror-only --limit 100
```

Media lands in `<desktop>/x-media/<handle>/<tweet id>-<n>.<ext>`, next to
`links.txt`.

## Why it shells out

The extraction itself is delegated to **yt-dlp** (video) and **gallery-dl**
(images). X's GraphQL endpoints and guest-token flow change every few weeks;
those two projects track it, and an extractor written here would rot within a
month. What `x-download` owns is everything around them: which links still need
doing, retry policy, naming, parallelism, and idempotency.

Because they rot, **update them when downloads start failing** — the panel's
Settings page has a button for it, or:

```sh
make downloader-update    # same as: x-download --setup
```

## About "highest quality"

X caps what it serves, and the tools take the top of that ladder. Measured on a
real tweet, the progressive `http-2176` variant and the HLS ladder resolve to
byte-identical media (728x720, 589 kbps h264, 128 kbps AAC) — the `2176k` label
is yt-dlp's estimate, not the real bitrate. So the default selection is already
the best available; the explicit `-S res,fps,vbr,abr,br` here just makes it
deterministic.

Nothing is ever re-encoded. `--merge-output-format mp4` stream-copies when video
and audio arrive separately; images are taken at `:orig`, which is the file as
uploaded.

## When X refuses: the escalation chain

A tweet is attempted along four routes, cheapest and most private first. The
first one that produces media wins.

| # | route | login | notes |
|---|---|---|---|
| 1 | yt-dlp, GraphQL | guest token, or yours with `--cookies` | the normal path |
| 2 | yt-dlp, syndication | none | X's own embed endpoint — automatic |
| 3 | gallery-dl | guest token, or `--cookies` | images |
| 4 | mirror API | none | third party, only with `--mirror` |

A link that yt-dlp reports as *media-less* stops there: it saw the tweet, and
the tweet holds no video. Only an outright failure escalates.

**Route 2 (syndication)** is X's own endpoint, the one that powers tweets
embedded on other websites. It needs no login, and on a normal tweet it returns
byte-identical media to route 1 — so the fallback costs no quality and runs
automatically. It does **not** get past the age gate: for a restricted tweet it
answers `{"__typename":"TweetTombstone"}`, measured.

**Route 4 (`--mirror`) is what gets age-restricted tweets without a login.**
Measured on a real age-gated tweet: routes 1–3 all refused, the mirror returned
the full 39 MB / 720x1280 h264 file. The mirror services run their own
authenticated X sessions, which is why they see what a guest token cannot.

It asks `api.fxtwitter.com`, then `api.vxtwitter.com`, for the media URLs and
fetches them directly. These are documented JSON APIs, not the ssstwitter-style
download pages — no HTML scraping, no captcha. It is opt-in because it sends the
tweet id to someone else's server, and it is only reached once X has actually
refused, so a plain text-only tweet never leaks its id to a mirror:

```sh
./x-download --mirror --retry-failed
```

### A collection that is all age-restricted

`--mirror` still pays X's three refusals first: measured at 3.3s and three
rejected requests per link, which over a few hundred links is both slow and a
good way to get rate-limited. When you already know the links are gated, skip
straight to route 4:

```sh
./x-download --mirror-only          # or: make download-all
```

Long runs are built for it: progress carries an ETA, the state file makes the
run resumable after Ctrl-C, mirror requests retry with backoff on rate limits,
and the run stops rather than filling the disk if free space drops under 1 GB.

## Login

Your own session is still the most capable route, and the only one that covers
protected accounts you follow:

```sh
./x-download --cookies vivaldi --retry-failed
```

This reads your logged-in X session from Vivaldi's cookie store and sends it to
X, exactly as the browser would. A `cookies.txt` path works too. The run summary
tells you when links failed for a reason a login would fix, and suggests both
this and `--mirror`.

## Splitting a large list

Nothing has to be done in one sitting. Every finished link is recorded before
the next starts, so stopping and resuming is free — Ctrl-C included.

```sh
./x-download --mirror-only --limit 299    # do 299, stop, remember where
./x-download --mirror-only                # carry on with the rest
./x-download --mirror-only --batch 100 --pause 30
```

`--limit` caps a single run; the run ends with `stopped after 299; 315 link
still pending`, and `--status` shows `progress: 299 / 614 (48%)`. `--batch`
splits the same work into groups and reports after each, optionally pausing
between them, which is the gentler shape for a few hundred links.

Keep `--batch` at or above `--jobs`, or the batch size becomes the parallelism
ceiling.

## Deduplication

Links are deduplicated on the **tweet id**, not the URL text, because x.com
serves the same tweet under any handle in the path — `/alice/status/1` and
`/bob/status/1` are one link and download once. Query strings and `mobile.`
hosts make no difference either. Collapsed lines are reported at startup.

The receiver already applies this rule when collecting, so a `links.txt` it
produced is clean; this matters for hand-edited or imported lists.

## State and retries

`<media dir>/.x-download-state.json` records one entry per link:

| status | meaning | retried? |
|---|---|---|
| `done` | media on disk, paths recorded | never |
| `no-media` | text-only tweet, nothing to fetch | only with `--retry-failed` |
| `failed` | error, with a readable reason | up to 3 attempts, then `--retry-failed` |

Delete the state file to rebuild from scratch; already-downloaded files are
skipped by yt-dlp anyway.

`no-media` is terminal, so it must never be reached by guesswork. X reports an
age-gated tweet with the same "no video could be found" line as a genuinely
text-only one; only gallery-dl's `'Unavailable'` separates them. A hard failure
from any tool therefore outranks a media-less verdict, which keeps restricted
links in the retry queue instead of retiring them.

## Settings from the panel

What you choose in the panel is saved to one file (see the main README for
where), and `x-download` takes its defaults from it: the links file, the media
folder, parallel downloads, mirrors, login, timeout and `--metadata`. An option
given on the command line still wins, and `--no-config` ignores the file
altogether. Without the file, everything is as described here.

## Options

```
--links PATH        links file           (default: <desktop>/links.txt)
--out DIR           media directory      (default: <desktop>/x-media)
-j, --jobs N        parallel downloads   (default: 3 — X rate-limits above ~5)
--cookies B|FILE    browser name or cookies.txt
--mirror            fall back to open mirror APIs when X refuses
--mirror-only       skip X's routes entirely and use the mirrors directly
--watch [SECONDS]   keep running, re-checking the links file (default: 30s)
--retry-failed      re-queue failed and media-less links
--metadata          also write the tweet's metadata JSON
--limit N           stop after N links this run; the rest stay queued
--batch N           work the queue in groups of N, reporting after each
--pause SECONDS     wait between batches (with --batch)
--timeout SECONDS   per-link timeout (default: 900)
--status            print a summary and exit
--dry-run           list what would be downloaded
--only URL          just this link from the links file; repeat for more
--setup             install or upgrade yt-dlp + gallery-dl in tools/.venv
--check             show which tools a download would use, with versions
--no-config         ignore the panel's settings file
```

## Failure reasons you may see

| message | what to do |
|---|---|
| `tweet not found (deleted, private account, or bad id)` | nothing; the tweet is gone |
| `restricted tweet -- retry with --mirror, or --cookies vivaldi` | age-gated; `--mirror` is the no-login route |
| `login required -- retry with --cookies vivaldi` | run with `--cookies`, or try `--mirror` |
| `rate limited by X` | wait, or lower `--jobs` |
| `network error` | retried automatically on the next run |

## Alternative installs

The venv keeps everything inside this folder. If you would rather have the tools
system-wide, `pipx install yt-dlp gallery-dl` (or your package manager) also
works — `x-download` prefers `tools/.venv` and falls back to `$PATH`.
