#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
Take the README's pictures of the panel again: docs/panel.png (light) and
docs/panel-dark.png (dark), from the made-up collection in demo.py.

    python3 tests/screenshots.py receiver/target/release/x-link-receiver
    (or: make screenshots)

The panel says which port it runs on, and the pictures should say 9876. When
a receiver of your own already has that port, Linux runs this in a network
namespace of its own (unshare), where 9876 is free; elsewhere, stop yours
first. Needs Chrome or Chromium (set CHROME to pick one).
"""

from __future__ import annotations

import os
import shutil
import socket
import subprocess
import sys
import tempfile
from pathlib import Path

import demo

PORT = 9876
SHOTS = {"panel.png": "1", "panel-dark.png": "0"}  # preferredColorScheme: 1 light, 0 dark


def port_free(port: int) -> bool:
    with socket.socket() as s:
        try:
            s.bind(("127.0.0.1", port))
        except OSError:
            return False
    return True


def shoot(binary: Path) -> None:
    docs = demo.ROOT / "docs"
    with tempfile.TemporaryDirectory(prefix="x-link-shots-") as tmp:
        config = demo.make(Path(tmp))
        with demo.serve(binary, config, PORT) as base:
            for name, scheme in SHOTS.items():
                run = demo.headless(f"{base}/#overview", "en", "--window-size=1200,700",
                                    f"--blink-settings=preferredColorScheme={scheme}",
                                    f"--screenshot={docs / name}")
                if not (docs / name).is_file() or run.returncode:
                    raise SystemExit(f"no picture for {name}:\n{run.stderr[-2000:]}")
                print(f"  wrote docs/{name}")


def main(argv: list[str]) -> int:
    inside = argv[:1] == ["--inside"]
    argv = argv[1:] if inside else argv
    if len(argv) != 1:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    binary = Path(argv[0]).resolve()
    if port_free(PORT):
        shoot(binary)
        return 0
    if inside or not sys.platform.startswith("linux") or not shutil.which("unshare"):
        print(f"port {PORT} is taken; stop the receiver using it and run this again",
              file=sys.stderr)
        return 1
    # A network namespace of our own, loopback up, where nothing holds 9876.
    script = Path(__file__).resolve()
    inner = f"ip link set lo up && exec {sys.executable} {script} --inside {binary}"
    return subprocess.call(["unshare", "--map-root-user", "--net", "sh", "-c", inner],
                           env=dict(os.environ))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
