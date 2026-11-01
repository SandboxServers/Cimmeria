"""Tests for the SubagentStop report-contract hook in .claude/hooks/report_contract.py."""

import json
import os
import subprocess
import sys
import tempfile
import time
import unittest
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HOOK = REPO / ".claude/hooks/report_contract.py"
BRIEF = "Review HT-01. Write your report to .claude/reports/ht/HT-01.md and end with the return line."
GOOD = "HT-01 done abc1234 .claude/reports/ht/HT-01.md"
AGENTS = {
    "packet-reviewer": "---\nname: packet-reviewer\nmodel: sonnet\n---\n\n## Reporting\n\nWrite a report.\n",
    "packet-coder": "---\nname: packet-coder\nmodel: haiku\n---\n\n## Reporting\n\nWrite a line.\n",
    "packet-fixer": "---\nname: packet-fixer\nmodel: claude-haiku-4-5\n---\n\n## Reporting\n\nWrite a line.\n",
}
DENY_TEXT = ("PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. "
             "Do not call any more tools.")
CAP_RESULT = {"type": "user", "timestamp": None, "message": {"role": "user", "content": [
    {"type": "tool_result", "tool_use_id": "t1", "is_error": True, "content": DENY_TEXT}]}}
OLD = datetime(2020, 1, 1, tzinfo=timezone.utc).timestamp()


