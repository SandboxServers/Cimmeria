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
# CM-03 review pins, kept apart from REQUESTS so the totals above hold. Campaign camp-d has PRs 501 (OPEN) and 502
# (MERGED); camp-e has rows with no PR. The schema allows a campaign on a row only when it has no PR, so a row
# campaign and a PR campaign never meet in one row (see test_row_campaign_beats_pr_campaign).
# pr, head branch, state, campaign
PINNED_PRS = [(501, "camp-d/lr-05-x", "OPEN", "camp-d"), (502, "camp-d/lr-05-y", "MERGED", "camp-d")]
# id, session, agent, model, input, output, cache_read, cache_write_5m, cache_write_1h, [(pr or None, method, weight, campaign)]
PINNED_REQUESTS = [
    # Sonnet 5.5 cache writes: (1000 * 2.0 * 1.25 + 1000 * 2.0 * 2.0) / 1e6 = 0.0065 on PR 502.
    ("p1", "s-main", None, "claude-sonnet-5-5", 0, 0, 0, 1000, 1000, [(502, "branch", 1.0, None)]),
    # Opus 5.5: (1000 * 4 + 1000 * 20) / 1e6 = 0.024 on the OPEN PR 501.
    ("p2", "s-coder", "a-coder", "claude-opus-5-5", 1000, 1000, 0, 0, 0, [(501, "branch", 1.0, None)]),
    # No price for this model: unpriced, on the MERGED PR 502.
    ("p3", "s-coder", "a-coder", "mystery-model", 1000, 1000, 0, 0, 0, [(502, "branch", 1.0, None)]),
    # Haiku 5.5 prompt is 50000 + 60000 = 110000 > 100000, so the x5 long card applies:
    # (50000 * 0.10 + 1000 * 0.50 + 60000 * 0.01) / 1e6 = 0.0061, times 5 = 0.0305, on camp-e with no PR.
    ("p4", "s-rev", "a-rev", "claude-haiku-5-5", 50000, 1000, 60000, 0, 0, [(None, "campaign", 1.0, "camp-e")]),
    # Sonnet 5.5 input: 1000 * 2 / 1e6 = 0.002, on camp-e with no PR.
    ("p5", "s-main", None, "claude-sonnet-5-5", 1000, 0, 0, 0, 0, [(None, "campaign", 1.0, "camp-e")]),
]


def build(path, pinned=False):
    """A profiler database with the CM-03 fixture, built on schema.sql; `pinned` adds the review rows."""
    db = sqlite3.connect(path)
    db.executescript(SCHEMA.read_text(encoding="utf-8"))
    for session, worktree, coordinator in SESSIONS:
        db.execute("INSERT INTO sessions (session_id, project_dir, first_ts, last_ts, is_coordinator, worktree)"
                   " VALUES (?, 'project', ?, ?, ?, ?)", (session, T0, T0, coordinator, worktree))
    for agent, session, custom in AGENTS:
        db.execute("INSERT INTO agents (agent_id, session_id, agent_type, custom_agent_type) VALUES (?, ?, 'general', ?)",
                   (agent, session, custom))
    for pr, branch, state, campaign in PRS + (PINNED_PRS if pinned else []):
        db.execute("INSERT INTO prs (pr_number, head_branch, created_at, merged_at, state, campaign)"
                   " VALUES (?, ?, ?, ?, ?, ?)", (pr, branch, T0, T0, state, campaign))
    for rid, session, agent, model, inp, out, read, attribution in REQUESTS:
        add_request(db, rid, session, agent, model, inp, out, read, 0, 0, [(*item, None) for item in attribution])
    for row in PINNED_REQUESTS if pinned else []:
        add_request(db, *row)
    db.commit()
    db.close()


