"""The report layers over the synthetic fixture, loaded into schema.sql by fixture_db."""

import json
import tempfile
import unittest

from . import db as dbmod
from . import fixture_db, generate
from .scrub import Scrubber
from .sections import cache_sim

P = fixture_db.PRICES["claude-opus-5-5"]


def usd(w):
    return (w["input_tokens"] * P["input"] + w["output_tokens"] * P["output"] + w["cache_read"] * P["cache_read"]
            + w["cache_write_5m"] * P["cache_write_5m"] + w["cache_write_1h"] * P["cache_write_1h"]) / 1e6


def walk(obj, path=""):
    if isinstance(obj, dict):
        for k, v in obj.items():
            yield path + "/" + str(k), k, v
            yield from walk(v, path + "/" + str(k))
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            yield from walk(v, f"{path}[{i}]")


class ReportTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.db_path, cls.expected = fixture_db.build(cls.tmp.name)
        cls.md, js = generate.generate(cls.db_path, use_local_deny=False)
        cls.report = json.loads(js)
        cls.want = cls.expected["requests"]

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_raw_token_categories_are_reported_independently(self):
        totals = self.report["tokens"]["totals"]
        for column in ("input_tokens", "output_tokens", "thinking_tokens", "cache_read", "cache_write_5m",
                       "cache_write_1h"):
            self.assertEqual(totals[column], sum(w[column] for w in self.want.values()), column)
        self.assertEqual(totals["requests"], len(self.want))
        dims = {d: rows for d, rows in self.report["tokens"]["by"].items()}
        self.assertEqual(set(dims), {"model", "scope", "agent_type", "trigger"})
        kinds = {r["key"] for r in dims["trigger"]}
        self.assertIn("unknown", kinds)
        self.assertIn("mixed", kinds)

    def test_usd_does_not_add_thinking_on_top_of_output(self):
        cost = self.report["cost"]["totals"]
        self.assertAlmostEqual(cost["usd"], sum(usd(w) for w in self.want.values()))
        self.assertAlmostEqual(cost["usd_output"], sum(w["output_tokens"] for w in self.want.values()) * P["output"] / 1e6)
        self.assertTrue(any(w["thinking_tokens"] for w in self.want.values()))

    def test_usd_is_labelled_as_a_plan_usage_proxy(self):
        self.assertIn("plan-usage proxy, not a bill", self.report["cost"]["label"])
        self.assertIn("plan-usage proxy, not a bill", self.report["prs"]["label"])
        self.assertIn("## Estimated USD\n\n**Estimated list-price USD", self.md)

    def test_main_subagent_and_agent_type_are_split(self):
        by = {r["key"]: r for r in self.report["cost"]["by"]["agent_type"]}
        sub = [rid for rid, w in self.want.items() if w["agent_id"]]
        self.assertAlmostEqual(by["rust-gameserver-dev"]["usd"], sum(usd(self.want[r]) for r in sub))
        self.assertEqual(by["main (coordinator)"]["requests"], len(self.want) - len(sub))

    def test_context_and_exposure_carry_no_money(self):
        for section in ("context", "tools", "tokens"):
            for path, key, _ in walk(self.report[section]):
                self.assertNotIn("usd", str(key).lower(), f"{section}{path}")
        for path, key, _ in walk(self.report["cost"]):
            self.assertNotIn("exposure", str(key), path)

    def test_every_distribution_has_the_full_tail(self):
        found = 0
        for path, _, value in walk(self.report):
            # The per-PR block's context {peak, p50} is the cimmeria-pr-stats/1 shape from the plan.
            if path.startswith("/prs/top") and path.endswith("/context"):
                continue
            if isinstance(value, dict) and ("mean" in value or "p50" in value):
                found += 1
                self.assertTrue({"p50", "p75", "p90", "p95", "p99", "max"} <= set(value), path)
        self.assertGreater(found, 20)

    def test_compactions_are_reported(self):
        c = self.report["context"]["compactions"]
        self.assertEqual(c["count"], self.expected["compactions"])
        self.assertEqual(c["by_trigger"], {"auto": 1})
        self.assertEqual(c["dropped_tokens"]["max"], 56200)

    def test_fingerprints_are_grouped_and_shown(self):
        fps = {(r["tool"], r["fingerprint"]) for r in self.report["tools"]["by_fingerprint"]}
        for tid, fp in self.expected["tool_fingerprints"].items():
            if fp:
                self.assertTrue(any(f == fp for _, f in fps), fp)
        self.assertEqual(self.report["stamp"]["rejected"], {})

    def test_cost_per_merged_pr(self):
        prs = self.report["prs"]
        self.assertEqual(prs["merged_prs"], 2)
        by_pr = {r["pr"]: r for r in prs["top"]}
        want = {}
        for rid, rows in fixture_db.ATTRIBUTION.items():
            for pr, _, weight, _ in rows:
                want[pr] = want.get(pr, 0.0) + weight * usd(self.want[rid])
        self.assertAlmostEqual(by_pr[fixture_db.PR_MAIN]["usd_est"], want[fixture_db.PR_MAIN], places=4)
        self.assertAlmostEqual(by_pr[fixture_db.PR_OTHER]["usd_est"], want[fixture_db.PR_OTHER], places=4)
        self.assertEqual(by_pr[fixture_db.PR_MAIN]["attribution"]["method"], "branch")
        self.assertEqual(by_pr[fixture_db.PR_OTHER]["attribution"]["method"], "split")
        self.assertEqual(by_pr[fixture_db.PR_MAIN]["agents"]["rust-gameserver-dev"]["count"], 1)
        self.assertEqual(by_pr[fixture_db.PR_MAIN]["tools"]["read_chars"], 20000)

    def test_unattributed_is_reported_not_forced(self):
        spend = self.report["prs"]["window_spend"]
        unattributed = sum(usd(self.want[r]) for r in self.want if r not in fixture_db.ATTRIBUTION)
        unattributed += 0.75 * usd(self.want["req_J"])
        self.assertAlmostEqual(spend["unattributed"]["usd"], unattributed)
        self.assertAlmostEqual(sum(spend[k]["share"] for k in ("merged_in_window", "other_prs", "unattributed")), 1)
        for r in self.report["prs"]["top"]:
            self.assertGreaterEqual(r["attribution"]["unattributed_share"], 0)
            self.assertLessEqual(r["attribution"]["unattributed_share"], 1)

    def test_every_report_is_version_stamped(self):
        s = self.report["stamp"]
        self.assertEqual(s["profiler_commit"], fixture_db.PROFILER_COMMIT[:12])
        self.assertEqual(s["price_table"], fixture_db.PRICE_VERSION)
        self.assertEqual(s["claude_code"]["all"], ["2.1.999"])
        self.assertEqual(s["window"]["requests"], len(self.want))
        self.assertEqual(s["unknown_shapes"], {"shapes": 1, "records": 1})
        for label in ("Profiler commit", "Report commit", "Price table", "Claude Code", "Window", "Unknown records"):
            self.assertIn(f"| {label} |", self.md)

    def test_window_limits_requests(self):
        _, js = generate.generate(self.db_path, until="2026-10-01T12:10:00.000Z", use_local_deny=False)
        r = json.loads(js)
        self.assertEqual(r["tokens"]["totals"]["requests"], 4)  # req_A, req_B, req_S1, req_S2
        self.assertEqual(r["stamp"]["window"]["until"], "2026-10-01T12:10:00.000Z")
        self.assertEqual(r["context"]["compactions"]["count"], 0)

    def test_unknown_price_table_leaves_requests_unpriced(self):
        _, js = generate.generate(self.db_path, price_table="no-such-table", use_local_deny=False)
        cost = json.loads(js)["cost"]
        self.assertEqual(cost["totals"]["unpriced_requests"], len(self.want))
        self.assertEqual(cost["unpriced_models"], ["claude-opus-5-5"])

    def test_schema_version_is_checked(self):
        db = dbmod.open_db(self.db_path)
        db.execute("CREATE TEMP TABLE meta (key TEXT, value TEXT)")  # shadows main.meta
        db.execute("INSERT INTO temp.meta VALUES ('schema_version', '99')")
        with self.assertRaises(dbmod.ReportError):
            dbmod.scope(db)
        db.close()


