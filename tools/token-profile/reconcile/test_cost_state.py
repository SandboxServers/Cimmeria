"""The cost-state reconciliation, end to end: synthetic transcripts through the ingest, then the checks."""

import contextlib
import io
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

from ingest import prices
from ingest.test_support import build_fixtures as bf
from ingest.test_support import run_ingest, transcript, write_session
from report import db as report_db
from report.scrub import Scrubber

from . import cli

OPUS = "claude-opus-5-5"


def epoch_ms(iso):
    return int(datetime.fromisoformat(iso.replace("Z", "+00:00")).timestamp() * 1000)


def tokens(u):
    return {"input": u["input_tokens"], "output": u["output_tokens"], "cache_read": u["cache_read_input_tokens"],
            "cache_write_5m": u["cache_creation"]["ephemeral_5m_input_tokens"],
            "cache_write_1h": u["cache_creation"]["ephemeral_1h_input_tokens"]}


def cost_state(sid, start, usages, model=OPUS, scale=1.0, extra_output=0, extra_read=0):
    """The cost-state record Claude Code would write for these requests, optionally distorted."""
    t = {k: sum(tokens(u)[k] for u in usages) for k in prices.TOKEN_COLUMNS}
    usd = sum(prices.estimate_usd(model, tokens(u)) for u in usages)
    usd += (extra_output * prices.price_for(model)["output"] + extra_read * prices.price_for(model)["cache_read"]) / 1e6
    return {"type": "cost-state", "sessionId": sid, "totalCostUSD": usd * scale, "hasUnknownModelCost": False,
            "startTime": epoch_ms(start),
            "modelUsage": {model: {"inputTokens": t["input"], "outputTokens": t["output"] + extra_output,
                                   "cacheReadInputTokens": t["cache_read"] + extra_read,
                                   "cacheCreationInputTokens": t["cache_write_5m"] + t["cache_write_1h"],
                                   "thinkingTokens": 0, "webSearchRequests": 0, "costUSD": usd * scale}}}


class CostStateReconcileTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.n = 0

    def session(self, n_requests=25, resumed_before=0, model=OPUS, **distort):
        """A session of n_requests (plus resumed_before older ones) with a subagent; returns its cost-state usages."""
        self.n += 1
        sid = f"{self.n:08d}-0000-4000-8000-000000000000"
        m = transcript(sid)
        m.user_text(bf.ts(0), "old work", origin={"kind": "human"})
        for i in range(resumed_before):
            bf.request(m, bf.ts(0, i + 1), f"{sid}-old{i}", bf.usage(5, 100, 0, 9000, 0, 800), model=model)
        start = bf.ts(10)
        m.user_text(bf.ts(10, 30), "go", origin={"kind": "human"})
        usages = []
        for i in range(n_requests):
            u = bf.usage(3, 200 + i, 0, 40000 + i, 0, 1000)
            bf.request(m, bf.ts(11, i), f"{sid}-r{i}", u, model=model)
            usages.append(u)
        sub = transcript(sid, agent_id=f"agent{self.n}")
        sub.user_text(bf.ts(20), "subtask", isMeta=True)
        u = bf.usage(2, 300, 0, 20000, 1500, 0)
        bf.request(sub, bf.ts(20, 5), f"{sid}-s0", u, model=model)
        usages.append(u)
        m.raw(cost_state(sid, start, usages, model=model, **distort))
        write_session(self.root / "p", sid, m, [(f"agent{self.n}", sub, {"agentType": "general-purpose"})])

    def reconcile(self):
        db = self.root / "d.sqlite"
        self.assertEqual(run_ingest(self.root / "p", db, prs=[]), 0)
        result = cli.build(db, sc=Scrubber(use_local=False))
        with contextlib.redirect_stdout(io.StringIO()):
            code = cli.main(["--db", str(db)])
        checks = {c["name"]: c for c in result["cost_state"]["checks"]}
        return result["cost_state"], checks, code

    def test_agreeing_sources_pass(self):
        self.session()
        self.session(n_requests=30)
        c, checks, code = self.reconcile()
        self.assertEqual(code, 0)
        self.assertTrue(all(x["ok"] for x in checks.values()), checks)
        self.assertAlmostEqual(c["totals"]["gap_share"], 0.0, places=9)
        self.assertAlmostEqual(checks["price_residual"]["value"], 0.0, places=9)
        self.assertEqual(c["sessions_checked_individually"], 2)

    def test_resumed_session_is_compared_over_its_process_only(self):
        # The cost-state of a resumed process holds none of the earlier requests.
        self.session(resumed_before=40)
        c, checks, code = self.reconcile()
        self.assertEqual(code, 0, checks)
        self.assertAlmostEqual(c["totals"]["gap_share"], 0.0, places=9)
        self.assertEqual((c["uncovered"]["requests"], c["uncovered"]["resumed_sessions"]), (40, 1))

    def test_profiler_over_the_cost_state_fails(self):
        # What a request counted twice, or a copy kept, looks like.
        self.session(scale=0.9)
        _, checks, code = self.reconcile()
        self.assertEqual(code, 4)
        self.assertFalse(checks["overcount"]["ok"])
        self.assertEqual(checks["session_overcount"]["value"], 1)

    def test_price_table_drift_fails(self):
        # Same tokens, different dollars: the profiler's table no longer matches Claude Code's.
        self.session(scale=1.05, extra_output=0)
        _, checks, code = self.reconcile()
        self.assertEqual(code, 4)
        self.assertFalse(checks["price_residual"]["ok"])

    def test_missing_output_fails(self):
        # A first-record dedupe keeps each request's streaming partial: most output disappears.
        self.session(extra_output=20000)
        _, checks, code = self.reconcile()
        self.assertEqual(code, 4)
        self.assertFalse(checks["output_gap"]["ok"])

    def test_untranscribed_side_requests_within_tolerance(self):
        # Requests only the cost-state sees: big cache reads, little output.
        self.session(extra_read=300_000)
        c, checks, code = self.reconcile()
        self.assertEqual(code, 0, checks)
        self.assertLess(c["totals"]["gap_share"], -0.01)

    def test_undercount_beyond_tolerance_fails(self):
        # The transcripts stop recording a class of requests they used to.
        self.session(extra_read=1_000_000)
        _, checks, code = self.reconcile()
        self.assertEqual(code, 4)
        self.assertFalse(checks["undercount"]["ok"])

    def test_dated_model_ids_are_priced(self):
        self.session(model="claude-haiku-4-5-20251001")
        c, checks, code = self.reconcile()
        self.assertEqual(code, 0, checks)
        self.assertEqual(c["totals"]["unpriced_cost_state_usd"], 0)
        self.assertGreater(c["totals"]["profiler_usd"], 0)
        # The reports' per-request prices too (they matched model ids exactly before).
        db = report_db.open_db(self.root / "d.sqlite")
        report_db.scope(db)
        self.assertEqual(db.execute("SELECT COUNT(*) FROM rcost WHERE NOT priced").fetchone()[0], 0)
        db.close()

    def test_output_is_scrubbed_aggregates(self):
        self.session()
        db = self.root / "d.sqlite"
        run_ingest(self.root / "p", db, prs=[])
        out = self.root / "out"
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(cli.main(["--db", str(db), "--out", str(out)]), 0)
        text = (out / "reconcile.md").read_text(encoding="utf-8") + (out / "reconcile.json").read_text(encoding="utf-8")
        self.assertNotIn("00000001-0000", text)
        self.assertNotIn(str(self.root), text)


if __name__ == "__main__":
    unittest.main()
