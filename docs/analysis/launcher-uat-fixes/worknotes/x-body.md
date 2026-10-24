## Summary

Close-out of the launcher-uat-fixes campaign (PR X) plus the post-merge fixes for #1988.

- LF-18 follow-up (adversarial review of #1988, merged before its fixes landed): `launcher-guide.md` no longer points at the removed log-upload storage or "three signing keys"; the unused `gethostname` dependency and its lockfile block are gone; the mid-flight LF-18 worknote is replaced by a final note.
- LF-20: the campaign folder `docs/analysis/launcher-uat-fixes/` (README with the packet status table, work packets, final RESUME, worknotes) and the laptop UAT worknote it is based on; UAT rows U1-U6 in `docs/guides/unified-uat.md`; generated blocks regenerated with `tools/docs-gen/regen.py` (no hand edits).

Campaign PRs: #1986 (U1), #1984 (U2), #1985 (U3), #1987 (U4), #1988 (U5), this PR (X).

## Tested / untested

Docs and one dependency removal; packet review loops (2 passes each, ship) and `regen.py --check` clean. The engine and shell tests and clippy/fmt/`--locked` checks passed locally for the dependency change. No live UAT has run: owner UAT U1-U6 is owed (laptop use needs the owner's OK).

## Owner risks recorded

Chat and character names leave the machine (per-word scrub only); the 2,048-byte message floor; the server drops rows over 4,096 bytes after escaping; older lines beyond 20,000 new lines of one kind per file are not uploaded; daily ceilings; secrets under 32 characters or dot-split are kept; log rows are forgeable by log-token holders; budgets are speed bumps.

## Docs

Rows touched are listed in `.claude/reports/launcher-uat-fixes/LF-20-coder.md`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NdhQoumCZtZKGCMdPDbWJF
