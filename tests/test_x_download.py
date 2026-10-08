# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
Tests for tools/x-download that need neither yt-dlp nor the network.

    python3 -m unittest discover -s tests      (or: make test-py)
"""

from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent


def load(relative: str, name: str):
    """Import an extension-less script as a module."""
    loader = importlib.machinery.SourceFileLoader(name, str(ROOT / relative))
    spec = importlib.util.spec_from_loader(name, loader)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module  # dataclasses looks its module up here
    loader.exec_module(module)
    return module


xd = load("tools/x-download", "x_download")


class ReadLinks(unittest.TestCase):
    def read(self, text: str, encoding: str = "utf-8"):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "links.txt"
            path.write_bytes(text.encode(encoding))
            return xd.read_links(path)

    def test_dedupes_on_the_tweet_id(self):
        links, collapsed, skipped = self.read(
            "https://x.com/alice/status/1\n"
            "https://twitter.com/bob/status/1?s=20\n"
            "https://x.com/alice/status/2\n"
        )
        self.assertEqual(links, ["https://x.com/alice/status/1", "https://x.com/alice/status/2"])
        self.assertEqual((collapsed, skipped), (1, 0))

    def test_takes_bare_ids_and_drops_notes(self):
        links, _, skipped = self.read(
            "﻿# my list\n\n1790637656616943991\nremember to check this\n"
        )
        self.assertEqual(links, ["https://x.com/i/web/status/1790637656616943991"])
        self.assertEqual(skipped, 1)

    def test_only_keeps_the_links_asked_for(self):
        links = ["https://x.com/a/status/1", "https://x.com/b/status/2", "https://example.com/x"]
        self.assertEqual(xd.only(links, None), links)
        self.assertEqual(xd.only(links, ["https://twitter.com/zz/status/2?s=20"]), [links[1]])
        self.assertEqual(xd.only(links, ["https://example.com/x", "https://x.com/c/status/9"]), [links[2]])

    def test_missing_file_is_empty(self):
        self.assertEqual(xd.read_links(Path("/nonexistent/links.txt")), ([], 0, 0))


class PanelSettings(unittest.TestCase):
    def test_turns_settings_into_option_defaults(self):
        defaults = xd.config_defaults({
            "jobs": 4, "timeout": 120, "cookies": " vivaldi ", "metadata": True,
            "mirror": "only", "links_file": "/elsewhere/links.txt",
        })
        self.assertEqual(defaults, {
            "jobs": 4, "timeout": 120, "cookies": "vivaldi", "metadata": True, "mirror_only": True,
        })
        self.assertEqual(xd.config_defaults({"mirror": "fallback"}), {"mirror": True})

    def test_ignores_what_it_cannot_use(self):
        self.assertEqual(xd.config_defaults({
            "jobs": True, "timeout": 5, "cookies": "  ", "metadata": "yes", "mirror": "sometimes",
        }), {})
        self.assertEqual(xd.config_defaults({"jobs": 99}), {})

    def test_reads_the_file_the_variable_points_at(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "config.json"
            path.write_text("﻿" + json.dumps({"jobs": 2}), encoding="utf-8")
            with mock.patch.dict(os.environ, {"X_LINK_COLLECTOR_CONFIG": str(path)}):
                self.assertEqual(xd.config_path(), path)
                self.assertEqual(xd.load_config(), {"jobs": 2})
            path.write_text("not json", encoding="utf-8")
            with mock.patch.dict(os.environ, {"X_LINK_COLLECTOR_CONFIG": str(path)}):
                self.assertEqual(xd.load_config(), {})

    def test_command_line_wins_over_settings(self):
        parser = xd.build_parser()
        parser.set_defaults(**xd.config_defaults({"jobs": 5, "mirror": "only"}))
        self.assertEqual(parser.parse_args([]).jobs, 5)
        self.assertTrue(parser.parse_args([]).mirror_only)
        self.assertEqual(parser.parse_args(["--jobs", "2"]).jobs, 2)


class Helpers(unittest.TestCase):
    def test_cookie_args(self):
        self.assertEqual(xd.cookie_args(None), [])
        self.assertEqual(xd.cookie_args("vivaldi"), ["--cookies-from-browser", "vivaldi"])
        self.assertEqual(xd.cookie_args("firefox:work"), ["--cookies-from-browser", "firefox:work"])
        cookies = str(Path.home() / "x" / "cookies.txt")
        self.assertEqual(xd.cookie_args("~/x/cookies.txt"), ["--cookies", cookies])

    def test_safe_component(self):
        self.assertEqual(xd.safe_component("nul"), "_nul")
        self.assertEqual(xd.safe_component("COM1.txt"), "_COM1.txt")
        self.assertEqual(xd.safe_component('a<b>:c"d|e?f*'), "a_b__c_d_e_f_")
        self.assertEqual(xd.safe_component("..."), "_")

    def test_tweet_id(self):
        self.assertEqual(xd.tweet_id("https://x.com/a/status/123/photo/1"), "123")
        self.assertEqual(xd.tweet_id("https://x.com/i/web/status/9"), "9")
        self.assertIsNone(xd.tweet_id("https://example.com/"))

    def test_mirror_shapes(self):
        fx = {"tweet": {"author": {"screen_name": "alice"},
                        "media": {"all": [{"type": "photo", "url": "https://pbs/img.jpg"}]}}}
        handle, items = xd.mirror_media(fx)
        self.assertEqual(handle, "alice")
        self.assertEqual(xd.best_variant(items[0]), "https://pbs/img.jpg?name=orig")
        vx = {"user_screen_name": "bob", "media_extended": [{
            "type": "video", "url": "https://v/low.mp4",
            "formats": [{"container": "mp4", "bitrate": 100, "url": "https://v/100.mp4"},
                        {"container": "mp4", "bitrate": 900, "url": "https://v/900.mp4"},
                        {"container": "m3u8", "url": "https://v/list.m3u8"}],
        }]}
        handle, items = xd.mirror_media(vx)
        self.assertEqual((handle, xd.best_variant(items[0])), ("bob", "https://v/900.mp4"))

    def test_a_failure_outranks_no_media(self):
        no_media = xd.Result(xd.NO_MEDIA, tool="yt-dlp")
        failed = xd.Result(xd.FAILED, error="restricted", tool="gallery-dl")
        self.assertIs(xd.summarise([no_media, failed]), failed)
        self.assertIs(xd.summarise([no_media]), no_media)

    def test_errors_read_as_advice(self):
        self.assertEqual(xd.last_error("ERROR: HTTP Error 429: Too Many Requests"),
                         "rate limited by X -- wait a bit, or lower --jobs")
        self.assertEqual(xd.last_error("something odd\nERROR: unable to frob"), "ERROR: unable to frob")


class Pending(unittest.TestCase):
    def test_skips_what_is_settled(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = xd.State(Path(tmp) / xd.STATE_FILE)
            state.record("https://x.com/a/status/1", status=xd.DONE)
            state.record("https://x.com/a/status/2", status=xd.NO_MEDIA)
            state.record("https://x.com/a/status/3", status=xd.FAILED, attempts=1)
            state.record("https://x.com/a/status/4", status=xd.FAILED, attempts=xd.MAX_ATTEMPTS)
            links = [f"https://x.com/a/status/{n}" for n in range(1, 6)]
            opts = SimpleNamespace(retry_failed=False)
            self.assertEqual(xd.pending(links, state, opts), [links[2], links[4]])
            opts.retry_failed = True
            self.assertEqual(xd.pending(links, state, opts), links[1:])

    def test_state_survives_a_reload(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / xd.STATE_FILE
            state = xd.State(path)
            state.record("https://x.com/a/status/1", status=xd.DONE, files=["a/1-1.mp4"])
            state.save(force=True)
            again = xd.State(path)
            self.assertEqual(again.get("https://x.com/a/status/1")["files"], ["a/1-1.mp4"])
            self.assertEqual(again.counts(), {xd.DONE: 1})


if __name__ == "__main__":
    unittest.main()
