"""D-TP6 and D-TP7 in the per-PR block: the cost-state comparison and the campaign, appended after the plan's keys."""

import sqlite3

from report import fixture_db

from . import block
from .test_pr_stats import PLAN_KEYS, Fixture


class CostStateFieldTest(Fixture):
    def test_new_keys_come_after_the_plans(self):
        b = block.parse(self.body())
        self.assertEqual(list(b)[:len(PLAN_KEYS)], PLAN_KEYS)
        self.assertEqual(list(b)[len(PLAN_KEYS):], list(block.APPENDED))

    def test_no_per_pr_cost_state_number_is_invented(self):
        cs = block.parse(self.body())["cost_state"]
        self.assertIsNone(cs["usd"])
        self.assertEqual(cs["reason"], block.COST_STATE_REASON)

    def test_gap_is_measured_over_the_prs_sessions(self):
        # The fixture session's cost-state says $12.50, more than its transcribed requests, and covers
        # every request (no process_start).
        body = self.body()
        cs = block.parse(body)["cost_state"]
        self.assertEqual(cs["sessions"], 1)
        self.assertEqual(cs["covered_share"], 1.0)
        self.assertLess(cs["sessions_gap_share"], 0)
        self.assertIn("is not split by PR", body)
        self.assertIn("read the USD as a floor", body)

    def test_sentence_without_coverage(self):
        self.assertIn("No Claude Code cost-state record covers",
                      block.cost_state_sentence({"sessions": 0, "sessions_gap_share": None}))


class CampaignFieldTest(Fixture):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        db = sqlite3.connect(cls.db_path)
        db.execute("UPDATE prs SET campaign = 'camp-a' WHERE pr_number = ?", (fixture_db.PR_MAIN,))
        db.commit()
        db.close()

    def test_campaign_is_in_the_block_and_the_line(self):
        body = self.body()
        self.assertEqual(block.parse(body)["campaign"], "camp-a")
        self.assertIn("Campaign: camp-a.", body)
