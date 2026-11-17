#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
Every page of the panel, in English and in Turkish, in a real headless
browser: the page is in the language asked for, each heading has text, and
no untranslated key ("nav.links", "ga.title") reaches the screen.

    python3 tests/panel_dom.py receiver/target/release/x-link-receiver

Needs Chrome or Chromium (set CHROME to pick one).
"""

from __future__ import annotations

import json
import re
import sys
import tempfile
from html.parser import HTMLParser
from pathlib import Path

import demo

VIEWS = ["overview", "links", "gallery", "download", "folder", "settings"]
# What t() hands back when a key is missing is the key itself. The few keys
# without a dot ("copy", "cancel") are plain words, and words are on the page.
KEYS = {k for k in json.loads((demo.ROOT / "receiver/panel/i18n/en.json").read_text(encoding="utf-8"))
        if "." in k}
WORD = re.compile(r"[\w.-]+")
# One word each page shows in that language, and not in the other.
SAYS = {"en": "Settings", "tr": "Ayarlar"}


class Page(HTMLParser):
    """The visible text of the page, the <html lang>, and the text of each h1."""

    SKIP = {"script", "style", "svg", "template"}

    def __init__(self):
        super().__init__()
        self.lang = ""
        self.text: list[str] = []
        self.h1: dict[str, str] = {}
        self.hidden = 0
        self.skip = 0
        self.in_h1 = None
        self.stack: list[tuple[str, bool]] = []

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "html":
            self.lang = a.get("lang", "")
        hidden = "hidden" in a
        self.stack.append((tag, hidden))
        self.hidden += hidden
        self.skip += tag in self.SKIP
        if tag == "h1":
            self.in_h1 = a.get("id", "")
            self.h1[self.in_h1] = ""

    def handle_endtag(self, tag):
        while self.stack:
            name, hidden = self.stack.pop()
            self.hidden -= hidden
            self.skip -= name in self.SKIP
            if name == "h1":
                self.in_h1 = None
            if name == tag:
                break

    def handle_data(self, data):
        if self.in_h1 is not None:
            self.h1[self.in_h1] += data
        if not self.hidden and not self.skip and data.strip():
            self.text.append(data.strip())


def check(base: str, view: str, lang: str) -> list[str]:
    run = demo.headless(f"{base}/#{view}", lang, "--dump-dom")
    page = Page()
    page.feed(run.stdout)
    text = " ".join(page.text)
    problems = []
    if page.lang != lang:
        problems.append(f'<html lang="{page.lang}">, not {lang}')
    raw = sorted({w for w in WORD.findall(text) if w in KEYS})
    if raw:
        problems.append(f"untranslated: {', '.join(raw)}")
    title = {"overview": "ov-title", "links": "links-title", "gallery": "ga-title",
             "download": "dl-title", "folder": "fo-title", "settings": "st-title"}[view]
    if not page.h1.get(title, "").strip():
        problems.append(f"#{title} is empty")
    if SAYS[lang] not in text:
        problems.append(f"no {SAYS[lang]!r} on the page")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    binary = Path(argv[0]).resolve()
    failed = 0
    with tempfile.TemporaryDirectory(prefix="x-link-dom-") as tmp:
        config = demo.make(Path(tmp))
        with demo.serve(binary, config, demo.free_port()) as base:
            for lang in ("en", "tr"):
                for view in VIEWS:
                    problems = check(base, view, lang)
                    failed += bool(problems)
                    mark = "ok " if not problems else "FAIL"
                    print(f"  {mark} {lang} #{view}" + "".join(f"\n       {p}" for p in problems))
    print("all good" if not failed else f"{failed} page(s) failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
