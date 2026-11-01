# HT-08 tests review 3

Fix commit 3df7b6466 (base 6e17dd290). Method: scratch copy of `.claude/hooks` and `tools/agent-skills` (worktree untouched), `ResumeCardTest` (23 tests, all pass) run against the current hook, the pre-HT-08 hook, and single-line mutants.

## Findings

- T1 (revert proof, holds). With the pre-HT-08 hook swapped in, 15 of 23 `ResumeCardTest` tests fail (13 errors, 2 failures), including the F7 case (`test_no_card_when_the_session_never_wrote_the_pointer`), last-write, three-of-three, BOM and the two new tests (reports path, leading blank lines).
- T2 (tests-2 T3 closed). `test_only_the_last_40_prompts_vote` (30 other then 40 demo) now fails under `prompts[:LAST_PROMPTS]` (FAIL: first 40 give other) and under `LAST_PROMPTS = 1000` (ERROR: no card at 40 of 70). The comment matches the fixture.
- T3 (tests-2 T4 closed). New `test_three_of_three_prompts_name_the_campaign_by_reports_path` is killed by deleting the reports regex (mutant `reports[` to `reportz[` errors it).
- T4 (tests-2 T5 closed). `test_pointer_with_leading_blank_lines_names_the_first_non_empty_line` passes and errors on the old hook.
- T5 (nit, accepted). Mutants `LAST_PROMPTS = 41` and `= 50` survive: the window edge is pinned only against 1000 and the first-40 direction. A 40-vs-50 boundary is not a realistic regression; no action required.

## Fix notes

None required. Optional only: T5.

## Not checked

- NOT ASSESSED - the real transcript JSONL shape against the fixtures.
- NOT ASSESSED - the 200 MB seek path; `sync.py --check` and a full `test_hooks.py` run in the worktree; CI.

## Verdict

ship
