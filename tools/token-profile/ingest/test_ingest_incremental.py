"""Incremental ingest: a re-run adds nothing, and an appended file adds only its new records."""

import shutil
import sqlite3
import tempfile
import unittest
from pathlib import Path

from .test_support import PREFIX, build_fixtures, dump, pr, run_ingest

PRS = [pr(4242, build_fixtures.WORKER_BRANCH, "2026-10-01T11:00:00Z")]


class IncrementalTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.base = Path(cls.tmp.name)
        cls.full = build_fixtures.build(cls.base / "full")
        cls.main_file = cls.full / PREFIX / f"{build_fixtures.SESSION}.jsonl"
        run_ingest(cls.full, cls.base / "oneshot.sqlite", "--allow-unknown", prs=PRS)
        cls.oneshot = dump(cls.base / "oneshot.sqlite")

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def staged(self, name, cut):
        """A copy of the fixture tree whose main transcript holds only its first `cut` bytes."""
        root = self.base / name
        shutil.copytree(self.full, root)
        data = self.main_file.read_bytes()
        target = root / PREFIX / self.main_file.name
        target.write_bytes(data[:cut])
        return root, target, data

    def test_rerun_adds_nothing(self):
        db = self.base / "rerun.sqlite"
        run_ingest(self.full, db, "--allow-unknown", prs=PRS)
        first = dump(db)
        run_ingest(self.full, db, "--allow-unknown", prs=PRS)
        self.assertEqual(dump(db), first)
        self.assertEqual(first, self.oneshot)
        conn = sqlite3.connect(db)
        files = conn.execute("SELECT files_scanned FROM profiler_runs ORDER BY run_id").fetchall()
        conn.close()
        self.assertEqual(files, [(2,), (0,)])

    def test_appending_at_every_line_gives_the_one_shot_result(self):
        data = self.main_file.read_bytes()
        cuts = [i + 1 for i, b in enumerate(data) if b == ord("\n")]
        # Also cut mid-line: the partial line must wait for the next run, not be dropped or misread.
        cuts += [c + 7 for c in cuts[:-1:5]]
        for n, cut in enumerate(sorted(cuts)):
            with self.subTest(cut=cut):
                root, target, _ = self.staged(f"cut{n}", cut)
                db = self.base / f"cut{n}.sqlite"
                run_ingest(root, db, "--allow-unknown", prs=PRS)
                target.write_bytes(data)
                run_ingest(root, db, "--allow-unknown", prs=PRS)
                self.assertEqual(dump(db), self.oneshot)

    def test_partial_then_final_record_keeps_the_final_usage(self):
        data = self.main_file.read_bytes()
        lines = data.split(b"\n")
        partial = next(i for i, line in enumerate(lines) if b'"req_A"' in line)
        cut = len(b"\n".join(lines[:partial + 1])) + 1
        root, target, _ = self.staged("partial", cut)
        db = self.base / "partial.sqlite"
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        conn = sqlite3.connect(db)
        self.addCleanup(conn.close)
        self.assertEqual(conn.execute("SELECT output_tokens, records_seen FROM requests"
                                      " WHERE request_id = 'req_A'").fetchone(), (3, 1))
        target.write_bytes(data)
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        self.assertEqual(conn.execute("SELECT output_tokens, records_seen FROM requests"
                                      " WHERE request_id = 'req_A'").fetchone(), (400, 2))

    def test_a_rewritten_file_is_read_again_without_duplicates(self):
        root, target, data = self.staged("rewrite", len(self.main_file.read_bytes()))
        db = self.base / "rewrite.sqlite"
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        # Same records, but the first line changed: the stored offset no longer means anything.
        target.write_bytes(b'{"type": "mode", "mode": "x"}\n' + data)
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        got = dump(db)
        self.assertEqual(got["requests"], self.oneshot["requests"])
        self.assertEqual(got["tool_calls"], self.oneshot["tool_calls"])

    def test_deleted_transcripts_keep_their_rows(self):
        root, target, _ = self.staged("deleted", len(self.main_file.read_bytes()))
        db = self.base / "deleted.sqlite"
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        target.unlink()
        run_ingest(root, db, "--allow-unknown", prs=PRS)
        self.assertEqual(dump(db)["requests"], self.oneshot["requests"])


if __name__ == "__main__":
    unittest.main()