def now_iso(offset=0):
    """An ISO timestamp for now, shifted by `offset` seconds."""
    return datetime.fromtimestamp(time.time() + offset, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def user(text, offset=0):
    """A transcript user record with a text content."""
    return {"type": "user", "timestamp": now_iso(offset), "message": {"role": "user", "content": text}}


def assistant(text, offset=1):
    """A transcript assistant record with one text block."""
    return {"type": "assistant", "timestamp": now_iso(offset),
            "message": {"role": "assistant", "content": [{"type": "text", "text": text}]}}


class HookTest(unittest.TestCase):
    """Runs the hook as a subprocess against a temporary project."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        agents = self.root / ".claude" / "agents"
        agents.mkdir(parents=True)
        for name, text in AGENTS.items():
            (agents / f"{name}.md").write_text(text, encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def transcript(self, *records):
        """Write the subagent transcript for session s1 and return the main transcript path."""
        sub = self.root / "s1" / "subagents" / "agent-a1.jsonl"
        sub.parent.mkdir(parents=True, exist_ok=True)
        sub.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
        return str(self.root / "s1.jsonl")

    def payload(self, agent="packet-reviewer", final=None, **extra):
        """A SubagentStop payload for agent a1 in session s1."""
        data = {"hook_event_name": "SubagentStop", "agent_type": agent, "agent_id": "a1",
                "transcript_path": str(self.root / "s1.jsonl"), "cwd": str(self.root),
                "stop_hook_active": False}
        if final is not None:
            data["last_assistant_message"] = final
        data.update(extra)
        return data

    def report(self, rel="ht/HT-01.md", text="# HT-01\n", mtime=None):
        """Write a report under .claude/reports and optionally set its mtime."""
        path = self.root / ".claude" / "reports" / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        if mtime is not None:
            os.utime(path, (mtime, mtime))
        return path

    def run_hook(self, data, raw=None, project_env=True, proc_cwd=None):
        """Run the hook and return its decoded reason, or None when it printed nothing."""
        env = dict(os.environ)
        if project_env:
            env["CLAUDE_PROJECT_DIR"] = str(self.root)
        else:
            env.pop("CLAUDE_PROJECT_DIR", None)
        text = raw if raw is not None else json.dumps(data)
        done = subprocess.run([sys.executable, str(HOOK)], input=text, capture_output=True,
                              text=True, check=False, cwd=proc_cwd or str(self.root), env=env)
        self.assertEqual(done.returncode, 0, done.stderr)
        if not done.stdout.strip():
            return None
        decision = json.loads(done.stdout)
        self.assertEqual(decision["decision"], "block")
        return decision["reason"]


class ReportModeTest(HookTest):
    """packet-reviewer: a return line plus a fresh report under the brief's path."""

    def brief_transcript(self, final_text=None, brief=BRIEF):
        records = [user(brief)]
        if final_text is not None:
            records.append(assistant(final_text))
        return self.transcript(*records)

    def test_no_line_blocks_with_a(self):
        self.brief_transcript()
        reason = self.run_hook(self.payload(final="Reviewed. All fine."))
        self.assertIn("no return line", reason)
        self.assertIn(".claude/reports/ht/HT-01.md", reason)

    def test_good_line_with_fresh_report_passes(self):
        self.brief_transcript()
        self.report()
        self.assertIsNone(self.run_hook(self.payload(final=GOOD)))

    def test_good_line_with_missing_report_blocks_c(self):
        self.brief_transcript()
        reason = self.run_hook(self.payload(final=GOOD))
        self.assertIn("is missing, empty or older than this run", reason)

    def test_report_older_than_the_run_blocks_c(self):
        self.brief_transcript()
        self.report(mtime=OLD)
        reason = self.run_hook(self.payload(final=GOOD))
        self.assertIn("older than this run", reason)

    def test_line_naming_another_path_blocks_b(self):
        self.brief_transcript()
        self.report()
        reason = self.run_hook(self.payload(final="HT-01 done abc1234 .claude/reports/ht/OTHER.md"))
        self.assertIn("your return line names .claude/reports/ht/OTHER.md", reason)

    def test_stop_hook_active_passes(self):
        self.brief_transcript()
        self.assertIsNone(self.run_hook(self.payload(final="No line.", stop_hook_active=True)))

    def test_brief_without_report_path_passes(self):
        self.brief_transcript(brief="Review HT-01 and reply.")
        self.assertIsNone(self.run_hook(self.payload(final="No line.")))

    def test_reads_last_assistant_text_when_message_is_absent(self):
        self.brief_transcript(final_text=GOOD)
        self.report()
        self.assertIsNone(self.run_hook(self.payload()))
        self.brief_transcript(final_text="Prose only.")
        self.assertIn("no return line", self.run_hook(self.payload()))

    def test_derives_transcript_path_from_transcript_path(self):
        self.brief_transcript()
        reason = self.run_hook(self.payload(final=GOOD))
        self.assertIn("is missing, empty or older than this run", reason)

    def test_backticked_and_buried_lines_pass(self):
        self.brief_transcript()
        self.report()
        self.assertIsNone(self.run_hook(self.payload(final=f"`{GOOD}`")))
        self.assertIsNone(self.run_hook(self.payload(final=f"Summary of the review.\n{GOOD}\nThanks.")))

    def test_relative_brief_path_resolves_against_cwd(self):
        self.brief_transcript()
        self.report()
        self.assertIsNone(self.run_hook(self.payload(final=GOOD), project_env=False,
                                           proc_cwd=tempfile.gettempdir()))

    def test_explicit_agent_transcript_path_is_used(self):
        self.brief_transcript()
        explicit = self.root / "elsewhere.jsonl"
        explicit.write_text(json.dumps(user(BRIEF)) + "\n", encoding="utf-8")
        data = self.payload(final=GOOD, agent_transcript_path=str(explicit))
        data["transcript_path"] = str(self.root / "nonexistent.jsonl")
        self.assertIn("is missing, empty or older than this run", self.run_hook(data))

    def test_marker_in_read_result_is_not_a_cap(self):
        quote = '    54\tCAP_MARKER = "Context cap reached"      # the deny text'
        read = {"type": "user", "timestamp": now_iso(), "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "t2", "content": quote}]}}
        self.transcript(user(BRIEF), read, assistant("Stopped."))
        self.assertIn("has no return line", self.run_hook(self.payload(final="Stopped.")))

    def test_pass_two_names_old_and_new_report(self):
        brief = "Pass 2. Old: .claude/reports/ht/old.md. New: .claude/reports/ht/new.md."
        self.brief_transcript(brief=brief)
        self.report("ht/old.md", mtime=OLD)
        self.report("ht/new.md")
        new_line = "HT-01 done abc1234 .claude/reports/ht/new.md"
        self.assertIsNone(self.run_hook(self.payload(final=new_line)))
        (self.root / ".claude" / "reports" / "ht" / "new.md").unlink()
        self.assertIn("ht/new.md", self.run_hook(self.payload(final="No line.")))


class LineModeTest(HookTest):
    """packet-coder (haiku): one return line, and no report file."""

    def test_prose_blocks_l_without_naming_the_brief_path(self):
        self.transcript(user(BRIEF), assistant("Packet done, all checks ran."))
        reason = self.run_hook(self.payload(agent="packet-coder", final="Packet done, all checks ran."))
        self.assertIn("Write no file", reason)
        self.assertNotIn(".claude/reports", reason)

    def test_line_with_stray_report_path_and_no_file_passes(self):
        self.transcript(user(BRIEF))
        self.assertIsNone(self.run_hook(self.payload(agent="packet-coder", final="HT-01 done abc1234 -")))

    def test_context_line_passes(self):
        self.transcript(user(BRIEF))
        final = "HT-01 blocked - - reason=context"
        self.assertIsNone(self.run_hook(self.payload(agent="packet-coder", final=final)))

    def test_context_cap_result_passes_prose(self):
        cap = dict(CAP_RESULT, timestamp=now_iso())
        self.transcript(user(BRIEF), cap, assistant("Stopped."))
        self.assertIsNone(self.run_hook(self.payload(agent="packet-coder", final="Stopped.")))

    def test_marker_in_brief_is_not_a_cap(self):
        self.transcript(user(BRIEF + " At `Context cap reached`, call no tool."))
        reason = self.run_hook(self.payload(agent="packet-coder", final="Stopped."))
        self.assertIn("Write no file", reason)

    def test_haiku_model_string_is_line_mode(self):
        self.transcript(user(BRIEF))
        reason = self.run_hook(self.payload(agent="packet-fixer", final="Prose."))
        self.assertIn("Write no file", reason)


class EveryAgentTest(HookTest):
    """Rules that hold for every agent, reporter or not."""

    def test_cap_result_passes_a_reviewer_with_no_line(self):
        self.transcript(user(BRIEF), dict(CAP_RESULT, timestamp=now_iso()), assistant("Stopped."))
        self.assertIsNone(self.run_hook(self.payload(final="Stopped.")))

    def test_cap_deny_as_text_blocks_passes(self):
        cap = dict(CAP_RESULT, timestamp=now_iso())
        cap["message"] = {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "t1", "is_error": True,
             "content": [{"type": "text", "text": DENY_TEXT}]}]}
        self.transcript(user(BRIEF), cap, assistant("Stopped."))
        self.assertIsNone(self.run_hook(self.payload(final="Stopped.")))

    def test_agent_without_a_file_passes(self):
        self.transcript(user(BRIEF))
        self.assertIsNone(self.run_hook(self.payload(agent="Explore", final="Found it.")))

    def test_malformed_stdin_exits_zero_silently(self):
        self.assertIsNone(self.run_hook({}, raw="{not json"))


if __name__ == "__main__":
    unittest.main()
