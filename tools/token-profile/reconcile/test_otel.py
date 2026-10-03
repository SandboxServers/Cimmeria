"""The OTel reconciliation against a synthetic event export."""

import json
import tempfile
import unittest
from pathlib import Path

from ingest import prices
from ingest.test_support import build_fixtures as bf
from ingest.test_support import run_ingest, transcript, write_session
from report import db as report_db
from report.scrub import Scrubber

from . import otel

SID = "77777777-0000-4000-8000-000000000007"
OPUS = "claude-opus-5-5"
N = 60


def usd(u):
    return prices.estimate_usd(OPUS, {"input": u["input_tokens"], "output": u["output_tokens"],
                                      "cache_read": u["cache_read_input_tokens"],
                                      "cache_write_5m": u["cache_creation"]["ephemeral_5m_input_tokens"],
                                      "cache_write_1h": u["cache_creation"]["ephemeral_1h_input_tokens"]})


def event(rid, when, u, query_source="main", cost=None):
    return {"event.name": "api_request", "session.id": SID, "request_id": rid, "model": OPUS, "query_source": query_source,
            "event.timestamp": when, "input_tokens": u["input_tokens"], "output_tokens": u["output_tokens"],
            "cache_read_tokens": u["cache_read_input_tokens"],
            "cache_creation_tokens": u["cache_creation_input_tokens"], "cost_usd": usd(u) if cost is None else cost}


def otlp(events):
    """The same events as an OTLP JSON log export."""
    def attr(k, v):
        if isinstance(v, str):
            return {"key": k, "value": {"stringValue": v}}
        if isinstance(v, int):
            return {"key": k, "value": {"intValue": str(v)}}
        return {"key": k, "value": {"doubleValue": v}}
    records = [{"timeUnixNano": "1790856000000000000", "body": {"stringValue": otel.EVENT},
                "attributes": [attr(k, v) for k, v in e.items() if k != "session.id"]} for e in events]
    return {"resourceLogs": [{"resource": {"attributes": [attr("session.id", SID),
                                                          attr("deployment.environment", "cimmeria-dev")]},
                              "scopeLogs": [{"logRecords": records}]}]}


class OtelReconcileTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        root = Path(cls.tmp.name)
        m = transcript(SID)
        m.user_text(bf.ts(0), "go", origin={"kind": "human"})
        cls.requests = []
        for i in range(N):
            u = bf.usage(3, 100 + i, 0, 50000 + i, 0, 700)
            bf.request(m, bf.ts(1 + i // 60, i % 60), f"r{i}", u)
            cls.requests.append((f"r{i}", bf.ts(1 + i // 60, i % 60), u))
        write_session(root / "p", SID, m)
        cls.db_path = root / "d.sqlite"
        assert run_ingest(root / "p", cls.db_path, prs=[]) == 0

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def compare(self, events, fmt="flat"):
        path = Path(self.tmp.name) / f"events-{fmt}.json"
        if fmt == "otlp":
            path.write_text(json.dumps(otlp(events)), encoding="utf-8")
        elif fmt == "signoz":
            rows = [{"timestamp": e["event.timestamp"], "body": otel.EVENT,
                     "attributes_string": {k: v for k, v in e.items() if isinstance(v, str)},
                     "attributes_number": {k: v for k, v in e.items() if not isinstance(v, str)}} for e in events]
            path.write_text(json.dumps({"status": "success", "data": {"results": [{"rows": rows}]}}),
                            encoding="utf-8")
        else:
            path.write_text("\n".join(json.dumps(e) for e in events), encoding="utf-8")
        db = report_db.open_db(self.db_path)
        report_db.scope(db)
        try:
            out = otel.compare(db, Scrubber(use_local=False), otel.load(path), {SID: 1.0})
        finally:
            db.close()
        return out, {c["name"]: c for c in out["checks"]}

    def events(self):
        return [event(rid, when, u) for rid, when, u in self.requests]

    def test_every_format_matches_every_request(self):
        for fmt in ("flat", "otlp", "signoz"):
            out, checks = self.compare(self.events(), fmt)
            self.assertEqual(out["matched"]["requests"], N, fmt)
            self.assertEqual(out["matched"]["token_mismatches"], 0, fmt)
            self.assertTrue(out["ok"], (fmt, checks))

    def test_otel_only_requests_are_grouped_by_query_source(self):
        side = bf.usage(9000, 40, 0, 120000, 0, 0)
        out, checks = self.compare(self.events() + [event("aux1", bf.ts(1, 30), side, "auxiliary")])
        self.assertTrue(out["ok"], checks)
        self.assertEqual([(r["query_source"], r["requests"]) for r in out["otel_only"]], [("auxiliary", 1)])
        self.assertAlmostEqual(out["otel_only_cost_usd"], usd(side))

    def test_lost_telemetry_fails(self):
        out, checks = self.compare(self.events()[: N // 2] + self.events()[N // 2 + 5:])
        self.assertEqual(out["profiler_only"]["requests"], 5)
        self.assertFalse(checks["profiler_only"]["ok"])

    def test_token_mismatch_fails(self):
        ev = self.events()
        ev[3]["output_tokens"] = 3  # what a streaming partial would carry
        out, checks = self.compare(ev)
        self.assertEqual(out["matched"]["token_mismatches"], 1)
        self.assertFalse(checks["matched_token_mismatch"]["ok"])

    def test_cost_disagreement_fails(self):
        ev = [dict(e, cost_usd=e["cost_usd"] * 1.1) for e in self.events()]
        _, checks = self.compare(ev)
        self.assertFalse(checks["matched_cost_residual"]["ok"])

    def test_no_identifiers_in_output(self):
        out, _ = self.compare(self.events())
        text = json.dumps(out)
        self.assertNotIn(SID, text)
        self.assertNotIn('"r1"', text)


if __name__ == "__main__":
    unittest.main()
