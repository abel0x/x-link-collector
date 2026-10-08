# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
Start a built receiver and use it the way the extension and the panel do.

    python3 tests/smoke.py receiver/target/release/x-link-receiver

CI runs this on Linux, macOS and Windows, so the parts that differ per system
-- processes, paths, the settings folder, stopping a job -- run for real on
each. Everything happens in a throwaway folder on a free port, and nothing
reaches the network.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TWEET = "https://twitter.com/smoke_test/status/1234567890123456789?s=20"
CLEAN = "https://x.com/smoke_test/status/1234567890123456789"


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Receiver:
    def __init__(self, binary: Path, work: Path):
        self.port = free_port()
        self.work = work
        self.links = work / "links.txt"
        self.media = work / "x-media"
        self.config = work / "config" / "config.json"
        self.config.parent.mkdir(parents=True)
        self.config.write_text(json.dumps({
            "port": self.port,
            "links_file": str(self.links),
            "media_dir": str(self.media),
            "python": sys.executable,
            "tools_dir": str(ROOT),
        }))
        self.log = open(work / "receiver.log", "wb")
        env = dict(os.environ, X_LINK_COLLECTOR_CONFIG=str(self.config))
        self.process = subprocess.Popen([str(binary), "--no-open"], env=env,
                                        stdout=self.log, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            try:
                self.get("/health")
                return
            except OSError:
                time.sleep(0.2)
        raise AssertionError("the receiver did not come up")

    def request(self, path: str, data: bytes | None = None, headers: dict | None = None,
                method: str | None = None) -> tuple[int, dict, bytes]:
        req = urllib.request.Request(f"http://127.0.0.1:{self.port}{path}", data=data,
                                     headers=headers or {}, method=method)
        try:
            with urllib.request.urlopen(req, timeout=30) as res:
                return res.status, dict(res.headers), res.read()
        except urllib.error.HTTPError as e:
            return e.code, dict(e.headers), e.read()

    def get(self, path: str, headers: dict | None = None):
        return self.request(path, headers=headers)

    def api(self, path: str, body: dict | None = None):
        if body is None:
            status, _, raw = self.get(path)
        else:
            status, _, raw = self.request(path, json.dumps(body).encode(),
                                          {"Content-Type": "application/json"}, "POST")
        return status, json.loads(raw or b"{}")


def check(condition: bool, what: str) -> None:
    print(f"  {'ok ' if condition else 'FAIL'} {what}", flush=True)
    if not condition:
        raise AssertionError(what)


def run(binary: Path, work: Path) -> None:
    r = Receiver(binary, work)
    try:
        status, _, raw = r.get("/health")
        check(status == 200 and json.loads(raw)["ok"], "health answers")

        status, _, raw = r.get("/", {"Accept": "text/html"})
        check(status == 200 and b"<title>x-link-collector</title>" in raw, "the panel is served")

        status, _, raw = r.request("/", TWEET.encode(), {
            "Content-Type": "text/plain;charset=UTF-8", "Origin": "chrome-extension://smoke"}, "POST")
        check(status == 200 and json.loads(raw)["added"] == 1, "a capture is accepted")
        check(r.links.read_text().splitlines() == [CLEAN], "and written to links.txt, cleaned")

        status, _, _ = r.request("/", b"https://x.com/a/status/1", {"Origin": "https://evil.example"}, "POST")
        check(status == 403, "a web page cannot add links")
        status, _, _ = r.get("/api/status", {"Host": "evil.example"})
        check(status == 421, "nor read the panel through another name")

        status, s = r.api("/api/status")
        check(s["total"] == 1 and s["downloads"]["pending"] == 1, "status counts the link")

        status, view = r.api("/api/settings", {"jobs": 2})
        check(status == 200 and json.loads(r.config.read_text())["jobs"] == 2, "settings are saved")

        status, tools = r.api("/api/tools?refresh=1")
        check(tools.get("downloader") and tools.get("flattener") and tools.get("python_version"),
              "the tools and Python are found")

        (r.media / "alice").mkdir(parents=True)
        (r.media / "alice" / "1-1.mp4").write_bytes(bytes(range(256)) * 4)
        status, headers, raw = r.get("/media/alice/1-1.mp4", {"Range": "bytes=10-19"})
        check(status == 206 and raw == bytes(range(10, 20)), "media is served by range")
        status, _, _ = r.get("/media/..%2flinks.txt")
        check(status == 404, "nothing outside the media folder is")

        status, started = r.api("/api/job", {"kind": "flatten", "dry_run": True})
        check(status == 200, "a job starts")
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            _, snap = r.api("/api/job?since=0")
            if snap["job"] and snap["job"]["state"] not in ("running", "stopping"):
                break
            time.sleep(0.3)
        text = "\n".join(line["text"] for line in snap["lines"])
        check(snap["job"]["state"] == "done" and "would move" in text, "and runs x-flatten to the end")

        # A job that would run for a minute, to stop it: SIGINT to its group on
        # Unix, taskkill on Windows. A stand-in downloader keeps X out of it.
        fake = work / "fake-tools"
        (fake / "downloader").mkdir(parents=True)
        (fake / "spliter").mkdir()
        (fake / "downloader" / "x-download").write_text(
            "import time\nprint('[1/9] ok  someone/1-1.mp4', flush=True)\ntime.sleep(60)\n")
        (fake / "spliter" / "x-flatten").write_text("print('unused')\n")
        r.api("/api/settings", {"tools_dir": str(fake)})
        status, _ = r.api("/api/job", {"kind": "download"})
        check(status == 200, "a long job starts")
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            _, snap = r.api("/api/job?since=0")
            if snap["job"]["done"] == 1:
                break
            time.sleep(0.2)
        check(snap["job"]["state"] == "running" and snap["job"]["total"] == 9, "its progress is read")
        r.api("/api/job/stop", {})
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            _, snap = r.api("/api/job?since=0")
            if snap["job"]["state"] not in ("running", "stopping"):
                break
            time.sleep(0.2)
        check(snap["job"]["state"] == "stopped", "and Stop ends it")

        status, _ = r.api("/api/quit", {})
        check(status == 200, "quit is accepted")
        check(r.process.wait(timeout=20) == 0, "and the receiver exits cleanly")
    finally:
        if r.process.poll() is None:
            r.process.kill()
        r.log.close()
        if sys.exc_info()[0] is not None:
            # Given a console of its own (as on a Windows runner) the receiver
            # logs to a file next to its settings instead of to stdout.
            for log in (work / "receiver.log", r.config.parent / "receiver.log"):
                if log.exists():
                    print(f"--- {log.name}:\n{log.read_text(errors='replace')}")


def main() -> int:
    binary = Path(sys.argv[1]).resolve()
    print(f"smoke test: {binary}")
    with tempfile.TemporaryDirectory() as tmp:
        try:
            run(binary, Path(tmp))
        except AssertionError as e:
            print(f"failed: {e}")
            return 1
    print("all good")
    return 0


if __name__ == "__main__":
    sys.exit(main())
