"""Unit tests for the pure parts of the ingest: prices, fingerprints and trigger classification."""

import sqlite3
import tempfile
import unittest
from pathlib import Path

from . import prices
from .classify import classify
from .fingerprint import command_head, description_pr_ref, fingerprint, gh_pr_ref, repo_relative
from .test_support import run_ingest, transcript, write_session
from .test_support import build_fixtures as bf


class PriceTest(unittest.TestCase):
    def test_cache_read_weights_differ_by_model(self):
        opus = prices.price_for("claude-opus-5-5")
        fable = prices.price_for("claude-fable-5-1")
        self.assertAlmostEqual(opus["cache_read"] / opus["input"], 0.05)
        self.assertAlmostEqual(fable["cache_read"] / fable["input"], 0.025)
        self.assertAlmostEqual(prices.price_for("claude-sonnet-5")["cache_read"] / 2.0, 0.1)

    def test_cache_writes_split_by_ttl(self):
        for model in prices.PRICE_TABLES[prices.CURRENT]:
            row = prices.price_for(model)
            self.assertAlmostEqual(row["cache_write_5m"], row["input"] * 1.25, msg=model)
            self.assertAlmostEqual(row["cache_write_1h"], row["input"] * 2.0, msg=model)

    def test_estimate_is_per_model_and_never_adds_thinking(self):
        tokens = {"input": 1_000_000, "output": 1_000_000, "cache_read": 1_000_000, "cache_write_5m": 1_000_000,
                  "cache_write_1h": 1_000_000}
        self.assertAlmostEqual(prices.estimate_usd("claude-opus-5-5", tokens), 4 + 20 + 0.2 + 5 + 8)
        self.assertAlmostEqual(prices.estimate_usd("claude-fable-5-1", tokens), 10 + 50 + 0.25 + 12.5 + 20)
        self.assertIsNone(prices.estimate_usd("claude-unknown-9", tokens))

    def test_model_ids_are_normalized(self):
        self.assertEqual(prices.normalize_model("claude-haiku-4-5-20251001"), "claude-haiku-4-5")
        self.assertEqual(prices.normalize_model("claude-opus-5[1m]"), "claude-opus-5")

    def test_table_is_stored_with_its_version(self):
        db = sqlite3.connect(":memory:")
        db.executescript((Path(__file__).resolve().parent.parent / "schema.sql").read_text(encoding="utf-8"))
        prices.store(db)
        row = db.execute("SELECT input, cache_read FROM price_tables WHERE version = ? AND model = ?",
                         (prices.CURRENT, "claude-opus-5-5")).fetchone()
        self.assertEqual(row, (4.0, 0.2))


class FingerprintTest(unittest.TestCase):
    def test_shell_heads(self):
        cases = {
            "cargo nextest run --token supersecret": "cargo nextest",
            "cd C:\\x && DATABASE_URL=postgres://u:p@h/db cargo clippy --all": "cargo clippy",
            "cd /tmp; git rebase origin/main": "git rebase",
            'FOO="a b" BAR=1 gh pr view 12': "gh pr",
            "curl https://user:pass@example.internal/": "curl",
            "echo abc123def456ghi789": "echo",
            "C:\\Python\\python.exe C:\\Users\\x\\secret.py": "python",
            '& "C:\\Program Files\\Tool\\tool.EXE" --key k': "tool",
            "bash tools/build-lane/lane.sh cargo check -p x": "bash tools/build-lane/lane.sh",
            "bash /home/u/secret.sh": "bash",
            "sed -n 1,20p file": "sed -n",
            "": None,
        }
        for command, want in cases.items():
            self.assertEqual(command_head(command), want, command)

    def test_paths(self):
        root = "c:/users/x/src/cimmeria"
        self.assertEqual(repo_relative("C:\\Users\\x\\src\\Cimmeria\\docs\\a.md", root), "docs/a.md")
        self.assertEqual(repo_relative("C:\\Users\\x\\src\\Cimmeria\\.claude\\worktrees\\w1\\crates\\b.rs", root),
                         "crates/b.rs")
        self.assertEqual(repo_relative("C:\\Users\\x\\secret\\k.pem", root), "<external>")
        self.assertEqual(repo_relative("/etc/passwd", root), "<external>")
        self.assertEqual(repo_relative("../../outside", root), "<external>")
        self.assertEqual(repo_relative("docs/b.md", root), "docs/b.md")

    def test_mcp_and_others(self):
        self.assertEqual(fingerprint("mcp__ghidra__decompile_function", {"x": "C:\\secret"}),
                         ("mcp__ghidra__decompile_function", "ghidra"))
        self.assertEqual(fingerprint("WebFetch", {"url": "https://user:pass@h/"}), (None, None))

    def test_pr_references(self):
        self.assertEqual(gh_pr_ref("gh pr checks 4242 --watch"), (4242, True))
        self.assertEqual(gh_pr_ref("cd x && gh pr merge #12 --squash"), (12, True))
        self.assertEqual(gh_pr_ref("gh pr view https://github.com/o/r/pull/7"), (7, True))
        self.assertEqual(gh_pr_ref("gh pr create --title 'fix 12'"), (None, True))
        self.assertEqual(gh_pr_ref("gh pr list"), (None, False))
        self.assertEqual(description_pr_ref("Watch CI for #88"), 88)
        self.assertIsNone(description_pr_ref("Compare #1 and #2"))


