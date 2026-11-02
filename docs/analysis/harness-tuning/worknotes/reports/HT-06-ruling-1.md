# HT-06 ruling 1 applied

- Hook: CAP_RE and is_cap_deny added; read_transcript no longer matches the marker on raw lines (239 lines).
- Tests: CAP_RESULT now the real deny shape; cases 21-23 added; N1 (proc_cwd) and N2 (explicit path) applied.
- work-packets.md: C3 step 5 rewritten, test list gains 21-23 (edited in the worktree copy; the main checkout's ledger copy is not touched).
- Test audit HT-06-tests-1.md did not exist; nothing applied from it.
- Checks: test_report_contract 24 OK; all agent-skills 115 OK (1 skipped); sync.py --check current; test_harness_tables 7 OK.
- Revert proofs: raw-marker match fails 21 and 22; string-only content fails 23; ignoring agent_transcript_path fails N2 test; resolve() without cwd fails N1 test.
- Not done: C1 cap_denied containment follow-up (ruling: orchestrator's call).