def add_request(db, rid, session, agent, model, inp, out, read, cache_5m, cache_1h, attribution):
    """One request, its trigger, and one pr_attribution row per (pr or campaign or unattributed) item."""
    db.execute("INSERT INTO triggers (trigger_id, session_id, agent_id, ts, kind, rule)"
               " VALUES (?, ?, ?, ?, 'human_prompt', 'R1')", ("t-" + rid, session, agent, T0))
    db.execute("INSERT INTO requests (request_id, session_id, agent_id, trigger_id, message_uuid, records_seen,"
               " ts, model, input_tokens, output_tokens, cache_read, cache_write_5m, cache_write_1h,"
               " context_tokens) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?)",
               (rid, session, agent, "t-" + rid, "u-" + rid, T0, model, inp, out, read, cache_5m, cache_1h,
                inp + read + cache_5m + cache_1h))
    for pr, method, weight, campaign in attribution:
        db.execute("INSERT INTO pr_attribution (request_id, pr_number, method, weight, confidence, campaign)"
                   " VALUES (?, ?, ?, ?, ?, ?)", (rid, pr, method, weight, 0.5 if pr else 0.0, campaign))


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

    def test_base_roles_are_zero_filled(self):
        # camp-c has no fixer rows, but every base role is present in by_role, at 0.0 when it has no spend.
        by_role = self.rollup()["camp-c"]["cost"]["by_role"]
        self.assertLessEqual(set(cost.BASE_ROLES), set(by_role))
        self.assertEqual(by_role["fixer"], 0.0)

    def test_accepted_count_or_merged_prs_as_the_denominator(self):
        out = self.rollup(accepted={"camp-a": 2})
        self.assertAlmostEqual(out["camp-a"]["accepted"]["cost_per_accepted_usd"], 0.0592 / 2, places=9)
        fallback = self.rollup()["camp-a"]["accepted"]
        self.assertEqual(fallback["accepted_gap"], "no packet table, used merged PRs")
        self.assertAlmostEqual(fallback["cost_per_accepted_usd"], 0.0592, places=9)

    def test_window_bounds_take_the_stored_millisecond_form(self):
        # Stored ts carry .000Z; the config's bounds end in Z. Since is inside, until is outside.
        writer = sqlite3.connect(self.path)
        writer.execute("UPDATE requests SET ts = ? WHERE request_id = 'r1'", (self.since[:-1] + ".000Z",))
        writer.execute("UPDATE requests SET ts = ? WHERE request_id = 'r2'", (self.until[:-1] + ".000Z",))
        writer.commit()
        writer.close()
        ids = {row["request_id"] for row in cost.load_requests(self.db, self.since, self.until)}
        self.assertIn("r1", ids)
        self.assertNotIn("r2", ids)

    def test_tier_and_family(self):
        self.assertEqual(cost.tier_of("claude-opus-5-5"), "opus")
        self.assertEqual(cost.tier_of("claude-haiku-5-5-20251001"), "haiku")
        self.assertEqual(cost.tier_of("claude-fable-5-1"), "fable")
        self.assertEqual(cost.tier_of("mystery-model"), "other")
        self.assertEqual(cost.family_of(".claude/worktrees/lr-08a-fix2"), "lr")
        self.assertEqual(cost.family_of(None), "main")


