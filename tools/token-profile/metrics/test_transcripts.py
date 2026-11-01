"""The transcript reader (C1) and the harness CLI wiring (C2), on synthetic transcripts in a temp folder."""

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path

from metrics import cli, harness_cli
from metrics.transcript_fixtures import FIXTURE_PREFIX, write_agent_file, write_subagent
from metrics.transcripts import (CAP_RE, Dispatch, ToolUse, brief_report_paths, cap_denied, is_context_stop,
                                 iter_dispatches, norm_path, read_dispatch, records, report_suffix,
                                 report_writer_types, report_written, return_line)

REPO = Path(__file__).resolve().parents[3]
REPORT = "C:\\repo\\.claude\\reports\\ht-01\\HT-01.md"
DENY = ("PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. "
        "Do not call any more tools.")


def _dispatch(final_text="", tool_uses=(), results=None):
    """A dispatch built in memory, with the given final text, tool uses and results."""
    return Dispatch("s1", "a1", "packet-coder", "2026-10-10T10:00:00Z", "", final_text, [], list(tool_uses),
                    results or {})


class ReturnLineTest(unittest.TestCase):
    def test_all_six_kinds(self):
        cases = [
            ("HT-01 done abc1234 .claude/reports/ht-01/HT-01.md", "strict"),
            ("`HT-01 done abc1234 -`", "strict"),
            ("HT-01 blocked - - reason=context", "strict"),
            ("HT-01 done abc1234 -\n\nMore notes after the line.", "line_first"),
            ("Notes first.\nHT-01 done abc1234 -", "buried"),
            ("x " * 150, "prose"),
            ("Something went wrong.", "short"),
            ("", "empty"),
        ]
        for text, kind in cases:
            with self.subTest(text=text[:30]):
                self.assertEqual(return_line(text)[0], kind)

    def test_reason_suffix_and_dash_report(self):
        kind, m = return_line("HT-01 blocked - - reason=context")
        self.assertEqual((kind, m.group("status"), m.group("report")), ("strict", "blocked", "-"))
        self.assertEqual(m.group("extra"), " reason=context")


class BriefPathTest(unittest.TestCase):
    def test_absolute_and_relative_paths(self):
        brief = f"Write to {REPORT} and also .claude/reports/ht-01/HT-01.md."
        paths = brief_report_paths(brief)
        self.assertEqual(len(paths), 2)
        self.assertTrue(paths[0].endswith("HT-01.md") and paths[0].startswith("C:"))
        self.assertEqual(paths[1], ".claude/reports/ht-01/HT-01.md")


class ReportWrittenTest(unittest.TestCase):
    def test_write_edit_and_powershell(self):
        paths = [REPORT]
        write = ToolUse("t1", "Write", {"file_path": REPORT}, 0)
        edit = ToolUse("t2", "Edit", {"file_path": REPORT}, 0)
        ps = ToolUse("t3", "PowerShell", {"command": f"[IO.File]::WriteAllText('{REPORT}', $text)"}, 0)
        self.assertTrue(report_written(_dispatch(tool_uses=[write]), paths))
        self.assertTrue(report_written(_dispatch(tool_uses=[edit]), paths))
        self.assertTrue(report_written(_dispatch(tool_uses=[ps]), paths))

    def test_other_file_is_not_the_report(self):
        other = ToolUse("t1", "Write", {"file_path": "C:\\repo\\notes.md"}, 0)
        read = ToolUse("t2", "PowerShell", {"command": "Get-Content HT-01.md"}, 0)
        self.assertFalse(report_written(_dispatch(tool_uses=[other, read]), [REPORT]))

    def test_suffix_and_norm(self):
        self.assertEqual(report_suffix("'C:\\a\\.Claude\\Reports\\x\\Y.md'"), ".claude/reports/x/y.md")
        self.assertEqual(report_suffix("C:\\a\\b.md"), "")
        self.assertEqual(norm_path('"A\\B"'), "a/b")


class ContextStopTest(unittest.TestCase):
    def test_reason_context_line(self):
        self.assertTrue(is_context_stop(_dispatch("HT-01 blocked - - reason=context")))

    def test_prose_blocked_and_context(self):
        self.assertTrue(is_context_stop(_dispatch("I am blocked, and the context window is full.")))

    def test_cap_denial(self):
        d = _dispatch("", [ToolUse("t1", "Edit", {}, 0)], {"t1": DENY})
        self.assertEqual(len(cap_denied(d)), 1)
        self.assertTrue(is_context_stop(d))

    def test_marker_quoted_in_read_is_not_a_cap(self):
        read = ToolUse("t1", "Read", {}, 0)
        grep = ToolUse("t2", "Grep", {}, 1)
        results = {"t1": '    54\tCAP_MARKER = "Context cap reached"      # the deny text',
                   "t2": "see Context cap reached in the doc"}
        d = _dispatch("HT-01 done abc1234 -", [read, grep], results)
        self.assertEqual(cap_denied(d), [])
        self.assertFalse(is_context_stop(d))

    def test_cap_deny_as_text_blocks(self):
        def rec(kind, content):
            return json.dumps({"type": kind, "timestamp": "2026-10-10T10:00:00Z",
                               "message": {"role": kind, "content": content}})
        records_ = [
            rec("user", "brief"),
            rec("assistant", [{"type": "tool_use", "id": "t1", "name": "Edit", "input": {}}]),
            rec("user", [{"type": "tool_result", "tool_use_id": "t1", "is_error": True,
                          "content": [{"type": "text", "text": DENY}]}]),
            rec("assistant", [{"type": "text", "text": "Stopped."}]),
        ]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "agent-a1.jsonl"
            path.write_text("\n".join(records_) + "\n", encoding="utf-8")
            d = read_dispatch(path, "s1", "a1")
        self.assertEqual(len(cap_denied(d)), 1)

    @unittest.skipUnless((REPO / ".claude/hooks/report_contract.py").is_file(), "the hook is not in this tree")
    def test_cap_re_matches_the_hook(self):
        spec = importlib.util.spec_from_file_location("report_contract", REPO / ".claude/hooks/report_contract.py")
        hook = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(hook)
        self.assertEqual(hook.CAP_RE.pattern, CAP_RE.pattern)

    def test_done_is_not_a_stop(self):
        self.assertFalse(is_context_stop(_dispatch("HT-01 done abc1234 -")))


