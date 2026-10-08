# SPDX-License-Identifier: Apache-2.0
# Copyright 2026 abel0x <https://github.com/abel0x>
"""
Tests for spliter/x-flatten, run against a throwaway media folder.

    python3 -m unittest discover -s tests      (or: make test-py)
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from test_x_download import load

xf = load("spliter/x-flatten", "x_flatten")


class Flatten(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.src = Path(self.tmp.name) / "x-media"
        for name in ["alice/1-1.mp4", "alice/2-1.jpg", "bob/3-1.mp4"]:
            path = self.src / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(name)
        (self.src / ".x-download-state.json").write_text("{}")
        # The panel's settings file must not leak into these runs.
        self.env = mock.patch.dict(os.environ, {"X_LINK_COLLECTOR_CONFIG": str(Path(self.tmp.name) / "none.json")})
        self.env.start()

    def tearDown(self):
        self.env.stop()
        self.tmp.cleanup()

    def run_tool(self, *args: str) -> str:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(xf.main(["--src", str(self.src), *args]), 0)
        return out.getvalue()

    def test_moves_into_one_folder_and_back(self):
        dest = self.src / "x-media"
        self.run_tool()
        self.assertEqual(sorted(p.name for p in dest.iterdir()),
                         [".x-flatten-manifest.json", "1-1.mp4", "2-1.jpg", "3-1.mp4"])
        self.assertFalse((self.src / "alice").exists(), "emptied folders are removed")
        self.assertTrue((self.src / ".x-download-state.json").exists())
        self.assertIn("no files to move", self.run_tool(), "a second run finds nothing")

        self.run_tool("--undo")
        self.assertEqual((self.src / "alice" / "1-1.mp4").read_text(), "alice/1-1.mp4")
        self.assertEqual((self.src / "bob" / "3-1.mp4").read_text(), "bob/3-1.mp4")
        self.assertFalse(dest.exists())

    def test_dry_run_changes_nothing(self):
        output = self.run_tool("--dry-run")
        self.assertIn("would move", output)
        self.assertTrue((self.src / "alice" / "1-1.mp4").exists())
        self.assertFalse((self.src / "x-media").exists())

    def test_options(self):
        dest = Path(self.tmp.name) / "flat"
        self.run_tool("--dest", str(dest), "--prefix-handle", "--videos-only", "--copy")
        self.assertEqual(sorted(p.name for p in dest.iterdir()), ["alice-1-1.mp4", "bob-3-1.mp4"])
        self.assertTrue((self.src / "alice" / "1-1.mp4").exists(), "--copy leaves the originals")

    def test_names_never_overwrite(self):
        taken: set[str] = set()
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp)
            (dest / "1-1.mp4").write_text("already here")
            self.assertEqual(xf.unique(dest, "1-1.mp4", taken), "1-1~2.mp4")
            self.assertEqual(xf.unique(dest, "1-1.mp4", taken), "1-1~3.mp4")

    def test_panel_settings_are_the_defaults(self):
        config = Path(self.tmp.name) / "config.json"
        dest = Path(self.tmp.name) / "from-settings"
        config.write_text(json.dumps({"media_dir": str(self.src), "flatten_dest": str(dest)}))
        with mock.patch.dict(os.environ, {"X_LINK_COLLECTOR_CONFIG": str(config)}):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(xf.main(["--dry-run"]), 0)
                self.assertIn(str(dest), out.getvalue())
                out.truncate(0)
                xf.main(["--dry-run", "--no-config", "--src", str(self.src)])
            self.assertIn(str(self.src / "x-media"), out.getvalue())


if __name__ == "__main__":
    unittest.main()
