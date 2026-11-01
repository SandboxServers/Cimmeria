# AX-12 report
Done. Verdict reading in the UAT runner.
- New uat/runner/verdict.rs (of_outcome, settle) and verdict_tests.rs (5 tests per packet table).
- RowCtx.inconclusive added; literals updated in row.rs and capture_tests.rs (2; needed to compile, not named in packet).
- actions.rs: 7 lines after record_outcome. row.rs: settle after mark_unverified.
- Deviation: tests use a local run helper (tests.rs run_one is private, tests.rs takes no edits). The "no passing clause" test uses a passing `required = false` clause (a row needs >= 1 clause; a failing required clause would grade FAIL, not UNVERIFIED).
- Teardown denied still fails that action (grade only flags it); only inconclusive is ignored for teardown.
- Revert proofs by reasoning, not executed. Checks: L1 clippy clean, L2 nextest 1640 passed.
- No tool added (R9 n/a).