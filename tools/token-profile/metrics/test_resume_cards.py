"""The resume-cards subcommand (C2): verdicts, tiers and the two dispatch counts, on synthetic main transcripts."""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from metrics import resume_cards
from metrics.transcript_fixtures import FIXTURE_PREFIX, write_main

TS = "2026-10-10T09:00:00Z"


def _boundary(ts=TS):
    """A compaction record."""
    return {"type": "system", "subtype": "compact_boundary", "timestamp": ts}


def _agent(i, prompt):
    """An assistant turn that dispatches one Agent with `prompt`."""
    return {"type": "assistant", "timestamp": TS, "message": {"id": f"agent-{i}", "role": "assistant", "content": [
        {"type": "tool_use", "id": f"toolu_{i}", "name": "Agent", "input": {"prompt": prompt}}]}}


def _card(text, ts=TS):
    """An attachment record that carries a resume card."""
    return {"type": "attachment", "timestamp": ts, "attachment": {"type": "text", "content": text}}


def _after():
    """The assistant turn that follows the compaction and its card."""
    return {"type": "assistant", "timestamp": TS, "message": {"id": "after", "role": "assistant", "content": [
        {"type": "text", "text": "Resumed."}]}}


def _session(root, session_id, prompts, card=None):
    """A main session: one Agent turn per prompt, then a compaction with `card` (if any), then an assistant turn."""
    recs = [_agent(i, p) for i, p in enumerate(prompts)] + [_boundary()]
    if card is not None:
        recs.append(_card(card))
    recs.append(_after())
    write_main(root, session_id, recs)


def _run(root, since=None, until=None):
    """Run the subcommand over `root`. Returns (status code, stdout, the written JSON or None)."""
    with tempfile.TemporaryDirectory() as out:
        args = SimpleNamespace(projects_root=root, prefix=FIXTURE_PREFIX, since=since, until=until, out=out)
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(io.StringIO()):
            code = resume_cards.run(args)
        data = json.loads(Path(out, "resume-cards.json").read_text(encoding="utf-8")) if code == 0 else None
    return code, stdout.getvalue(), data


ALPHA = "Work on docs/analysis/alpha/ now."
BETA = "Work on docs/analysis/beta/ now."


class ResumeCardsTest(unittest.TestCase):
    def test_card_naming_another_campaign_is_wrong(self):
        with tempfile.TemporaryDirectory() as tmp:
            _session(tmp, "s1", [ALPHA] * 4, card="Active campaign: beta.")
            code, out, data = _run(tmp)
            self.assertEqual(code, 0)
            self.assertEqual((data["compactions"], data["firings"], data["wrong"]), (1, 1, 1))
            self.assertEqual(data["by_tier"]["active"]["wrong"], 1)
            self.assertIn("status=ok resume-cards compactions=1 firings=1 wrong=1 wrong_pct=100.0", out)

    def test_card_naming_the_top_campaign_is_matched(self):
        with tempfile.TemporaryDirectory() as tmp:
            _session(tmp, "s1", [ALPHA] * 4, card="Active campaign: alpha.")
            _, _, data = _run(tmp)
            self.assertEqual((data["firings"], data["matched"], data["wrong"]), (1, 1, 0))

    def test_split_prompts_are_mixed(self):
        with tempfile.TemporaryDirectory() as tmp:
            _session(tmp, "s1", [ALPHA, ALPHA, BETA, BETA], card="Active campaign: alpha.")
            _, _, data = _run(tmp)
            self.assertEqual((data["mixed"], data["matched"], data["wrong"]), (1, 0, 0))

    def test_one_prompt_is_undetermined(self):
        with tempfile.TemporaryDirectory() as tmp:
            _session(tmp, "s1", [ALPHA], card="Active campaign: beta.")
            _, _, data = _run(tmp)
            self.assertEqual((data["firings"], data["undetermined"], data["wrong"]), (1, 1, 0))

    def test_inferred_card_lands_in_inferred_tier(self):
        with tempfile.TemporaryDirectory() as tmp:
            _session(tmp, "s1", [ALPHA] * 4, card="Inferred campaign (from this session's dispatches): alpha.")
            _, _, data = _run(tmp)
            self.assertEqual(data["by_tier"]["inferred"]["matched"], 1)
            self.assertEqual(data["by_tier"]["active"]["firings"], 0)
            self.assertEqual(data["matched"], 1)

    def test_resume_read_before_dispatch_is_counted(self):
        with tempfile.TemporaryDirectory() as tmp:
            read = {"type": "assistant", "timestamp": TS, "message": {"id": "read", "role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_read", "name": "Read",
                 "input": {"file_path": "C:/repo/docs/analysis/alpha/RESUME.md"}}]}}
            _session_records = [_boundary(), read, _agent(0, ALPHA)]
            write_main(tmp, "s1", _session_records)
            _, _, data = _run(tmp)
            self.assertEqual(data["read_resume_before_dispatch"], 1)
            self.assertEqual(data["compactions_followed_by_dispatch"], 1)

    def test_boundary_outside_the_window_is_ignored(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_main(tmp, "s1", [_boundary("2026-09-01T09:00:00Z"), _card("Active campaign: beta."), _after()])
            code, out, data = _run(tmp, since="2026-10-01", until="2026-11-01")
            self.assertEqual(code, 0)
            self.assertEqual((data["compactions"], data["firings"]), (0, 0))
            self.assertIn("compactions=0 firings=0 wrong=0 wrong_pct=0.0", out)


if __name__ == "__main__":
    unittest.main()
