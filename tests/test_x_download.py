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
        self.assertEqual(xd.config_defaults({"remember_downloads": False}), {"again_missing": True})
        self.assertEqual(xd.config_defaults({"remember_downloads": True}), {})

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

    def test_only_tweets_on_x_are_fetched(self):
        for ok in ("https://x.com/a/status/1", "https://twitter.com/a/status/1?s=20",
                   "http://mobile.twitter.com/a/status/1", "https://x.com/i/web/status/9",
                   "https://WWW.X.COM/a/status/1/photo/1"):
            self.assertTrue(xd.is_x_link(ok), ok)
        for bad in ("https://example.com/a/status/1", "https://x.com.evil.tld/a/status/1",
                    "http://192.168.1.1/status/1", "file:///x.com/a/status/1",
                    "https://x.com/home", "https://evil.tld/?u=https://x.com/a/status/1", ""):
            self.assertFalse(xd.is_x_link(bad), bad)

    def test_other_links_never_reach_the_tools(self):
        opts = SimpleNamespace(mirror_only=False, mirror=True)
        tools = xd.Tools(ytdlp=["yt-dlp"], gallery_dl=["gallery-dl"])
        with mock.patch.object(xd, "run") as run, mock.patch.object(xd, "http_json") as api:
            result = xd.fetch("http://192.168.1.1/status/1", opts, tools)
        run.assert_not_called()
        api.assert_not_called()
        self.assertEqual((result.status, result.permanent), (xd.FAILED, True))

    def test_a_mirror_cannot_point_at_local_files(self):
        answer = {"tweet": {"author": {"screen_name": "alice"}, "media": {"all": [
            {"type": "video", "url": "file:///home/someone/.ssh/id_ed25519"},
            {"type": "photo", "url": "https://pbs.twimg.com/media/a.jpg"},
        ]}}}
        fetched = []
        with tempfile.TemporaryDirectory() as tmp:
            opts = SimpleNamespace(out=Path(tmp), timeout=30)
            with mock.patch.object(xd, "http_json", return_value=answer), \
                 mock.patch.object(xd, "download_file",
                                   side_effect=lambda src, dest, t: fetched.append(src) or True):
                result = xd.run_mirror("https://x.com/alice/status/5", opts)
        self.assertEqual(fetched, ["https://pbs.twimg.com/media/a.jpg?name=orig"])
        self.assertEqual(result.status, xd.DONE)

    def test_the_panel_translates_every_reason_given(self):
        """Each failure text x-download writes has an entry in the panel's WHY table."""
        import re
        panel = (ROOT / "receiver/panel/panel.js").read_text(encoding="utf-8")
        table = panel[panel.index("const WHY = {"):]
        table = table[:table.index("};")]
        known = set(re.findall(r"^\s*'([^']+)': 'why\.", table, re.M))
        source = (ROOT / "tools/x-download").read_text(encoding="utf-8")
        written = {m for _, m in xd.ERROR_MAP}
        written |= set(re.findall(r'error="([^"]+)"', source))
        self.assertEqual(written - known, set())
        words = {lang: json.loads((ROOT / f"receiver/panel/i18n/{lang}.json").read_text(encoding="utf-8"))
                 for lang in ("en", "tr")}
        for key in set(re.findall(r"'(why\.\w+)'", table)):
            self.assertIn(key, words["en"])
            self.assertIn(key, words["tr"])

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
            opts = SimpleNamespace(retry_failed=False, again=None)
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

    def test_finds_a_tweet_under_another_handle(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = xd.State(Path(tmp) / xd.HISTORY_FILE)
            state.record("https://x.com/alice/status/7", status=xd.DONE)
            self.assertEqual(state.get("https://twitter.com/bob/status/7?s=20")["status"], xd.DONE)
            state.record("https://x.com/bob/status/7", status=xd.FAILED)
            self.assertEqual(list(state.entries), ["https://x.com/alice/status/7"])

    def test_asked_for_again_comes_along_whatever_the_record_says(self):
        with tempfile.TemporaryDirectory() as tmp:
            state = xd.State(Path(tmp) / xd.HISTORY_FILE)
            links = [f"https://x.com/a/status/{n}" for n in (1, 2, 3)]
            for url in links:
                state.record(url, status=xd.DONE)
            opts = SimpleNamespace(retry_failed=False, again=["https://x.com/zed/status/2"])
            self.assertEqual(xd.pending(links, state, opts), [links[1]])
            gone = {xd.link_key(links[2])}
            self.assertEqual(xd.pending(links, state, opts, gone), links[1:])


class History(unittest.TestCase):
    """Moving the media folder away must not make the downloader forget."""

    def test_the_old_state_in_the_media_folder_is_taken_over(self):
        with tempfile.TemporaryDirectory() as tmp:
            media = Path(tmp) / "x-media"
            media.mkdir()
            (media / xd.STATE_FILE).write_text(json.dumps({"version": 1, "entries": {
                "https://x.com/a/status/1": {"status": "done", "files": ["a/1-1.mp4"]},
                "https://x.com/a/status/2": {"status": "failed", "attempts": 1},
            }}), encoding="utf-8")
            history = Path(tmp) / "config" / xd.HISTORY_FILE
            state = xd.State(history)
            state.record("https://x.com/b/status/2", status=xd.DONE)
            self.assertEqual(state.adopt(media / xd.STATE_FILE), 1)
            self.assertEqual(state.get("https://x.com/a/status/2")["status"], xd.DONE,
                             "what the history knows wins")
            # Saved where the history lives, so it outlives the media folder.
            (media / xd.STATE_FILE).unlink()
            media.rmdir()
            again = xd.State(history)
            self.assertEqual(again.get("https://x.com/a/status/1")["status"], xd.DONE)
            self.assertEqual(again.adopt(media / xd.STATE_FILE), 0)

    def test_files_are_found_by_tweet_id_wherever_they_are(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "alice").mkdir()
            (root / "alice" / "100-1.mp4").write_bytes(b"")
            (root / "x-media").mkdir()
            (root / "x-media" / "bob-200-2~2.jpg").write_bytes(b"")
            (root / "alice" / "300-1.mp4.part").write_bytes(b"")
            (root / "notes-400.txt").write_bytes(b"")
            self.assertEqual(xd.ids_on_disk([root, root / "missing"]), {"100", "200"})

    def test_a_link_is_gone_once_its_files_are(self):
        present = {"100"}
        done = {"status": xd.DONE}
        self.assertFalse(xd.is_gone("https://x.com/a/status/100", done, present))
        self.assertTrue(xd.is_gone("https://x.com/a/status/101", done, present))
        self.assertFalse(xd.is_gone("https://x.com/a/status/101", {"status": xd.FAILED}, present))
        self.assertFalse(xd.is_gone("https://example.com/clip", done, present))

    def test_again_takes_links_the_file_no_longer_has(self):
        links = ["https://x.com/a/status/1"]
        self.assertEqual(
            xd.again_too(links, ["https://x.com/b/status/1", " https://x.com/a/status/9 "]),
            ["https://x.com/a/status/1", "https://x.com/a/status/9"],
        )


if __name__ == "__main__":
    unittest.main()


class PlatformPaths(unittest.TestCase):
    """
    The desktop and settings lookups per system, run on any one of them, so
    CI needs no runner of each kind. x-flatten makes the same lookup.
    """

    CLEAN = {"HOME": "/u", "XDG_DESKTOP_DIR": "", "X_LINK_COLLECTOR_CONFIG": "",
             "XDG_CONFIG_HOME": "", "APPDATA": "", "OneDrive": "", "OneDriveConsumer": ""}

    def paths(self, module, platform: str, **env):
        with mock.patch.dict(os.environ, {**self.CLEAN, **env}), \
             mock.patch.object(module.sys, "platform", platform):
            return module.desktop_dir(), module.config_path()

    def test_macos(self):
        for module in (xd, load("tools/x-flatten", "x_flatten_paths")):
            desktop, config = self.paths(module, "darwin")
            self.assertEqual(desktop, Path("/u") / "Desktop")
            self.assertEqual(config, Path("/u") / "Library" / "Application Support"
                             / "x-link-collector" / "config.json")

    def test_windows(self):
        desktop, config = self.paths(xd, "win32", APPDATA="/u/AppData/Roaming")
        self.assertEqual(desktop, Path("/u") / "Desktop")
        self.assertEqual(config, Path("/u/AppData/Roaming") / "x-link-collector" / "config.json")

    def test_linux_reads_user_dirs(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "user-dirs.dirs").write_text('XDG_DESKTOP_DIR="$HOME/Masaüstü"\n',
                                                      encoding="utf-8")
            desktop, config = self.paths(xd, "linux", XDG_CONFIG_HOME=tmp)
        self.assertEqual(desktop, Path("/u") / "Masaüstü")
        self.assertEqual(config, Path(tmp) / "x-link-collector" / "config.json")


class Duplicates(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def put(self, rel: str, data: bytes) -> Path:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def test_the_same_file_again_is_kept_once(self):
        video = os.urandom(40 * 1024)
        first = self.put("alice/100-1.mp4", video)
        again = self.put("bob/200-1.mp4", video)
        linked = xd.Duplicates(self.root).settle(["bob/200-1.mp4"])
        self.assertEqual(linked, [("bob/200-1.mp4", str(Path("alice/100-1.mp4")), len(video))])
        self.assertTrue(os.path.samefile(first, again))
        self.assertEqual(again.read_bytes(), video)
        self.assertEqual([p.name for p in (self.root / "bob").iterdir()], ["200-1.mp4"])

    def test_different_files_of_one_size_stay_apart(self):
        self.put("alice/100-1.mp4", os.urandom(40 * 1024))
        other = self.put("bob/200-1.mp4", os.urandom(40 * 1024))
        self.assertEqual(xd.Duplicates(self.root).settle(["bob/200-1.mp4"]), [])
        self.assertFalse(os.path.samefile(self.root / "alice/100-1.mp4", other))

    def test_small_files_and_hidden_ones_are_left_alone(self):
        tiny = b"x" * 100
        self.put("alice/100-1.jpg", tiny)
        self.put("bob/200-1.jpg", tiny)
        self.put("alice/.state.json", os.urandom(40 * 1024))
        self.assertEqual(xd.Duplicates(self.root).settle(["bob/200-1.jpg"]), [])

    def test_two_new_files_in_one_run_find_each_other(self):
        video = os.urandom(40 * 1024)
        dupes = xd.Duplicates(self.root)
        self.put("alice/100-1.mp4", video)
        self.assertEqual(dupes.settle(["alice/100-1.mp4"]), [])
        self.put("bob/200-1.mp4", video)
        self.assertEqual(len(dupes.settle(["bob/200-1.mp4"])), 1)

    def test_the_setting_turns_it_off(self):
        self.assertEqual(xd.config_defaults({"link_duplicates": False}), {"keep_duplicates": True})
        self.assertEqual(xd.config_defaults({"link_duplicates": True}), {})
