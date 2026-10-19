"""S4c: a packet-coder briefed as a review fix pass is a fixer in the packet join, the cost rollup and overstep.

The fixture helpers are imported; no test class from another module is, so each test here runs once.
"""

import sqlite3
import tempfile
import unittest
from pathlib import Path

from ingest.prices import CURRENT
from metrics import cost, overstep, packets, registry
from metrics.test_cost import build
from metrics.test_overstep import _detect, _req
from metrics.test_packets import CONFIG, WORKTREE, _agent, _github, _ledger, _record


def tag_coder(path):
    """Tag the fixture's packet coder (a-coder) as a review fix pass, the way the ingest tags a first prompt."""
    conn = sqlite3.connect(path)
    conn.execute("UPDATE agents SET brief_tag = 'review-fix' WHERE agent_id = 'a-coder'")
    conn.commit()
    conn.close()


def drop_column(path):
    """Turn a version 7 database into version 6: drop agents.brief_tag and reset the schema version."""
    conn = sqlite3.connect(path)
    conn.execute("ALTER TABLE agents DROP COLUMN brief_tag")
    conn.execute("UPDATE meta SET value = '6' WHERE key = 'schema_version'")
    conn.commit()
    conn.close()


class BriefRoleTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = str(Path(tmp.name) / "profile.db")

    def test_brief_role_flips_only_a_haiku_coder(self):
        self.assertEqual(packets.brief_role("implementer-haiku", "review-fix"), "fixer")
        self.assertEqual(packets.brief_role("implementer-haiku", None), "implementer-haiku")
        self.assertEqual(packets.brief_role("reviewer", "review-fix"), "reviewer")
        self.assertEqual(packets.brief_role("implementer-domain", "review-fix"), "implementer-domain")
        self.assertIsNone(packets.brief_role(None, "review-fix"))

    def test_a_tagged_coder_is_a_fixer_in_the_packet_join(self):
        agents = [_agent("cm01-a1", WORKTREE.format("lr-01"), "2026-10-01T06:00:00Z", "packet-coder", "packet-coder",
                         name="cm01-a1"),
                  _agent("cm01-rev1", WORKTREE.format("lr-01"), "2026-10-01T10:00:00Z", "packet-reviewer",
                         "packet-reviewer", name="cm01-rev1"),
                  _agent("cm01-a2", WORKTREE.format("lr-01"), "2026-10-01T11:00:00Z", "packet-coder", "packet-coder",
                         name="cm01-a2")]
        agents[2]["brief_tag"] = "review-fix"
        record = _record(packets.join_campaigns(_ledger(), {"agents": agents, "stops": {}}, _github(), CONFIG), "LR-01")
        self.assertEqual((record["implementer_sessions"], record["fixer_sessions"]), (1, 1))
        agents[2]["brief_tag"] = None
        record = _record(packets.join_campaigns(_ledger(), {"agents": agents, "stops": {}}, _github(), CONFIG), "LR-01")
        self.assertEqual((record["implementer_sessions"], record["fixer_sessions"]), (2, 0))

    def test_load_db_reads_the_tag(self):
        build(self.path)
        tag_coder(self.path)
        tags = {a["agent_id"]: a["brief_tag"] for a in packets.load_db(self.path)["agents"]}
        self.assertEqual(tags, {"a-coder": "review-fix", "a-rev": None})

    def test_cost_counts_a_tagged_coder_as_fixer(self):
        build(self.path)
        tag_coder(self.path)
        conn = sqlite3.connect(f"file:{self.path}?mode=ro", uri=True)
        try:
            config = registry.load_config()
            out = cost.rollup(cost.load_requests(conn, config["window"]["since"], config["window"]["until"]),
                              CURRENT, config["roles"])
        finally:
            conn.close()
        self.assertAlmostEqual(out["camp-a"]["cost"]["by_role"]["fixer"], 0.0012, places=9)
        self.assertAlmostEqual(out["camp-a"]["cost"]["by_role"]["implementer-haiku"], 0.0, places=9)

    def test_overstep_counts_a_tagged_coder_as_fixer(self):
        tagged = _req(agent_id="a3", agent_type="packet-coder", brief_tag="review-fix", worktree=".claude/worktrees/cm-07")
        self.assertEqual(_detect([tagged])["fixer_requests"], {"cm-07": 1})
        untagged = _req(agent_id="a3", agent_type="packet-coder", worktree=".claude/worktrees/cm-07")
        self.assertEqual(_detect([untagged])["fixer_requests"], {})

    def test_overstep_load_requests_carries_the_tag(self):
        build(self.path)
        tag_coder(self.path)
        pairs = [(r["agent_id"], r["brief_tag"]) for r in overstep.load_requests(self.path)]
        for agent_id, tag in pairs:
            self.assertEqual(tag, "review-fix" if agent_id == "a-coder" else None, msg=str(agent_id))
        self.assertTrue(any(agent_id == "a-coder" for agent_id, _ in pairs))
        self.assertTrue(any(agent_id != "a-coder" for agent_id, _ in pairs))

    def test_a_database_without_the_column_reads_untagged(self):
        build(self.path)
        drop_column(self.path)
        conn = sqlite3.connect(f"file:{self.path}?mode=ro", uri=True)
        try:
            self.assertEqual(packets.brief_tags(conn), {})
        finally:
            conn.close()
        agents = packets.load_db(self.path)["agents"]
        self.assertTrue(agents)
        self.assertTrue(all(a["brief_tag"] is None for a in agents))
        requests = overstep.load_requests(self.path)
        self.assertTrue(requests)
        self.assertTrue(all(r["brief_tag"] is None for r in requests))


if __name__ == "__main__":
    unittest.main()
