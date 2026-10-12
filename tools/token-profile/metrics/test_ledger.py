"""The ledger parser: packet tables, normalization, retry reasons, PR map and windows; fixtures and the real ledgers."""

import contextlib
import io
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from metrics import ledger
from metrics.cli import build_ledger, git_log, main

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
FIXTURES = HERE / "fixtures"


def _fixture_rows():
    return ledger.parse_packet_table((FIXTURES / "ledger_readme.md").read_text(encoding="utf-8"))


def _normalized(status_raw, implementer_raw=""):
    return ledger.normalize({"id": "X-1", "title": "", "status_raw": status_raw, "implementer_raw": implementer_raw,
                             "size": "", "wave": "", "depends": ""})


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

    def test_table_without_id_column(self):
        text = """\
| Packet | Status | PR |
|---|---|---|
| AM-00 Plan | Done (#1400) | |
"""
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertEqual([(r["id"], r["title"], r["status"], r["prs"]) for r in rows],
                         [("AM-00", "Plan", "Done", [1400])])

    def test_id_title_state_table(self):
        text = """\
| ID | Title | Wave | Sev | State |
|---|---|---|---|---|
| TG-NET-05 | Reaper logs each ticket | W3 | med | Integrated (#1573) |
"""
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertEqual([(r["id"], r["title"], r["status"], r["prs"]) for r in rows],
                         [("TG-NET-05", "Reaper logs each ticket", "Integrated", [1573])])

    def test_design_and_decision_tables_skipped(self):
        text = """\
| ID | What we couldn't see | Fix direction | Side | Packet |
|---|---|---|---|---|
| T1 | Ticket reaper | Log it | server | TG-01 |

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-1 | Use the table | Decided | Not a packet. |
"""
        self.assertEqual(ledger.parse_packet_table(text), [])

    def test_escaped_pipe_keeps_status_column(self):
        text = r"""| ID | Packet | Status |
|---|---|---|
| FX-09 | a \| b | Done (#1700) |
"""
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertEqual([(r["title"], r["status"]) for r in rows], [("a | b", "Done")])

    def test_real_token_usage_ledger(self):
        text = (REPO / "docs" / "analysis" / "token-usage" / "README.md").read_text(encoding="utf-8")
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertGreaterEqual(len(rows), 10)
        self.assertNotIn("unknown", [r["status"] for r in rows])

    def test_real_telemetry_gaps_ledger(self):
        text = (REPO / "docs" / "analysis" / "telemetry-gaps" / "README.md").read_text(encoding="utf-8")
        rows = ledger.parse_packet_table(text)
        self.assertGreater(len(rows), 100)

    def test_real_lab_record_ledger(self):
        text = (REPO / "docs" / "analysis" / "lab-record" / "README.md").read_text(encoding="utf-8")
        rows = [ledger.normalize(r) for r in ledger.parse_packet_table(text)]
        self.assertGreaterEqual(len(rows), 10)
        lr02 = next(r for r in rows if r["id"] == "LR-02")
        self.assertTrue(lr02["escalated"])
        self.assertEqual(lr02["status"], "Integrated")


class StatusTest(unittest.TestCase):
    def test_status_word_and_aliases(self):
        self.assertEqual(_normalized("merged, #1321")["status"], "Integrated")
        self.assertEqual(_normalized("merged, #1321")["prs"], [1321])
        self.assertEqual(_normalized("**Done**")["status"], "Done")
        self.assertEqual(_normalized("passed 2026-10-10")["status"], "Done")
        self.assertEqual(_normalized("Dropped: superseded")["status"], "unknown")