class ClassifyTest(unittest.TestCase):
    def test_precedence(self):
        self.assertEqual(classify("x", is_compact_summary=True, scheduled=True)[:2], ("compact_summary", "R3"))
        self.assertEqual(classify("<task-notification><task-id>b1</task-id>Monitor event: x</task-notification>"),
                         ("monitor_event", "R5", "b1"))
        self.assertEqual(classify("<command-message>skill is running</command-message>")[:2],
                         ("local_command", "R12"))
        self.assertEqual(classify("hi", origin={"kind": "future"}, is_meta=True)[:2], ("auxiliary", "R14"))
        self.assertEqual(classify("hi", origin={"kind": "future"})[:2], ("unknown", "R15"))

    def test_source_ref_is_never_free_text(self):
        kind, _, ref = classify('<teammate-message teammate_id="has spaces and a secret">x</teammate-message>')
        self.assertEqual((kind, ref), ("teammate_message", None))

    def test_auxiliary_record_does_not_replace_a_pending_turn_start(self):
        # Observed: a human prompt followed by harness meta records before the first request.
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        sid = "44444444-0000-4000-8000-000000000004"
        m = transcript(sid)
        m.user_text(bf.ts(1), "do it", origin={"kind": "human"})
        m.user_text(bf.ts(1, 1), "<system-reminder>x</system-reminder>", isMeta=True)
        bf.request(m, bf.ts(1, 5), "r_h", bf.usage(1, 1, 0, 1, 1, 0))
        m.user_text(bf.ts(2), "<system-reminder>y</system-reminder>", isMeta=True)
        bf.request(m, bf.ts(2, 5), "r_aux", bf.usage(1, 1, 0, 1, 1, 0))
        write_session(Path(tmp.name) / "p", sid, m)
        db = Path(tmp.name) / "d.sqlite"
        self.assertEqual(run_ingest(Path(tmp.name) / "p", db, prs=[]), 0)
        conn = sqlite3.connect(db)
        got = dict(conn.execute("SELECT r.request_id, t.kind FROM requests r JOIN triggers t USING (trigger_id)"))
        conn.close()
        self.assertEqual(got, {"r_h": "human_prompt", "r_aux": "auxiliary"})


class ForkAndCostStateTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)

    def test_copied_history_belongs_to_the_forks_parent(self):
        # Observed 2026-10-03: forks copy their parent's requests, and a parent subagent's
        # file name can sort after its forks' ("afec..." after "a1fc...").
        sid = "55555555-0000-4000-8000-000000000005"
        m = transcript(sid)
        m.user_text(bf.ts(1), "go", origin={"kind": "human"})
        bf.request(m, bf.ts(1, 5), "r_main", bf.usage(1, 1, 0, 1, 1, 0))
        parent = transcript(sid, agent_id="zz-parent")
        parent.user_text(bf.ts(2), "work", isMeta=True)
        bf.request(parent, bf.ts(2, 5), "r_parent", bf.usage(1, 50, 0, 1, 1, 0))
        fork = transcript(sid, agent_id="aa-fork")
        bf.request(fork, bf.ts(2, 5), "r_parent", bf.usage(1, 50, 0, 1, 1, 0))  # the copy
        fork.user_text(bf.ts(3), "forked task", isMeta=True)
        bf.request(fork, bf.ts(3, 5), "r_fork", bf.usage(1, 7, 0, 1, 1, 0))
        write_session(self.root / "p", sid, m, [
            ("zz-parent", parent, {"agentType": "general-purpose"}),
            ("aa-fork", fork, {"agentType": "fork", "isFork": True, "parentAgentId": "zz-parent"})])
        db = self.root / "d.sqlite"
        self.assertEqual(run_ingest(self.root / "p", db, prs=[]), 0)
        conn = sqlite3.connect(db)
        got = dict(conn.execute("SELECT request_id, agent_id FROM requests"))
        conn.close()
        self.assertEqual(got, {"r_main": None, "r_parent": "zz-parent", "r_fork": "aa-fork"})

    def test_cost_state_keeps_its_process_start(self):
        sid = "66666666-0000-4000-8000-000000000006"
        m = transcript(sid)
        m.user_text(bf.ts(1), "go", origin={"kind": "human"})
        bf.request(m, bf.ts(1, 5), "r_1", bf.usage(1, 1, 0, 1, 1, 0))
        m.raw({"type": "cost-state", "sessionId": sid, "totalCostUSD": 1.5, "hasUnknownModelCost": False,
               "startTime": 1790502074327, "modelUsage": {}})
        write_session(self.root / "p", sid, m)
        db = self.root / "d.sqlite"
        self.assertEqual(run_ingest(self.root / "p", db, prs=[]), 0)
        conn = sqlite3.connect(db)
        row = conn.execute("SELECT total_cost_usd, process_start FROM cost_states").fetchone()
        conn.close()
        self.assertEqual(row, (1.5, "2026-09-27T09:41:14.327Z"))


if __name__ == "__main__":
    unittest.main()
