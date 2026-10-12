"""The campaign cost rollup (CM-03): totals to the cent on a three-campaign fixture, the split and untagged cases, and the CLI."""

import contextlib
import io
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from ingest.prices import CURRENT

from . import cli, cost, registry

SCHEMA = Path(__file__).resolve().parent.parent / "schema.sql"
T0 = "2026-10-01T12:00:00Z"

# Prices in force (ingest/prices.py, CURRENT): Opus 5.5 4/20/0.20, Sonnet 5.5 2/10/0.20, Haiku 5.5 0.10/0.50/0.01 (USD/MTok).
# Session worktrees: s-coder is lr-01, s-rev is lr-02; s-main is the coordinator.
SESSIONS = [("s-main", None, 1), ("s-coder", ".claude/worktrees/lr-01", 0), ("s-rev", ".claude/worktrees/lr-02", 0)]
AGENTS = [("a-coder", "s-coder", "packet-coder"), ("a-rev", "s-rev", "packet-reviewer")]
# pr, head branch, state, campaign (None: the ledger has to tag it)
PRS = [(101, "camp-a/lr-01-x", "MERGED", "camp-a"), (201, "camp-b/lr-02-x", "MERGED", "camp-b"),
       (301, "camp-c/lr-03-x", "MERGED", "camp-c"), (401, "lr/lr-04-x", "MERGED", None)]
# id, session, agent, model, input, output, cache_read, [(pr or None, method, weight)]
REQUESTS = [
    ("r1", "s-main", None, "claude-opus-5-5", 1000, 2000, 10000, [(101, "pr-link", 1.0)]),
    ("r2", "s-coder", "a-coder", "claude-haiku-5-5", 2000, 1000, 50000, [(101, "branch", 1.0)]),
    ("r3", "s-rev", "a-rev", "claude-sonnet-5-5", 3000, 1500, 20000, [(201, "branch", 1.0)]),
    ("r4", "s-main", None, "claude-opus-5-5", 1000, 1000, 0, [(101, "split", 0.5), (201, "split", 0.5)]),
    ("r5", "s-main", None, "claude-opus-5-5", 500, 500, 0, [(None, "unattributed", 1.0)]),
    ("r6", "s-coder", "a-coder", "claude-haiku-5-5", 1000, 1000, 0, [(301, "branch", 1.0)]),
    ("r7", "s-rev", "a-rev", "claude-sonnet-5-5", 1000, 1000, 0, [(401, "branch", 1.0)]),
]


def build(path):
    """A profiler database with the CM-03 fixture, built on schema.sql."""
    db = sqlite3.connect(path)
    db.executescript(SCHEMA.read_text(encoding="utf-8"))
    for session, worktree, coordinator in SESSIONS:
        db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts, is_coordinator, worktree)"
                   " VALUES (?, 'project', ?, ?, ?, ?)", (session, T0, T0, coordinator, worktree))
    for agent, session, custom in AGENTS:
        db.execute("INSERT INTO agents (agent_id, session_id, agent_type, custom_agent_type) VALUES (?, ?, 'general', ?)",
                   (agent, session, custom))
    for pr, branch, state, campaign in PRS:
        db.execute("INSERT INTO prs (pr_number, head_branch, created_at, merged_at, state, campaign)"
                   " VALUES (?, ?, ?, ?, ?, ?)", (pr, branch, T0, T0, state, campaign))
    for rid, session, agent, model, inp, out, read, attribution in REQUESTS:
        db.execute("INSERT INTO triggers (trigger_id, session_id, agent_id, ts, kind, rule)"
                   " VALUES (?, ?, ?, ?, 'human_prompt', 'R1')", ("t-" + rid, session, agent, T0))
        db.execute("INSERT INTO requests (request_id, session_id, agent_id, trigger_id, message_uuid, records_seen,"
                   " ts, model, input_tokens, output_tokens, cache_read, cache_write_5m, cache_write_1h,"
                   " context_tokens) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, 0, 0, ?)",
                   (rid, session, agent, "t-" + rid, "u-" + rid, T0, model, inp, out, read, inp + read))
        for pr, method, weight in attribution:
            db.execute("INSERT INTO pr_attribution (request_id, pr_number, method, weight, confidence, campaign)"
                       " VALUES (?, ?, ?, ?, ?, NULL)", (rid, pr, method, weight, 0.5 if pr else 0.0))
    db.commit()
    db.close()


class CostTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name) / "profile.sqlite"
        build(self.path)
        self.db = sqlite3.connect(f"file:{self.path}?mode=ro", uri=True)
        self.addCleanup(self.db.close)
        config = registry.load_config()
        self.roles = config["roles"]
        self.since, self.until = config["window"]["since"], config["window"]["until"]

    def rollup(self, pr_map=None, accepted=None):
        return cost.rollup(cost.load_requests(self.db, self.since, self.until, pr_map), CURRENT, self.roles, accepted)

    def test_totals_to_the_cent(self):
        # Hand-computed from the price table: r1 0.046, r2 0.0012, r3 0.025, r4 0.024 split, r6 0.0006.
        # Reverting the weight multiplication makes camp-a and camp-b come out at the full request cost.
        out = self.rollup()
        self.assertAlmostEqual(out["camp-a"]["cost"]["total_usd"], 0.046 + 0.0012 + 0.012, places=9)
        self.assertAlmostEqual(out["camp-b"]["cost"]["total_usd"], 0.025 + 0.012, places=9)
        self.assertAlmostEqual(out["camp-c"]["cost"]["total_usd"], 0.0006, places=9)
        self.assertEqual(sorted(out), ["camp-a", "camp-b", "camp-c"])

    def test_split_request_counts_once_across_its_prs(self):
        # r4 is 0.024 in all: 0.012 on PR 101 and 0.012 on PR 201, never 0.024 on each.
        out = self.rollup()
        total = sum(c["cost"]["total_usd"] for c in out.values())
        self.assertAlmostEqual(total, 0.0968, places=9)
        per_pr = {p["pr"]: p["usd"] for p in out["camp-a"]["prs"]["per_pr"]}
        self.assertAlmostEqual(per_pr[101], 0.046 + 0.0012 + 0.012, places=9)

    def test_unattributed_request_is_in_no_campaign_but_in_the_stamp(self):
        # r5 (0.012) has no PR and no campaign: the campaigns above do not hold it, the unattributed share does.
        # Its family is 'main' with 3.0 weight over all main rows (r1, both halves of r4, r5), 1.0 unattributed.
        share = cost.unattributed_share(self.db, self.since, self.until)
        self.assertEqual(share["requests"], 7)
        self.assertAlmostEqual(share["by_request"], round(1 / 7, 4), places=4)
        self.assertAlmostEqual(share["by_usd"], round(0.012 / (0.0968 + 0.012 + 0.012), 4), places=4)
        self.assertAlmostEqual(share["by_family"]["main"]["requests"], round(1 / 3, 4), places=4)

    def test_ledger_map_tags_an_untagged_pr(self):
        # PR 401 has no campaign; without the ledger map its r7 (0.012) is in no campaign.
        self.assertAlmostEqual(self.rollup()["camp-c"]["cost"]["total_usd"], 0.0006, places=9)
        tagged = self.rollup(pr_map={401: "camp-c"})
        self.assertAlmostEqual(tagged["camp-c"]["cost"]["total_usd"], 0.0006 + 0.012, places=9)
        stamp_off = cost.stamp(self.db, self.since, self.until)
        stamp_on = cost.stamp(self.db, self.since, self.until, {401: "camp-c"})
        self.assertEqual(stamp_off["untagged_pr_count"], 1)
        self.assertEqual(stamp_on["untagged_pr_count"], 0)

    def test_tiers_and_roles(self):
        out = self.rollup()
        self.assertAlmostEqual(out["camp-a"]["cost"]["by_model_tier"]["haiku"], 0.0012, places=9)
        self.assertAlmostEqual(out["camp-a"]["cost"]["by_model_tier"]["opus"], 0.058, places=9)
        self.assertAlmostEqual(out["camp-a"]["cost"]["by_role"]["implementer-haiku"], 0.0012, places=9)
        self.assertAlmostEqual(out["camp-b"]["cost"]["by_role"]["reviewer"], 0.025, places=9)
        self.assertAlmostEqual(out["camp-b"]["cost"]["by_role"]["coordinator"], 0.012, places=9)

    def test_accepted_count_or_merged_prs_as_the_denominator(self):
        out = self.rollup(accepted={"camp-a": 2})
        self.assertAlmostEqual(out["camp-a"]["accepted"]["cost_per_accepted_usd"], 0.0592 / 2, places=9)
        fallback = self.rollup()["camp-a"]["accepted"]
        self.assertEqual(fallback["accepted_gap"], "no packet table, used merged PRs")
        self.assertAlmostEqual(fallback["cost_per_accepted_usd"], 0.0592, places=9)

    def test_tier_and_family(self):
        self.assertEqual(cost.tier_of("claude-opus-5-5"), "opus")
        self.assertEqual(cost.tier_of("claude-haiku-5-5-20251001"), "haiku")
        self.assertEqual(cost.tier_of("claude-fable-5-1"), "fable")
        self.assertEqual(cost.tier_of("mystery-model"), "other")
        self.assertEqual(cost.family_of(".claude/worktrees/lr-08a-fix2"), "lr")
        self.assertEqual(cost.family_of(None), "main")


class CliTest(unittest.TestCase):
    def test_cost_smoke_writes_cost_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "profile.sqlite"
            build(db)
            out = Path(tmp) / "out"
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(io.StringIO()):
                status = cli.main(["cost", "--db", str(db), "--out", str(out)])
            self.assertEqual(status, 0)
            self.assertIn("status=ok", buf.getvalue())
            text = (out / "cost.json").read_text(encoding="utf-8")
            self.assertNotIn(str(db), text)
            record = json.loads(text)
            self.assertAlmostEqual(record["campaigns"]["camp-a"]["cost"]["total_usd"], 0.0592, places=9)
            self.assertEqual(record["stamp"]["price_table"], CURRENT)
            self.assertEqual(record["stamp"]["ledger"], "not passed; PRs without a campaign stay untagged")


if __name__ == "__main__":
    unittest.main()