class EscalationTest(unittest.TestCase):
    def test_named_implementer_with_phrase_is_escalated(self):
        self.assertTrue(_normalized("Ready", "packet-coder (finished by rust-gameserver-dev)")["escalated"])

    def test_coordinator_naming_haiku_is_not_escalated(self):
        self.assertFalse(_normalized("Ready", "coordinator + Haiku lab-driver")["escalated"])

    def test_worknote_head_reason(self):
        head = (FIXTURES / "ledger_worknote.md").read_text(encoding="utf-8").splitlines()[:60]
        self.assertEqual(ledger.retry_reason("\n".join(head)), "context-exhausted")
        waiting = "Waiting for the notification from a background Bash"
        self.assertEqual(ledger.retry_reason(waiting), "stalled-background")
        self.assertEqual(ledger.retry_reason("clippy failed"), "ci-failure")
        self.assertEqual(ledger.retry_reason(""), "other")


class MapTest(unittest.TestCase):
    def test_pr_map_and_conflicts(self):
        a = [{"status": "Done", "prs": [1527, 1627]}]
        b = [{"status": "Integrated", "prs": [1627, 1634]}]
        pr_map, conflicts = ledger.campaign_pr_map({"lab-record": a, "lab-chaos": b})
        self.assertEqual(pr_map, {1527: "lab-record", 1634: "lab-chaos"})
        self.assertEqual(conflicts, [{"pr": 1627, "campaigns": ["lab-chaos", "lab-record"]}])

    def test_packet_not_yet_done_cites_no_pr(self):
        rows = [_normalized("Ready (coordinate with #1303)")]
        self.assertEqual(ledger.campaign_pr_map({"lab-x": rows}), ({}, []))

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


class BuildTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.root = self.tmp / "docs" / "analysis"

    def _write(self, campaign, name, text):
        folder = self.root / campaign
        folder.mkdir(parents=True, exist_ok=True)
        (folder / name).write_text(text, encoding="utf-8")
        return folder

    def test_duplicate_ids_keep_the_row_with_a_status(self):
        self._write("fx", "README.md", "| ID | Packet | Status |\n|---|---|---|\n"
                    "| FX-05 | Thing | |\n| FX-05 | Thing | Done (#1700) |\n")
        result = build_ledger(self.tmp, lambda args: "")
        self.assertEqual([(p["id"], p["status"], p["prs"]) for p in result["campaigns"]["fx"]["packets"]],
                         [("FX-05", "Done", [1700])])

    def test_pr_override_maps_a_campaign_without_a_table(self):
        (self.root / "fx").mkdir(parents=True)
        result = build_ledger(self.tmp, lambda args: "", {"fx": [831]})
        self.assertEqual(result["pr_map"], {831: "fx"})

    def test_worknote_gives_retry_reason(self):
        folder = self._write("fx", "README.md", (FIXTURES / "ledger_readme.md").read_text(encoding="utf-8"))
        (folder / "worknotes").mkdir()
        shutil.copy(FIXTURES / "ledger_worknote.md", folder / "worknotes" / "FX-02.md")
        out = self.tmp / "out"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(main(["ledger", "--repo", str(self.tmp), "--out", str(out)]), 0)
        result = json.loads((out / "ledger.json").read_text(encoding="utf-8"))
        packets = {p["id"]: p for p in result["campaigns"]["fx"]["packets"]}
        self.assertEqual(packets["FX-02"]["retry_reason"], "context-exhausted")
        self.assertIsNone(packets["FX-01"]["retry_reason"])

    def test_failed_git_run_is_recorded(self):
        failures = []
        missing = Path(tempfile.gettempdir()) / "cm05-no-such-repo"
        self.assertEqual(git_log(missing, failures)(["log"]), "")
        self.assertEqual(len(failures), 1)


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
            # The shipped config's pr_overrides adds one pseudo-packet (status unknown) for historical-cellblocks.
            self.assertEqual(result["pr_map"], {"1527": "fixture-campaign", "1627": "fixture-campaign",
                                                "831": "historical-cellblocks"})
            self.assertEqual(result["campaigns"]["fixture-campaign"]["window"], {"start": None, "end": None})
            self.assertEqual(json.loads(printed.getvalue()),
                             {"campaigns": {"fixture-campaign": 3, "historical-cellblocks": 1},
                              "unknown_status": 1, "conflicts": 0})
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