class ReportWritersTest(unittest.TestCase):
    def test_sonnet_with_reporting_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_agent_file(tmp, "sonnet-coder", "claude-sonnet-5-5", reporting=True)
            write_agent_file(tmp, "haiku-reader", "claude-haiku-5-5", reporting=True)
            write_agent_file(tmp, "quiet-coder", "claude-sonnet-5-5", reporting=False)
            self.assertEqual(report_writer_types(tmp), {"sonnet-coder"})


class IterDispatchTest(unittest.TestCase):
    def test_window_and_agent_type(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_subagent(tmp, "sess1", "a1", "packet-coder", "brief one",
                           [("Read", {"file_path": "x"}, "ok", 1000)], "HT-01 done abc1234 -",
                           ts="2026-10-09T10:00:00Z")
            write_subagent(tmp, "sess1", "a2", "packet-reviewer", "brief two", [], "done",
                           ts="2026-10-10T10:00:00Z")
            write_subagent(tmp, "sess1", "a3", "packet-coder", "brief three", [], "done",
                           ts="2026-10-11T10:00:00Z")
            got = list(iter_dispatches(tmp, FIXTURE_PREFIX, since="2026-10-10", until="2026-10-11"))
            self.assertEqual([d.agent_id for d in got], ["a2"])
            self.assertEqual(got[0].agent_type, "packet-reviewer")
            self.assertEqual(len(list(iter_dispatches(tmp, FIXTURE_PREFIX))), 3)

    def test_brief_tool_uses_and_results(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = write_subagent(tmp, "sess1", "a1", "packet-coder", f"Report: {REPORT}",
                                  [("Write", {"file_path": REPORT}, "wrote", 500)], "HT-01 done abc1234 -")
            d = read_dispatch(path, "sess1", "a1")
            self.assertEqual(d.brief, f"Report: {REPORT}")
            self.assertEqual(d.final_text, "HT-01 done abc1234 -")
            self.assertEqual([(t.name, t.request_index) for t in d.tool_uses], [("Write", 0)])
            self.assertEqual(d.results, {"toolu_a1_0": "wrote"})
            self.assertTrue(report_written(d, brief_report_paths(d.brief)))


class UsageTest(unittest.TestCase):
    def test_usage_is_max_per_message_id(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "sess" / "subagents" / "agent-x.jsonl"
            path.parent.mkdir(parents=True)
            lines = [
                {"type": "user", "timestamp": "2026-10-10T10:00:00Z", "message": {"content": "brief"}},
                {"type": "assistant", "timestamp": "2026-10-10T10:00:01Z", "message": {"id": "m1", "content": [
                    {"type": "text", "text": "thinking"}], "usage": {"input_tokens": 1000, "output_tokens": 5,
                                                                    "cache_read_input_tokens": 4000}}},
                {"type": "assistant", "timestamp": "2026-10-10T10:00:02Z", "message": {"id": "m1", "content": [
                    {"type": "text", "text": "thinking"}], "usage": {"input_tokens": 1000, "output_tokens": 80,
                                                                    "cache_read_input_tokens": 4000}}},
                {"type": "assistant", "timestamp": "2026-10-10T10:00:03Z", "message": {"id": "m2", "content": [
                    {"type": "text", "text": "done"}], "usage": {"input_tokens": 50, "output_tokens": 10}}},
            ]
            path.write_text("".join(json.dumps(r) + "\n" for r in lines) + "not json\n", encoding="utf-8")
            self.assertEqual(len(list(records(path))), 4)
            d = read_dispatch(path, "sess", "x")
            self.assertIsNone(d.agent_type)
            self.assertEqual(len(d.requests), 2)
            self.assertEqual((d.requests[0].output, d.requests[0].context), (80, 5000))
            self.assertEqual(d.requests[1].context, 50)
            self.assertEqual(d.final_text, "done")


class CliWiringTest(unittest.TestCase):
    def test_harness_commands_reach_the_stubs(self):
        # resume-cards is implemented (HT-04), so it no longer reaches a stub; test_resume_cards.py covers it
        for name in (n for n in harness_cli.NAMES if n != "resume-cards"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                err = io.StringIO()
                with contextlib.redirect_stderr(err):
                    code = cli.main([name, "--out", tmp])
                self.assertEqual(code, 2)
                self.assertIn("not-implemented", err.getvalue())

    def test_repo_root_and_registry_still_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            self.assertEqual(harness_cli.repo_root(base / ".claude" / "worktrees" / "ht-01"), base)
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = cli.main(["registry", "--repo", str(REPO)])
        self.assertEqual(code, 0)
        self.assertTrue(out.getvalue().startswith("{"))


if __name__ == "__main__":
    unittest.main()
