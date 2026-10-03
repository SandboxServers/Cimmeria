"""The cut-line measures over synthetic transcripts on either side of a cut."""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from ingest.test_support import build_fixtures as bf
from ingest.test_support import run_ingest, transcript, write_session
from report import db as report_db
from report.scrub import Scrubber

from . import cli, measures

CUT = bf.ts(30)
CUTS = {"TP-00": CUT, "TP-02": CUT, "TP-03": CUT}


def lane_call(t, when, rid, tool_id, chars, background=False):
    cmd = {"command": "bash tools/build-lane/lane.sh cargo check -p x", "run_in_background": background}
    bf.request(t, when, rid, bf.usage(1, 10, 0, 1000, 0, 0),
               content=[{"type": "tool_use", "id": tool_id, "name": "Bash", "input": cmd}])
    rec = t.tool_result(when, tool_id, "x" * chars)
    if background:
        rec["toolUseResult"] = {"backgroundTaskId": "b" + tool_id}


class CutlinesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        root = Path(cls.tmp.name)
        for n, (start, first_ctx, lane_chars, sub_requests) in enumerate(
                [(bf.ts(1), 80000, 30000, 250), (bf.ts(40), 60000, 300, 40)]):
            sid = f"8888888{n}-0000-4000-8000-000000000008"
            m = transcript(sid)
            m.user_text(start, "go", origin={"kind": "human"})
            bf.request(m, start, f"{sid}-first", bf.usage(10, 10, 0, first_ctx - 10, 0, 0))
            lane_call(m, start, f"{sid}-lane", f"tu{n}", lane_chars)
            lane_call(m, start, f"{sid}-lane-bg", f"tb{n}", 120, background=True)
            sub = transcript(sid, agent_id=f"w{n}")
            sub.user_text(start, "task", isMeta=True)
            for i in range(sub_requests):
                bf.request(sub, start, f"{sid}-s{i}", bf.usage(1, 1, 0, 50000 + i, 0, 0))
            write_session(root / "p", sid, m, [(f"w{n}", sub, {"agentType": "tm-name", "name": "tm-name",
                                                                "taskKind": "in_process_teammate"})])
        cls.db_path = root / "d.sqlite"
        assert run_ingest(root / "p", cls.db_path, prs=[]) == 0
        db = report_db.open_db(cls.db_path)
        cls.r = measures.build(db, Scrubber(use_local=False), bf.ts(0), CUTS)
        db.close()

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_static_context_splits_at_the_cut(self):
        main = self.r["static_context"]["by_scope"]["main"]
        self.assertEqual((main["before"]["n"], main["before"]["max"], main["after"]["max"]), (1, 80000, 60000))
        self.assertIn("teammate", self.r["static_context"]["by_agent_type"])
        self.assertNotIn("tm-name", json.dumps(self.r))

    def test_lane_output_separates_background_calls(self):
        lane = self.r["lane_output"]
        self.assertEqual((lane["foreground"]["before"]["max"], lane["foreground"]["after"]["max"]), (30000, 300))
        self.assertEqual(lane["background"]["after"]["n"], 1)

    def test_worker_lifetime(self):
        w = self.r["worker_lifetime"]
        self.assertEqual((w["before"]["requests"]["max"], w["after"]["requests"]["max"]), (250, 40))
        self.assertEqual((w["before"]["over_200_requests"], w["after"]["over_200_requests"]), (1, 0))

    def test_cli_writes_through_the_gate(self):
        out = Path(self.tmp.name) / "out"
        argv = ["--db", str(self.db_path), "--out", str(out), "--since", bf.ts(0)]
        argv += [a for k, v in CUTS.items() for a in ("--cut", f"{k}={v}")]
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(cli.main(argv), 0)
        self.assertIn("## Worker lifetime", (out / "cutlines.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