class PinnedCostTest(unittest.TestCase):
    """CM-03 review pins: cache writes, the long-prompt card, merged-only cost per PR, unpriced rows, merged flags."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name) / "profile.sqlite"
        build(self.path, pinned=True)
        self.db = sqlite3.connect(f"file:{self.path}?mode=ro", uri=True)
        self.addCleanup(self.db.close)
        config = registry.load_config()
        self.roles = config["roles"]
        self.since, self.until = config["window"]["since"], config["window"]["until"]

    def rollup(self):
        return cost.rollup(cost.load_requests(self.db, self.since, self.until), CURRENT, self.roles)

    def test_cache_writes_are_loaded_and_priced(self):
        # p1 is 0.0065 on PR 502: zeroing either cache write in load_requests gives 0.004 or 0.0025.
        rows = {row["request_id"]: row for row in cost.load_requests(self.db, self.since, self.until)}
        self.assertEqual(rows["p1"]["tokens"]["cache_write_5m"], 1000)
        self.assertEqual(rows["p1"]["tokens"]["cache_write_1h"], 1000)
        per_pr = {p["pr"]: p for p in self.rollup()["camp-d"]["prs"]["per_pr"]}
        self.assertAlmostEqual(per_pr[502]["usd"], 0.0065, places=9)

    def test_long_prompt_card_applies_above_100k(self):
        # p4 (Haiku, the x5 card) is 0.0305 and p5 (Sonnet) is 0.002: camp-e is 0.0325 and the tier split is Haiku's.
        out = self.rollup()
        self.assertAlmostEqual(out["camp-e"]["cost"]["by_model_tier"]["haiku"], 0.0305, places=9)
        self.assertAlmostEqual(out["camp-e"]["cost"]["total_usd"], 0.0325, places=9)

    def test_campaign_row_without_a_pr_is_unattributed_in_campaign(self):
        # camp-e has no PR: its 0.0325 is unattributed_in_campaign, nothing is attributed to a PR.
        camp = self.rollup()["camp-e"]
        self.assertAlmostEqual(camp["cost"]["unattributed_in_campaign_usd"], 0.0325, places=9)
        self.assertAlmostEqual(camp["cost"]["attributed_usd"], 0.0, places=9)
        self.assertEqual(camp["prs"]["count"], 0)

    def test_cost_per_pr_is_the_merged_pr_only(self):
        # PR 501 is OPEN at 0.024; PR 502 is MERGED at 0.0065. The mean over merged PRs is 0.0065, not 0.01525.
        camp = self.rollup()["camp-d"]
        self.assertEqual(camp["prs"]["count"], 2)
        self.assertEqual(camp["prs"]["merged"], 1)
        self.assertAlmostEqual(camp["prs"]["cost_per_pr"]["mean"], 0.0065, places=9)
        self.assertEqual(camp["prs"]["per_pr"], [{"pr": 501, "usd": 0.024, "merged": False},
                                                 {"pr": 502, "usd": 0.0065, "merged": True}])

    def test_unpriced_row_is_counted_and_left_out(self):
        # p3 (mystery-model) is counted once in unpriced_rows and is in no total: camp-d is 0.0065 + 0.024.
        camp = self.rollup()["camp-d"]
        self.assertEqual(camp["cost"]["unpriced_rows"], 1)
        self.assertAlmostEqual(camp["cost"]["total_usd"], 0.0305, places=9)

    def test_campaign_without_a_merged_pr_says_so(self):
        # camp-e has no PR, so no merged PR and no cost per PR; camp-d has merged PR 502, so it has no gap.
        out = self.rollup()
        self.assertEqual(out["camp-e"]["prs"]["cost_per_pr_gap"], "no merged PRs")
        self.assertIsNone(out["camp-e"]["prs"]["cost_per_pr"]["mean"])
        self.assertNotIn("cost_per_pr_gap", out["camp-d"]["prs"])

    def test_pr_merged_follows_the_pr_state(self):
        # p1 is on the MERGED PR 502 and p2 on the OPEN PR 501: pr_merged is not always true.
        rows = {row["request_id"]: row for row in cost.load_requests(self.db, self.since, self.until)}
        self.assertIs(rows["p1"]["pr_merged"], True)
        self.assertIs(rows["p2"]["pr_merged"], False)

    def test_row_campaign_beats_pr_campaign(self):
        # campaign_of's order: the row's own campaign, then its PR's, then the ledger map. The order is not observable
        # through the DB: the schema CHECKs make row_campaign and pr_campaign mutually exclusive (a campaign row has no
        # PR), so this pins the function on synthetic rows only.
        row = {"row_campaign": "camp-x", "pr_campaign": "camp-y", "pr_number": 5}
        self.assertEqual(cost.campaign_of(row, {5: "camp-z"}), "camp-x")
        self.assertEqual(cost.campaign_of({**row, "row_campaign": None}, {5: "camp-z"}), "camp-y")
        self.assertEqual(cost.campaign_of({**row, "row_campaign": None, "pr_campaign": None}, {5: "camp-z"}), "camp-z")


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
            self.assertEqual(record["stamp"]["profiler_schema"], "6")
            self.assertEqual(record["stamp"]["ledger"], "not passed; PRs without a campaign stay untagged")

    def test_ledger_in_the_real_shape_counts_accepted_packets(self):
        # Override rows are not packets: camp-a's override row is Done but is not counted.
        ledger = {"schema": 1, "pr_map": {"401": "camp-c"}, "conflicts": [],
                  "campaigns": {
                      "camp-a": {"window": {}, "packets": [
                          {"status": "Done"}, {"status": "Integrated"}, {"status": "Ready"}, {"status": "unknown"},
                          {"override": True, "status": "Done"}]},
                      "camp-b": {"window": {}, "packets": [{"status": "UATPending"}, {"status": "Ready"}]}}}
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "profile.sqlite"
            build(db)
            ledger_file = Path(tmp) / "ledger.json"
            ledger_file.write_text(json.dumps(ledger), encoding="utf-8")
            out = Path(tmp) / "out"
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                status = cli.main(["cost", "--db", str(db), "--out", str(out), "--ledger", str(ledger_file)])
            self.assertEqual(status, 0)
            record = json.loads((out / "cost.json").read_text(encoding="utf-8"))
            self.assertEqual(record["campaigns"]["camp-a"]["accepted"]["packets"], 2)
            self.assertEqual(record["campaigns"]["camp-b"]["accepted"]["packets"], 1)
            self.assertNotIn("accepted_gap", record["campaigns"]["camp-a"]["accepted"])

    def test_ledger_with_no_accepted_packets_says_so(self):
        # camp-a's only packet is Ready, which is not accepted: a count of 0, with no cost per packet.
        ledger = {"schema": 1, "pr_map": {}, "conflicts": [], "campaigns": {
            "camp-a": {"window": {}, "packets": [{"status": "Ready"}]}}}
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "profile.sqlite"
            build(db)
            ledger_file = Path(tmp) / "ledger.json"
            ledger_file.write_text(json.dumps(ledger), encoding="utf-8")
            out = Path(tmp) / "out"
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                status = cli.main(["cost", "--db", str(db), "--out", str(out), "--ledger", str(ledger_file)])
            self.assertEqual(status, 0)
            record = json.loads((out / "cost.json").read_text(encoding="utf-8"))
            self.assertEqual(record["campaigns"]["camp-a"]["accepted"],
                             {"packets": 0, "cost_per_accepted_usd": None,
                              "accepted_gap": "the ledger counts no accepted packets"})

    def test_malformed_ledger_exits_2_with_a_status_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "profile.sqlite"
            build(db)
            ledger_file = Path(tmp) / "ledger.json"
            # A campaign that is a list, not an object: indexing it by "packets" is a TypeError.
            ledger_file.write_text(json.dumps({"pr_map": {}, "campaigns": {"camp-a": []}}), encoding="utf-8")
            err = io.StringIO()
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
                status = cli.main(["cost", "--db", str(db), "--out", str(Path(tmp) / "out"),
                                   "--ledger", str(ledger_file)])
            self.assertEqual(status, 2)
            self.assertIn("status=error TypeError", err.getvalue().splitlines())

    def test_unreadable_database_exits_2_with_a_status_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "garbage.sqlite"
            db.write_bytes(b"this is not a sqlite database" * 100)
            err = io.StringIO()
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
                status = cli.main(["cost", "--db", str(db), "--out", str(Path(tmp) / "out")])
            self.assertEqual(status, 2)
            self.assertIn("status=error DatabaseError", err.getvalue().splitlines())


if __name__ == "__main__":
    unittest.main()
