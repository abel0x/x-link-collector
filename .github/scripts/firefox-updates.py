#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
The update manifest Firefox reads from the add-on's update_url: this version,
where its signed .xpi is in this release, and the file's SHA-256.

    firefox-updates.py extension/manifest.json x-link-collector-firefox.xpi v1.5.0 > updates.json
"""

import hashlib
import json
import sys
from pathlib import Path

REPO = "https://github.com/abel0x/x-link-collector"


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    manifest = json.loads(Path(argv[0]).read_text(encoding="utf-8"))
    xpi = Path(argv[1])
    tag = argv[2]
    gecko = manifest["browser_specific_settings"]["gecko"]
    version = manifest["version"]
    if tag.lstrip("v") != version:
        print(f"tag {tag} is not the manifest's version {version}", file=sys.stderr)
        return 1
    update = {
        "version": version,
        "update_link": f"{REPO}/releases/download/{tag}/{xpi.name}",
        "update_hash": "sha256:" + hashlib.sha256(xpi.read_bytes()).hexdigest(),
        "applications": {"gecko": {"strict_min_version": gecko["strict_min_version"]}},
    }
    json.dump({"addons": {gecko["id"]: {"updates": [update]}}}, sys.stdout, indent=2)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
