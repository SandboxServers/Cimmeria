# HT-08 tests review 1

Range e1de6742f..690f3c499. Method: ran `ResumeCardTest` (unchanged test file) against the old hook and against 19 single-line mutants of the new hook (scratch copy, nothing in the worktree edited). `test_hooks.py` 39 tests pass; `sync.py --check` clean; hook is 176 lines.

## Findings

- T1 (revert proof holds). With the pre-HT-08 hook swapped in, 7 tests fail: F7 case 1 (`test_no_card_when_the_session_never_wrote_the_pointer`), cases 2, 3, 4 (Active-campaign cards), inferred card, BOM, truncation (old hook cannot read the pointer from a transcript). The F7 bug shape is pinned. Not theatre.
- T2 (weak: case 8, RESUME.md existence filter). `test_prompt_naming_a_campaign_without_resume_does_not_count` (test_hooks.py:137) survives deleting the filter in `tier2` (`names = set(found)`): ghost then wins 3 of 4, but `card_for` -> `resume_file` returns None, so the test still sees no card. The filter itself is unpinned. Fix: 3 prompts naming `demo` plus 3 naming `ghost` (both ways it should produce a `demo` card with the filter; without the filter ghost+demo split 3/6 and no card). Assert the card names `demo`.
- T3 (boundary gaps, tier 2). Survivors: `len(named) <= 3` (naming == 3 exactly is never tested; the 3-of-4 case has naming 4) and `3*c > 2*n` (the exact 2-of-3 boundary is never tested). Fix: add 3-of-3 (card) and 2-of-3 (card) cases.
- T4 (minor survivors, tier 1). `MultiEdit` support (`edits[-1]` and the tool name) has no test; `.endswith("active-campaign")` loosened (a file like `x/foo-active-campaign` would count) is not rejected; first-non-empty-line vs joined lines survives (no multi-line pointer content). Fix: one MultiEdit case, one decoy-path Write (`state/other/active-campaign` must not count), one `"\n\ndemo\nextra\n"` content case.
- T5 (minor survivors, tier 2). The 40-prompt window upper bound (`LAST_PROMPTS = 1000`) survives (no test with >40 prompts where old prompts differ); `.lower()` of the prompt survives; the `.claude/reports/` regex is only pinned together with the docs regex (dropping the reports regex alone survives). Fix: 41+ prompts with the first one naming another campaign; a mixed-case path; a case where only reports paths carry the 3 votes.
- T6 (vacuous against revert, but covered by mutants). Case 5 (empty write), case 9 path rejection and case 10 missing transcript all pass on the old hook (no global file in those fixtures). They are real guards only because of the mutants they kill (`written is not None` -> truthy; `valid_name` removed); keep them.
- T7 (scope, nit). Packet said edit `ResumeCardTest` only; the diff also adds module-level helpers `tool_record` and `prompt_record` (test_hooks.py:40-48), which the class needs. Acceptable; note for the ledger.

Strong: the F7 test, "later write wins" (kills first-write), Edit `new_string` (kills `old_string`), empty-write-skips-tier2, 2-of-4 and 2-of-2 negatives (kill ratio and `naming>=2` mutants), BOM and 40-line truncation exact (`row 040` present, `row 041` absent).

## Fix notes

1. (T2) Rewrite case 8 as 3x demo + 3x ghost expecting a `demo` inferred card.
2. (T3) Add 3-of-3 and 2-of-3 tier-2 card cases.
3. (T4/T5, optional) Add the MultiEdit, decoy-path, mixed-case, >40-prompt cases.

## Not checked

- NOT ASSESSED - whether the real transcript JSONL shape (tool_use block layout, MultiEdit `edits`) matches the fixtures - only fixtures were read, no real transcript.
- NOT ASSESSED - orchestrator.md wording vs C5 text beyond a diff read; toml mirror verified only via `sync.py --check`.
- NOT ASSESSED - the 200 MB seek path in `matching_lines` (no test, not mutated).
- NOT ASSESSED - CI run; only local python.

## Verdict

fix (T2 is a test that does not guard the behavior it names; T3 is cheap to add). Core revert proof is sound.
