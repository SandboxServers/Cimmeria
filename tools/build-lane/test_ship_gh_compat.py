"""Tests for gh_compat.py, and for ship.py against an old gh (apt gh 2.45): the checks
fall back to statusCheckRollup, and restack retargets through the REST API (AH-11d)."""

from __future__ import annotations

import subprocess
import sys
import unittest
from unittest import mock

import gh_compat  # noqa: E402
import ship  # noqa: E402
from test_ship import FAKE_GH, GREEN, MergeCase  # noqa: E402

# Runs before FAKE_GH's body. It answers the four calls an old gh gets wrong, and every
# other call falls through to FAKE_GH, which records its own call (so this must not save
# the calls it only appends).
OLD_GH_PRELUDE = r'''
import json, os, sys
sys.stdout.reconfigure(newline="\n")
path = os.environ["FAKE_GH_STATE"]
st = json.load(open(path))
args = sys.argv[1:]
st["calls"].append(args)
def answer(out="", err="", code=0):
    json.dump(st, open(path, "w"))
    sys.stdout.write(out)
    sys.stderr.write(err)
    sys.exit(code)
if args[:2] == ["pr", "checks"] and "--json" in args:
    answer(err="unknown flag: --json\n", code=1)
if args[:2] == ["pr", "edit"]:
    answer(err="GraphQL: Projects (classic) is being deprecated in favor of the new Projects experience\n", code=1)
if args[:1] == ["api"] and "PATCH" in args:
    answer(out="{}\n")
if args[:2] == ["pr", "view"] and "statusCheckRollup" in args:
    seq = st["rollup"]
    frame = seq.pop(0) if len(seq) > 1 else seq[0]
    answer(out=json.dumps({"statusCheckRollup": frame}) + "\n")
'''


class FakeGh:
    """A gh callable that records its args and returns the scripted return codes in turn."""

    def __init__(self, *codes: int):
        self.codes = list(codes)
        self.calls: list[tuple] = []

    def __call__(self, *args: str, **kw) -> subprocess.CompletedProcess:
        self.calls.append(args)
        return subprocess.CompletedProcess(args, self.codes.pop(0), "", "")


def check_run(name: str, status: str, conclusion: str = "") -> dict:
    return {"__typename": "CheckRun", "name": name, "status": status, "conclusion": conclusion}


class BucketTests(unittest.TestCase):
    def test_completed_success_is_pass(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "SUCCESS")), "pass")

    def test_completed_neutral_is_pass(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "NEUTRAL")), "pass")

    def test_completed_skipped_is_skipping(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "SKIPPED")), "skipping")

    def test_completed_cancelled_is_cancel(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "CANCELLED")), "cancel")

    def test_completed_failure_is_fail(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "FAILURE")), "fail")

    def test_completed_timed_out_is_fail(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "TIMED_OUT")), "fail")

    def test_completed_action_required_is_fail(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "COMPLETED", "ACTION_REQUIRED")), "fail")

    def test_in_progress_is_pending(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "IN_PROGRESS")), "pending")

    def test_queued_is_pending(self):
        self.assertEqual(gh_compat.bucket(check_run("a", "QUEUED")), "pending")

    def test_status_context_success_is_pass(self):
        self.assertEqual(gh_compat.bucket({"context": "a", "state": "SUCCESS"}), "pass")

    def test_status_context_pending_is_pending(self):
        self.assertEqual(gh_compat.bucket({"context": "a", "state": "PENDING"}), "pending")

    def test_status_context_expected_is_pending(self):
        self.assertEqual(gh_compat.bucket({"context": "a", "state": "EXPECTED"}), "pending")

    def test_status_context_failure_is_fail(self):
        self.assertEqual(gh_compat.bucket({"context": "a", "state": "FAILURE"}), "fail")

    def test_status_context_error_is_fail(self):
        self.assertEqual(gh_compat.bucket({"context": "a", "state": "ERROR"}), "fail")


class SetBaseTests(unittest.TestCase):
    def test_edit_ok_is_one_call(self):
        gh = FakeGh(0)
        self.assertTrue(gh_compat.set_base(gh, "8", "main", "/repo"))
        self.assertEqual(gh.calls, [("pr", "edit", "8", "--base", "main")])

    def test_api_retargets_when_edit_fails(self):
        gh = FakeGh(1, 0)
        self.assertTrue(gh_compat.set_base(gh, "8", "main", "/repo"))
        self.assertEqual(gh.calls[1], ("api", "-X", "PATCH", "repos/{owner}/{repo}/pulls/8", "-f", "base=main"))

    def test_both_fail_is_false(self):
        gh = FakeGh(1, 1)
        self.assertFalse(gh_compat.set_base(gh, "8", "main", "/repo"))


class OldGhCase(MergeCase):
    def setUp(self):
        super().setUp()
        bindir = self.tmp / "bin"
        (bindir / "old_gh.py").write_text(OLD_GH_PRELUDE + FAKE_GH)
        self.patches[0].stop()
        self.patches[0] = mock.patch.object(ship, "GH", [sys.executable, str(bindir / "old_gh.py")])
        self.patches[0].start()

    def set_rollup(self, frames: list[list[dict]]):
        st = self.gh_state()
        st["rollup"] = frames
        self.save_state(st)


class OldGhTests(OldGhCase):
    def test_checks_fall_back_to_the_status_rollup(self):
        self.add_pr(["src.rs"])
        self.set_rollup([
            [check_run(g, "IN_PROGRESS") for g in GREEN],
            [check_run(g, "COMPLETED", "SUCCESS") for g in GREEN],
        ])
        code, f, line = self.merge("--timeout", "5s")
        self.assertEqual(code, 0, line)
        self.assertIn(["pr", "merge", "7", "--squash"], self.calls(["pr", "merge"]))
        self.assertTrue(any("statusCheckRollup" in c for c in self.calls(["pr", "view"])))

    def test_a_failed_rollup_check_stops_the_merge(self):
        self.add_pr(["src.rs"])
        self.set_rollup([[
            check_run(g, "COMPLETED", "FAILURE" if g == "cargo clippy -D warnings" else "SUCCESS") for g in GREEN
        ]])
        # A short timeout so a revert (checks always []) fails in seconds, not after 30 minutes.
        code, f, _ = self.merge("--timeout", "3s")
        self.assertEqual((code, f["status"], f["checks"]), (1, "ci-failed", "cargo clippy -D warnings"))
        self.assertEqual(self.calls(["pr", "merge"]), [])

    def test_restack_retargets_through_the_rest_api(self):
        children = [{"number": 8, "baseRefName": "feat"}]
        # LOG is the open log of a merge run; restack alone runs without one.
        with mock.patch.object(ship.ship_queue, "stacked_children", return_value=children), \
                mock.patch.object(ship, "REBASE", [sys.executable, "-c", "pass"]), \
                mock.patch.object(ship, "LOG", None):
            done, failed = ship.restack(str(self.repo.main), "feat", "0" * 40)
        self.assertEqual((done, failed), (["8"], []))
        self.assertIn(["api", "-X", "PATCH", "repos/{owner}/{repo}/pulls/8", "-f", "base=main"],
                      self.gh_state()["calls"])


if __name__ == "__main__":
    unittest.main()