class CacheSimTest(unittest.TestCase):
    PRICE = {"cache_read": 1.0, "cache_write_5m": 2.0, "cache_write_1h": 3.0}

    def req(self, read, w5, gap, after_compaction=False):
        return {"cache_read": read, "cache_write_5m": w5, "cache_write_1h": 0, "prev_gap_s": gap,
                "after_compaction": after_compaction}

    def test_replay_over_idle_gaps(self):
        run = [self.req(0, 100, None), self.req(100, 50, 60), self.req(0, 150, 600)]
        out = cache_sim.replay(run, self.PRICE, floor=0)
        self.assertAlmostEqual(out["observed"] * 1e6, 200 + 200 + 300)
        self.assertAlmostEqual(out["5m"] * 1e6, 200 + 200 + 300)  # the 10-minute gap expired the 5m cache
        self.assertAlmostEqual(out["1h"] * 1e6, 300 + 250 + 150)  # 1h writes cost more but the gap stays warm

    def test_compaction_is_cold_under_both(self):
        run = [self.req(0, 100, None), self.req(0, 100, 10, after_compaction=True)]
        out = cache_sim.replay(run, self.PRICE, floor=0)
        self.assertAlmostEqual(out["1h"] * 1e6, 300 + 300)

    def test_floor_is_read_when_cold(self):
        run = [self.req(40, 60, None), self.req(40, 60, 4000)]
        self.assertEqual(cache_sim.cold_floor(run), 40)
        out = cache_sim.replay(run, self.PRICE, floor=40)
        self.assertAlmostEqual(out["5m"] * 1e6, 2 * (40 + 120))

    def test_report_has_both_policies_and_calibration(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        path, _ = fixture_db.build(tmp.name)
        db = dbmod.open_db(path)
        dbmod.scope(db)
        rows = {r["agent_type"]: r for r in cache_sim.build(db, Scrubber(use_local=False))["by_agent_type"]}
        db.close()
        sub = rows["rust-gameserver-dev"]
        self.assertEqual(sub["observed_policy"], {"5m": 1})
        self.assertEqual(sub["gaps"], {"first": 1, "<=5m": 0, "5m-1h": 1, ">1h": 0})
        self.assertGreater(sub["sim_5m_usd"], sub["sim_1h_usd"] - 1)  # both present and priced
        self.assertIsNotNone(sub["calibration"])


if __name__ == "__main__":
    unittest.main()
