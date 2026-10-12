"""The ledger parser: packet tables, normalization, retry reasons, PR map and windows; fixtures and the real ledger."""

import contextlib
import io
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from metrics import ledger
from metrics.cli import main

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
FIXTURES = HERE / "fixtures"


def _fixture_rows():
    return ledger.parse_packet_table((FIXTURES / "ledger_readme.md").read_text(encoding="utf-8"))


class ParseTest(unittest.TestCase):
    def test_packet_table_only(self):
        rows = _fixture_rows()
        self.assertEqual([r["id"] for r in rows], ["FX-01", "FX-02", "FX-03"])
        self.assertEqual(rows[1]["title"], "Inbound decoder")
        self.assertEqual(rows[1]["depends"], "FX-01")

    def test_statuses_and_prs(self):
        rows = [ledger.normalize(r) for r in _fixture_rows()]
        self.assertEqual([r["status"] for r in rows], ["Integrated", "Integrated", "Ready"])
        self.assertEqual([r["prs"] for r in rows], [[1527], [1627], []])

    def test_escalation_from_parenthetical(self):
        # Reverting the parenthetical extraction leaves the note empty and fails here.
        rows = [ledger.normalize(r) for r in _fixture_rows()]
        self.assertEqual([r["implementer"] for r in rows],
                         ["packet-coder", "rust-gameserver-dev", "documentation-writer"])
        self.assertEqual(rows[1]["implementer_note"], "Haiku ran out of context")
        self.assertEqual([r["escalated"] for r in rows], [False, True, False])
        self.assertEqual(ledger.retry_reason(rows[1]["implementer_note"]), "context-exhausted")

    def test_worknote_head_reason(self):
        head = (FIXTURES / "ledger_worknote.md").read_text(encoding="utf-8").splitlines()[:60]
        self.assertEqual(ledger.retry_reason("\n".join(head)), "context-exhausted")
        waiting = "Waiting for the notification from a background Bash"
        self.assertEqual(ledger.retry_reason(waiting), "stalled-background")
        self.assertEqual(ledger.retry_reason("clippy failed"), "ci-failure")
        self.assertEqual(ledger.retry_reason(""), "other")

    def test_real_lab_record_ledger(self):
        text = (REPO / "docs" / "analysis" / "lab-record" / "README.md").read_text(encoding="utf-8")
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertGreaterEqual(len(rows), 10)
        lr02 = next(r for r in rows if r["id"] == "LR-02")
        self.assertTrue(lr02["escalated"])
        self.assertEqual(lr02["status"], "Integrated")


class MapTest(unittest.TestCase):
    def test_pr_map_and_conflicts(self):
        a = [{"prs": [1527, 1627]}]
        b = [{"prs": [1627, 1634]}]
        pr_map, conflicts = ledger.campaign_pr_map({"lab-record": a, "lab-chaos": b})
        self.assertEqual(pr_map, {1527: "lab-record", 1634: "lab-chaos"})
        self.assertEqual(conflicts, [{"pr": 1627, "campaigns": ["lab-chaos", "lab-record"]}])

    def test_window_start_and_newest_closeout(self):
        def git_log(args):
            if "--reverse" in args:
                return "2026-09-21T10:00:00+00:00\n2026-09-22T10:00:00+00:00\n"
            return ("2026-10-02T09:00:00+00:00\tclose-out: lab record\n"
                    "2026-10-01T09:00:00+00:00\tdocs\n"
                    "2026-09-30T09:00:00+00:00\tcloseout pass\n")
        self.assertEqual(ledger.campaign_window("lab-record", git_log),
                         {"start": "2026-09-21T10:00:00+00:00", "end": "2026-10-02T09:00:00+00:00"})

    def test_window_without_closeout(self):
        self.assertEqual(ledger.campaign_window("lab-record", lambda args: ""), {"start": None, "end": None})


class CliSmokeTest(unittest.TestCase):
    def test_ledger_writes_expected_json(self):
        tmp = Path(tempfile.mkdtemp())
        try:
            folder = tmp / "docs" / "analysis" / "fixture-campaign"
            folder.mkdir(parents=True)
            shutil.copy(FIXTURES / "ledger_readme.md", folder / "README.md")
            out = tmp / "out"
            with contextlib.redirect_stdout(io.StringIO()) as printed:
                self.assertEqual(main(["ledger", "--repo", str(tmp), "--out", str(out)]), 0)
            result = json.loads((out / "ledger.json").read_text(encoding="utf-8"))
            packets = result["campaigns"]["fixture-campaign"]["packets"]
            self.assertEqual([(p["id"], p["status"], p["escalated"]) for p in packets],
                             [("FX-01", "Integrated", False), ("FX-02", "Integrated", True), ("FX-03", "Ready", False)])
            self.assertEqual(result["pr_map"], {"1527": "fixture-campaign", "1627": "fixture-campaign"})
            self.assertEqual(result["campaigns"]["fixture-campaign"]["window"], {"start": None, "end": None})
            self.assertEqual(json.loads(printed.getvalue()),
                             {"campaigns": {"fixture-campaign": 3}, "unknown_status": 0, "conflicts": 0})
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
