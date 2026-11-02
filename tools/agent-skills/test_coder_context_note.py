"""Tests for the PostToolUse commit-before-cap note in .claude/hooks/coder_context_note.py."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HOOK = REPO / ".claude/hooks/coder_context_note.py"
AGENT = REPO / ".claude/agents/packet-coder.md"


def _transcript(tmp, contexts):
    """Write the subagent transcript of one dispatch; each item is an int or an (id, n) pair."""
    path = Path(tmp) / "sess" / "subagents" / "agent-a1.jsonl"
    path.parent.mkdir(parents=True)
    lines = []
    for index, item in enumerate(contexts):
        message_id, size = item if isinstance(item, tuple) else (f"m{index}", item)
        record = {"type": "assistant", "message": {"id": message_id, "model": "claude-haiku-x", "usage": {
            "input_tokens": size, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0,
            "output_tokens": 10}, "content": []}}
        lines.append(json.dumps(record))
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return Path(tmp) / "sess.jsonl"


def _run(payload, env=None):
    """Run the hook with the payload on stdin; return its completed process."""
    return subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                          capture_output=True, text=True, check=False, env=env)


class CoderContextNoteTest(unittest.TestCase):
    """The note fires once, on the first request past 75k and below the cap, for packet-coder only."""

    def _payload(self, tmp, contexts, **extra):
        """A PostToolUse payload for a packet-coder dispatch holding `contexts`."""
        main = _transcript(tmp, contexts)
        payload = {"hook_event_name": "PostToolUse", "agent_id": "a1", "agent_type": "packet-coder",
                   "transcript_path": str(main), "tool_name": "Read"}
        payload.update(extra)
        return payload

    def _note(self, contexts, **extra):
        """The parsed hook output for one dispatch, or None when the hook prints nothing."""
        with tempfile.TemporaryDirectory() as tmp:
            result = _run(self._payload(tmp, contexts, **extra))
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout) if result.stdout.strip() else None

    def test_first_request_past_threshold_gets_note(self):
        """Case 1: [70k, 76k] gives a PostToolUse note that names the size and the (part N) commit."""
        out = self._note([70_000, 76_000])
        hook = out["hookSpecificOutput"]
        self.assertEqual(hook["hookEventName"], "PostToolUse")
        self.assertTrue(hook["additionalContext"].startswith("coder_context_note:"))
        self.assertIn("76,000", hook["additionalContext"])
        self.assertIn("(part N)", hook["additionalContext"])

    def test_already_past_threshold_gets_nothing(self):
        """Case 2: [76k, 80k] fired on an earlier request, so nothing."""
        self.assertIsNone(self._note([76_000, 80_000]))

    def test_below_threshold_gets_nothing(self):
        """Case 3: [70k, 74k] stays under 75k."""
        self.assertIsNone(self._note([70_000, 74_000]))

    def test_past_cap_gets_nothing(self):
        """Case 4: [70k, 91k] crosses the threshold but is past the cap."""
        self.assertIsNone(self._note([70_000, 91_000]))

    def test_other_agent_gets_nothing(self):
        """Case 5: the same transcript for packet-reviewer gets no note."""
        self.assertIsNone(self._note([70_000, 76_000], agent_type="packet-reviewer"))

    def test_streaming_uses_max_per_request_id(self):
        """Case 6: the max per id counts, so [m1 70k, m2 74k, m2 76k] fires with 76k as the last."""
        out = self._note([("m1", 70_000), ("m2", 74_000), ("m2", 76_000)])
        self.assertIn("76,000", out["hookSpecificOutput"]["additionalContext"])

    def test_main_transcript_without_agent_id_gets_nothing(self):
        """Case 7: no agent_id and no subagent transcript, so the main session never gets the note."""
        with tempfile.TemporaryDirectory() as tmp:
            main = _transcript(tmp, [70_000, 76_000])
            payload = {"hook_event_name": "PostToolUse", "agent_type": "packet-coder",
                       "transcript_path": str(main), "tool_name": "Read"}
            result = _run(payload)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_not_json_fails_open(self):
        """Case 8: stdin that is not JSON exits 0 with no output."""
        result = subprocess.run([sys.executable, str(HOOK)], input="not json",
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_bad_cap_env_fails_open(self):
        """Case 8b: a non-number HAIKU_CONTEXT_CAP exits 0 with no output instead of a traceback."""
        result = _run({}, env={**os.environ, "HAIKU_CONTEXT_CAP": "abc"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "")

    def test_note_never_denies_and_never_asks_for_report(self):
        """Case 9: the output has no permissionDecision or decision, and says write no report."""
        with tempfile.TemporaryDirectory() as tmp:
            result = _run(self._payload(tmp, [70_000, 76_000]))
        self.assertNotIn("permissionDecision", result.stdout)
        self.assertNotIn('"decision"', result.stdout)
        text = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("Write no report", text)
        self.assertNotIn("final report", text)


class PacketCoderWiringTest(unittest.TestCase):
    """The hook is wired in packet-coder's frontmatter, and the body keeps its habits and cap rule."""

    def test_frontmatter_wires_the_hook_and_body_keeps_the_rules(self):
        """Case 10: the frontmatter has the PostToolUse entry; the body has the note, the habits and the cap rule."""
        text = AGENT.read_text(encoding="utf-8")
        head, _, body = text.partition("\n---\n")
        frontmatter = head.split("\n", 1)[1] if head.startswith("---") else head
        self.assertIn("PostToolUse:", frontmatter)
        self.assertIn("coder_context_note.py", frontmatter)
        for phrase in ("coder_context_note:", "files_with_matches", "head_limit"):
            self.assertIn(phrase, body)
        for phrase in ("Context cap reached", "reason=context"):
            self.assertIn(phrase, body)


if __name__ == "__main__":
    unittest.main()
