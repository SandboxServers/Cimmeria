"""S4a: a packet-coder whose first prompt names a review fix pass is tagged; no prompt text is stored."""

import sqlite3
import tempfile
import unittest
from pathlib import Path

from . import classify, store
from .test_support import run_ingest, transcript, write_session
from .test_support import build_fixtures as bf

SID = "77777777-0000-4000-8000-000000000007"
MARKER = "zq-brief-marker-5531"


def write(root, briefs, fork_brief=None):
    """A main session and one packet-coder per brief (agent ids c0, c1, ...); a brief may be a list of prompts.
    With fork_brief, a fork f0 of c0 whose first (copied) prompt is fork_brief. Returns (main, subagent list)."""
    m = transcript(SID)
    m.user_text(bf.ts(1), "go", origin={"kind": "human"})
    bf.request(m, bf.ts(1, 5), "r_main", bf.usage(1, 1, 0, 1, 1, 0))
    agents = []
    for i, brief in enumerate(briefs):
        a = transcript(SID, agent_id=f"c{i}")
        for k, text in enumerate(brief if isinstance(brief, list) else [brief]):
            a.user_text(bf.ts(2 + i, 2 * k), text)
            bf.request(a, bf.ts(2 + i, 2 * k + 1), f"r_c{i}_{k}", bf.usage(1, 5, 0, 1, 1, 0))
        agents.append((f"c{i}", a, {"agentType": "packet-coder"}))
    if fork_brief is not None:
        f = transcript(SID, agent_id="f0")
        f.user_text(bf.ts(20), fork_brief)
        bf.request(f, bf.ts(20, 5), "r_f0", bf.usage(1, 5, 0, 1, 1, 0))
        agents.append(("f0", f, {"agentType": "fork", "isFork": True, "parentAgentId": "c0"}))
    write_session(root / "p", SID, m, agents)
    return m, agents


def tags(db_path):
    conn = sqlite3.connect(db_path)
    try:
        return dict(conn.execute("SELECT agent_id, brief_tag FROM agents"))
    finally:
        conn.close()


class BriefTagTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.db = self.root / "d.sqlite"

    def test_pattern(self):
        for text in ("AH-99 review fix, pass 2 of 3, report x", "Review Fix, Pass 1 of 3"):
            self.assertEqual(classify.brief_tag(text), "review-fix", msg=text)
        for text in ("review fix, pass N of 3", "review fix, pass 4 of 3", "continuation 2 of 3", "", None):
            self.assertIsNone(classify.brief_tag(text), msg=repr(text))

    def test_only_a_first_prompt_tags(self):
        write(self.root, ["AH-99: implement the packet", "AH-99 review fix, pass 2 of 3",
                          ["AH-99: implement", "now review fix, pass 1 of 3"]])
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        self.assertEqual(tags(self.db), {"c0": None, "c1": "review-fix", "c2": None})

    def test_a_fork_is_never_tagged(self):
        write(self.root, ["plain brief"], fork_brief="review fix, pass 1 of 3")
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        self.assertIsNone(tags(self.db)["f0"])
        self.assertIsNone(tags(self.db)["c0"])

    def test_the_tag_survives_an_incremental_run(self):
        m, agents = write(self.root, ["review fix, pass 3 of 3"])
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        a = agents[0][1]
        a.user_text(bf.ts(30), "a later prompt")
        bf.request(a, bf.ts(30, 5), "r_late", bf.usage(1, 5, 0, 1, 1, 0))
        write_session(self.root / "p", SID, m, agents)
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        self.assertEqual(tags(self.db)["c0"], "review-fix")
        conn = sqlite3.connect(self.db)
        try:
            self.assertEqual(conn.execute("SELECT COUNT(*) FROM requests WHERE request_id = 'r_late'").fetchone()[0], 1)
        finally:
            conn.close()

    def test_no_prompt_text_is_stored(self):
        write(self.root, [f"review fix, pass 1 of 3 {MARKER}", f"plain {MARKER}"])
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        conn = sqlite3.connect(self.db)
        try:
            names = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table'")]
            for name in names:
                for row in conn.execute(f'SELECT * FROM "{name}"'):
                    with self.subTest(table=name):
                        self.assertNotIn(MARKER, repr(row))
        finally:
            conn.close()
        self.assertEqual(tags(self.db)["c0"], "review-fix")

    def test_a_version_6_database_is_upgraded_in_place(self):
        write(self.root, ["plain brief"])
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        conn = sqlite3.connect(self.db)
        try:
            requests = conn.execute("SELECT COUNT(*) FROM requests").fetchone()[0]
            conn.execute("ALTER TABLE agents DROP COLUMN brief_tag")
            conn.execute("UPDATE meta SET value = '6' WHERE key = 'schema_version'")
            conn.commit()
        finally:
            conn.close()
        db = store.open_db(self.db)
        try:
            self.assertEqual(db.execute("SELECT value FROM meta WHERE key = 'schema_version'").fetchone()[0], "7")
            self.assertIn("brief_tag", {r[1] for r in db.execute("PRAGMA table_info(agents)")})
            self.assertEqual(db.execute("SELECT COUNT(*) FROM requests").fetchone()[0], requests)
        finally:
            db.close()
