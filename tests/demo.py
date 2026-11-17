# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
A made-up collection for the panel to show, and a receiver serving it.

Used by panel_dom.py (the panel in a real browser) and screenshots.py (the
README's pictures). Every account and tweet id is invented.
"""

from __future__ import annotations

import json
import os
import random
import shutil
import socket
import subprocess
import sys
import time
import urllib.request
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ACCOUNTS = ["salt_flats", "mossy_rocks", "lowlight_club", "zine_corner",
            "aurora_lens", "paper_kite", "tidal_notes", "quiet_harbor"]
DAY = 86400


def make(work: Path, now: int | None = None) -> Path:
    """
    120 links over three weeks: 70 downloaded, 5 without media, 4 failed and
    41 waiting, with the last fortnight's capture times. Returns the settings
    file to start the receiver with.
    """
    rng = random.Random(7)
    now = now or int(time.time())
    per_day = [0, 2, 6, 7, 6, 7, 6, 7, 6, 7, 6, 7, 6, 7]
    older = 120 - sum(per_day)
    times = [now - 20 * DAY - i * 3600 for i in range(older)]
    for day, count in enumerate(per_day):
        times += [now - (13 - day) * DAY - 3 * 3600 - k * 1800 for k in range(count)]
    times.sort()

    config_dir = work / "config"
    config_dir.mkdir(parents=True, exist_ok=True)
    links, history, entries = [], [], {}
    for i, captured in enumerate(times):
        url = (f"https://x.com/{rng.choice(ACCOUNTS)}/status/"
               f"{2096517968860224831 + i * 4710000000 + rng.randint(0, 999999)}")
        links.append(url)
        if i >= older:
            history.append(f"{captured}\t{url}")
        if i < 70:
            entries[url] = {"status": "done", "files": [], "attempts": 1}
        elif i < 75:
            entries[url] = {"status": "no-media", "attempts": 1}
        elif i < 79:
            entries[url] = {"status": "failed", "attempts": 1,
                            "error": "rate limited by X -- wait a bit, or lower --jobs"}

    (work / "links.txt").write_text("\n".join(links) + "\n", encoding="utf-8")
    (config_dir / "history.tsv").write_text("\n".join(history) + "\n", encoding="utf-8")
    (config_dir / "download-history.json").write_text(json.dumps({"entries": entries}))
    # As if the extension had already been in touch: no setup box.
    (config_dir / "extension-seen").write_text(str(now))
    config = config_dir / "config.json"
    config.write_text(json.dumps({
        "links_file": str(work / "links.txt"),
        "media_dir": str(work / "x-media"),
        "python": sys.executable,
        "tools_dir": str(ROOT),
    }))
    return config


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


@contextmanager
def serve(binary: Path, config: Path, port: int):
    """The receiver on 127.0.0.1:port with these settings, until the block ends."""
    # The same port in the settings, or the panel asks for a restart to switch.
    settings = json.loads(config.read_text())
    settings["port"] = port
    config.write_text(json.dumps(settings))
    env = dict(os.environ, X_LINK_COLLECTOR_CONFIG=str(config))
    log = open(config.parent / "receiver.log", "wb")
    process = subprocess.Popen([str(binary), "--no-open", "--port", str(port)], env=env,
                               stdout=log, stderr=subprocess.STDOUT)
    try:
        deadline = time.monotonic() + 20
        while True:
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2).close()
                break
            except OSError:
                if time.monotonic() > deadline or process.poll() is not None:
                    raise AssertionError("the receiver did not come up")
                time.sleep(0.2)
        yield f"http://127.0.0.1:{port}"
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
        log.close()


def chrome() -> str:
    """A Chrome or Chromium to run headless: $CHROME, or the first one found."""
    explicit = os.environ.get("CHROME")
    if explicit:
        return explicit
    for name in ("google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"):
        found = shutil.which(name)
        if found:
            return found
    raise SystemExit("no Chrome or Chromium found; set CHROME to one")


def headless(url: str, lang: str, *extra: str) -> subprocess.CompletedProcess:
    """One page in a fresh headless browser, given time to run its script."""
    return subprocess.run(
        [chrome(), "--headless=new", "--disable-gpu", "--no-first-run", "--hide-scrollbars",
         "--no-sandbox", f"--lang={lang}", f"--accept-lang={lang}",
         "--virtual-time-budget=4000", *extra, url],
        capture_output=True, text=True, timeout=120, check=False,
    )
