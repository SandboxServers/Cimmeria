## Verdict
fix

## Fix notes
N1. tools/agent-skills/test_coder_contract.py:34 — the spec requires `<commit-sha|-> -` "in `## Reporting`", but the test asserts it against the whole file, and the context-cap bullet (`<packet> blocked <commit-sha|-> - reason=context`) also contains that text, so reverting the Reporting paragraph's line would not fail this test — in `test_packet_coder_names_the_contract`, delete the tuple entry `"<commit-sha|-> -",` (the line `            "<commit-sha|-> -",`), and after the `for` loop add, at the same indent as the `for`: `reporting = text.split("## Reporting", 1)[1]` then `self.assertIn("<commit-sha|-> -", reporting)` — verify: `python -m unittest discover -s tools/agent-skills -p "test_coder_contract.py"` passes; temporarily replacing the `## Reporting` paragraph's `<commit-sha|-> -` with `<commit-sha|-> <report-path>` makes it fail.

## Not fixed
- Commit subject is `HT-07 Haiku coder contract in agent files`, not the spec's `HT-07: packet-coder writes code and commits, returns one line, resumes from the branch`. The coder said the brief asked for this subject, so it is a brief-vs-spec difference, not a code defect. The orchestrator should settle which subject the merge uses; no fix note.
- test_coder_contract.py checks `names no report path` against the whole orchestrator.md rather than the `Every brief carries:` paragraph. It still fails on revert (the phrase is new), so no note.
- Other agents (architect.md, packet-reviewer.md, the advisors) still carry the report-path Reporting paragraph and `worknotes/` ruling paths. They are outside HT-07's file list; the skill and workflow docs follow in HT-11a.

## Not checked
- Behaviour of the user-level `Context cap reached` hook: it sits outside the repo.
- Whether a Haiku coder follows the new text: that is for the `metrics returns` re-measure.

## Checked and fine
- All 11 steps match the spec text verbatim.
- The `- Shell:` and `- Host:` bullets are unchanged.
- The agent `.md` files, the `.toml` files and the new test file are all pure CRLF.
- `python tools/agent-skills/sync.py --check` passes and the worktree is clean.
- The full agent-skills suite ran 94 tests, OK, and test_coder_contract.py ran 4, OK.
- The tests fail on revert: the old packet-coder.md has `worknotes/`, and the old orchestrator.md has `worknote path`.
