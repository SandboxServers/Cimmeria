"""The campaign rollup (D-TP7) and the cost-state total next to the profiler's (D-TP6), on the synthetic fixture."""

import json
import sqlite3
import tempfile
import unittest

from . import db as dbmod
from . import fixture_db, generate, render
from .scrub import Scrubber
from .sections import campaigns

CAMPAIGN = "camp-a"


class CampaignRollupTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.db_path, _ = fixture_db.build(self.tmp.name)
        db = sqlite3.connect(self.db_path)
        db.execute("UPDATE prs SET campaign = ?", (CAMPAIGN,))
        # req_J's unattributed share is packet work that reached the campaign without a PR.
        db.execute("UPDATE pr_attribution SET method = 'campaign', campaign = ?, confidence = 0.8"
                   " WHERE request_id = 'req_J' AND pr_number IS NULL", (CAMPAIGN,))
        db.commit()
        db.close()

    def tearDown(self):
        self.tmp.cleanup()

    def spend(self, db, where, *args):
        return db.execute("SELECT SUM(a.weight * c.usd) FROM pr_attribution a JOIN rcost c USING (request_id)"
                          f" WHERE {where}", args).fetchone()[0]

    def test_campaign_total_is_its_prs_plus_packets_without_a_pr(self):
        db = dbmod.open_db(self.db_path)
        try:
            dbmod.scope(db)
            got = campaigns.build(db, Scrubber(use_local=False))
            prs = self.spend(db, "a.pr_number IN (?, ?)", fixture_db.PR_MAIN, fixture_db.PR_OTHER)
            packets = self.spend(db, "a.method = 'campaign'")
        finally:
            db.close()
        (row,) = got["campaigns"]
        self.assertEqual(row["campaign"], CAMPAIGN)
        self.assertEqual((row["prs"], row["merged_prs"]), (2, 2))
        self.assertGreater(packets, 0)
        self.assertAlmostEqual(row["pr_usd_est"], prs, places=4)
        self.assertAlmostEqual(row["no_pr_usd_est"], packets, places=4)
        self.assertAlmostEqual(row["usd_est"], prs + packets, places=4)

    def test_campaign_rows_are_not_counted_as_unattributed(self):
        db = dbmod.open_db(self.db_path)
        try:
            dbmod.scope(db)
            packets = self.spend(db, "a.method = 'campaign'")
            unattributed = self.spend(db, "a.method = 'unattributed'")
        finally:
            db.close()
        _, js = generate.generate(self.db_path, use_local_deny=False)
        report = json.loads(js)
        spend = report["prs"]["window_spend"]
        self.assertAlmostEqual(spend["campaign"]["usd"], packets, places=6)
        self.assertAlmostEqual(spend["unattributed"]["usd"], unattributed, places=6)
        # req_J shares a session with PR 4243's requests. As a campaign row it is attributed spend, so the
        # PR's unattributed share falls below the unmodified fixture's.
        with tempfile.TemporaryDirectory() as tmp:
            plain_db, _ = fixture_db.build(tmp)
            _, plain_js = generate.generate(plain_db, use_local_deny=False)
        share = {r["pr"]: r["attribution"]["unattributed_share"] for r in report["prs"]["top"]}
        plain = {r["pr"]: r["attribution"]["unattributed_share"] for r in json.loads(plain_js)["prs"]["top"]}
        self.assertLess(share[fixture_db.PR_OTHER], plain[fixture_db.PR_OTHER])

    def test_report_shows_the_cost_state_total_next_to_the_profilers(self):
        md, _ = generate.generate(self.db_path, use_local_deny=False)
        self.assertIn("Claude Code's own cost-state total: **$12.50** over 1 sessions", md)
        self.assertIn("## Cost per campaign", md)

    def test_no_cost_state_is_said_not_invented(self):
        self.assertIn("no cost-state record covers this window", render.cost_state_line({"sessions": 0}))
        self.assertIn("no cost-state record covers this window", render.cost_state_line(None))


if __name__ == "__main__":
    unittest.main()
