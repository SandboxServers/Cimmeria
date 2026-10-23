"""Regression tests for stable day slots and publication precedence."""
import datetime as dt
import unittest
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from release_identity import choose, stable

RUN = "https://github.com/example/repo/actions/runs/42"


def epoch(day="2026-10-23"):
    return int(dt.datetime.fromisoformat(day).replace(tzinfo=dt.timezone.utc).timestamp())


def release(tag, version, run=RUN):
    return dict(tag_name=tag, created_at="2026-10-23T00:00:00Z",
                body=f"Stargate Worlds launcher {version}, built by {run}.")


class IdentityTests(unittest.TestCase):
    def test_first_second_third_and_legacy_migration(self):
        history = []
        for index in range(1, 4):
            sha = f"{index:07x}"
            identity = choose([history], epoch(), sha, RUN)
            self.assertEqual(identity["version"], f"2026.10.{2300+index}")
            history.append(release(identity["tag"], identity["version"]))
        history = [release("launcher-20261023-aaaaaaa", "2026.10.23"),
                   release("launcher-20261023-bbbbbbb", "2026.10.23-2")]
        self.assertEqual(choose([history], epoch(), "ccccccc", RUN)["version"], "2026.10.2303")

    def test_pagination_and_deleted_slots_never_reuse_a_version(self):
        pages = [[release("launcher-20261023-aaaaaaa", "2026.10.2309")], []]
        self.assertEqual(choose(pages, epoch(), "bbbbbbb", RUN)["version"], "2026.10.2310")

    def test_deleted_dated_release_cannot_reuse_rolling_feed_version(self):
        self.assertEqual(choose([[]], epoch(), "aaaaaaa", RUN, "2026.10.2307")["version"], "2026.10.2308")
        with self.assertRaisesRegex(ValueError, "not above launcher-current"):
            choose([[]], epoch(), "aaaaaaa", RUN, "2026.11.101")

    def test_slot_99_and_limit(self):
        pages = [[release("launcher-20261023-aaaaaaa", "2026.10.2398")]]
        self.assertEqual(choose(pages, epoch(), "bbbbbbb", RUN)["version"], "2026.10.2399")
        pages[0][0]["body"] = "Stargate Worlds launcher 2026.10.2399,"
        with self.assertRaisesRegex(ValueError, "99 releases"):
            choose(pages, epoch(), "bbbbbbb", RUN)

    def test_old_installs_and_boundaries(self):
        for before, after in [("2026.10.23", "2026.10.2301"),
                              ("2026.10.2399", "2026.10.2401"),
                              ("2026.10.3199", "2026.11.101"),
                              ("2026.12.3199", "2027.1.101")]:
            self.assertLess(stable(before), stable(after))

    def test_rerun_retains_identity_and_another_run_refuses(self):
        pages = [[release("launcher-20261023-aaaaaaa", "2026.10.2307")]]
        self.assertEqual(choose(pages, epoch(), "aaaaaaa", RUN)["version"], "2026.10.2307")
        with self.assertRaisesRegex(ValueError, "another run"):
            choose(pages, epoch(), "aaaaaaa", RUN + "0")

    def test_future_published_version_refuses(self):
        with self.assertRaisesRegex(ValueError, "not above"):
            choose([[release("launcher-20261024-aaaaaaa", "2026.10.2401")]], epoch(), "bbbbbbb", RUN)

    def test_reservation_retains_identity_across_midnight_and_partial_retry(self):
        script = Path(__file__).with_name("release_identity.py").resolve()
        with tempfile.TemporaryDirectory() as root:
            env = dict(os.environ, STAMP_EPOCH=str(epoch()), GITHUB_SHA="aaaaaaa",
                       GITHUB_SERVER_URL="https://github.com", GITHUB_REPOSITORY="example/repo",
                       GITHUB_RUN_ID="42", GITHUB_OUTPUT=str(Path(root) / "out"))
            def stamp(pages):
                return subprocess.run([sys.executable, str(script), "stamp"], input=json.dumps(pages),
                                      text=True, capture_output=True, cwd=root, env=env)
            first = stamp([[]])
            self.assertEqual(first.returncode, 0, first.stderr)
            saved = (Path(root) / "stamp-reservation/identity.json").read_bytes()
            env["STAMP_EPOCH"] = str(epoch("2026-10-24"))
            second = stamp([[release("launcher-20261023-bbbbbbb", "2026.10.2302")]])
            self.assertEqual(second.returncode, 0, second.stderr)
            self.assertEqual((Path(root) / "stamp-reservation/identity.json").read_bytes(), saved)
            env["GITHUB_RUN_ID"] = "43"
            self.assertNotEqual(stamp([[]]).returncode, 0)

    def test_current_refuses_regression_and_accepts_repair_or_newer(self):
        script = Path(__file__).with_name("release_identity.py").resolve()
        with tempfile.TemporaryDirectory() as root:
            previous, candidate = Path(root) / "old", Path(root) / "new"
            previous.write_text(json.dumps({"version":"2026.10.2302"}))
            for version, succeeds in [("2026.10.2301", False), ("2026.10.2302", True), ("2026.10.2303", True)]:
                candidate.write_text(json.dumps({"version":version}))
                result = subprocess.run([sys.executable, str(script), "current", str(candidate), str(previous)], capture_output=True)
                self.assertEqual(result.returncode == 0, succeeds)

    def test_workflow_requires_signing_on_every_release(self):
        workflow = Path(__file__).resolve().parents[2] / ".github/workflows/launcher-release.yml"
        text = workflow.read_text(encoding="utf-8")
        sign = text.split("\n  sign:\n", 1)[1].split("\n  publish:\n", 1)[0]
        self.assertNotIn("same_day", text)
        self.assertIn("if: github.ref == 'refs/heads/main'", sign)
        self.assertIn('if [ "${HAS_UPDATER_KEY}" != true ]', sign)
        self.assertIn("&& needs.sign.result == 'success' && needs.sign.outputs.signed == 'true'", text)
        self.assertNotIn("needs.sign.result == 'skipped'", text)

    def test_malformed_identity_and_windows_bounds(self):
        for value in ["2026.10.23-2", "2026.10.2301+2", "65536.1.1", "2026.10.65536"]:
            with self.assertRaises(ValueError):
                stable(value)
        with self.assertRaisesRegex(ValueError, "malformed"):
            choose([[release("launcher-garbage", "2026.10.23")]], epoch(), "bbbbbbb", RUN)


if __name__ == "__main__":
    unittest.main()
