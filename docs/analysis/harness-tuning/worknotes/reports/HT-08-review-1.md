## Verdict

ship

## Fix notes

none

## Not fixed

- `.claude/hooks/resume_card.py` (the `lstrip` call in `build_card`) and `tools/agent-skills/test_hooks.py:150` hold a literal U+FEFF character in the source. It works today (verified: exactly one U+FEFF in each file). An editor or normaliser that drops the invisible character would turn the BOM test into a pass-either-way test. `"\ufeff"` would be more robust. This is a style nit, not a defect, so it is not a fix note.
- No test covers the 200 MB tail-read branch of `matching_lines`. C5 asks for the behaviour but HT-08's test list does not, so this is out of scope.

Checks run in the worktree:
- `python -m unittest discover -s tools/agent-skills -p "test_hooks.py"`: 39 tests, OK.
- `python -m unittest discover -s tools/agent-skills -p "test_*.py"`: 98 tests, OK (1 skipped).
- `python tools/agent-skills/sync.py --check`: the mirror is current.
- `.claude/hooks/resume_card.py` is 144 lines (cap 200), and `test_hooks.py` is 326 lines.
- Line endings: the index is LF and the working tree is CRLF, the same as the other tracked files. `git diff --check` reports nothing.
- Revert proof: the old hook ignored stdin and read the pointer file, so it prints a card for case 1 and names `other` in case 2. Both tests assert otherwise. The BOM and path-name tests each fail without the matching guard.
- The orchestrator.md `## State on disk` paragraph matches the spec text, and the `.codex/agents/orchestrator.toml` mirror carries the same paragraph.