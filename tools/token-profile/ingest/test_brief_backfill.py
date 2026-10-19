"""S4b: the v6 to v7 upgrade backfills agents.brief_tag from the subagent transcripts."""

import sqlite3
import tempfile
import unittest
from pathlib import Path

from . import store
from .test_brief_tag import tags, write
from .test_support import run_ingest


def downgrade(db_path):
    """Turn a version 7 database back into version 6: drop agents.brief_tag and reset the version."""
    conn = sqlite3.connect(db_path)
    conn.execute("ALTER TABLE agents DROP COLUMN brief_tag")
    conn.execute("UPDATE meta SET value = '6' WHERE key = 'schema_version'")
    conn.commit()
    conn.close()


class BriefBackfillTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.db = self.root / "d.sqlite"

    def test_the_upgrade_backfills_the_tag(self):
        write(self.root, ["plain brief", "AH-99 review fix, pass 3 of 3"], fork_brief="review fix, pass 1 of 3")
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        downgrade(self.db)
        store.open_db(self.db).close()
        self.assertEqual(tags(self.db), {"c0": None, "c1": "review-fix", "f0": None})

    def test_a_missing_transcript_is_skipped(self):
        write(self.root, ["review fix, pass 1 of 3", "review fix, pass 2 of 3"])
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        downgrade(self.db)
        conn = sqlite3.connect(self.db)
        (path,) = conn.execute("SELECT path FROM ingest_files WHERE path LIKE '%agent-c1.jsonl'").fetchone()
        conn.close()
        Path(path).unlink()
        store.open_db(self.db).close()
        self.assertEqual(tags(self.db), {"c0": "review-fix", "c1": None})

    def test_an_upgraded_database_ingests_again(self):
        write(self.root, ["plain brief", "AH-99 review fix, pass 3 of 3"], fork_brief="review fix, pass 1 of 3")
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        downgrade(self.db)
        store.open_db(self.db).close()
        upgraded = tags(self.db)
        self.assertEqual(run_ingest(self.root / "p", self.db, prs=[]), 0)
        self.assertEqual(tags(self.db), upgraded)
