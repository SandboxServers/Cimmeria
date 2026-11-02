# HT-08 tests review 2

Fix commit 6e17dd290 (base 690f3c499). Method: scratch copy of `.claude/hooks` plus `test_hooks.py` (worktree untouched), `ResumeCardTest` run against the unchanged hook, the pre-HT-08 hook, and 19 single-line mutants of the current hook. Baseline: all `ResumeCardTest` tests pass.

## Findings

- T1 (revert proof, holds and grew). With the pre-HT-08 hook swapped in, 13 `ResumeCardTest` tests fail (was 7): F7 case, last-write, later-write-wins, Edit, inferred 3-of-4, BOM, 40-line truncation, case 8, and the new 3-of-3, 2-of-3, last-40, mixed-case and MultiEdit tests. Only `test_pointer_lookalike_path_does_not_count` and the empty/path/transcript guards pass on the old hook, as expected (see T2 of review 1: they guard by mutants).
- T2 (prior T2, T3 closed). Mutants now killed: delete the RESUME.md filter in `tier2` (killed by the rewritten case 8 alone); `len(named) <= 3` (3-of-3, 2-of-3 and case 8); `3*c > 2*n` (2-of-3 only); loosened `.endswith("active-campaign")` (decoy path); MultiEdit `edits[0]` and MultiEdit removed from `POINTER_TOOLS`; `.lower()` removed from `norm_path` (mixed-case); `LAST_PROMPTS = 1000`.
- T3 (new, weak: `test_only_the_last_40_prompts_vote` does not pin "last"). Mutant `prompts[:LAST_PROMPTS]` (the first 40 instead of the last 40) survives. Fixture: 10 other, 27 demo, 13 other. The first 40 are 10 other + 27 demo + 3 other, so demo is 27/40 and wins there too; only the 1000 window mutant dies. The comment's claim "first 10 are other" is not what the test distinguishes. Fix (tighten): make the first 10 prompts name `demo`-excluding votes that flip the result both ways, e.g. 40 prompts of `demo` last, preceded by 20 prompts of `other`: the last 40 give demo 40/40 (card), the first 40 give 20 other + 20 demo (no card), all 60 give demo 40/60 (card, so also add the 1000 guard with 25 other first: all 65 = demo 40/65 no card, last 40 card, first 40 = 25 other + 15 demo no card).
- T4 (new, reports regex still unpinned). Mutant deleting the `.claude/reports/` regex alone survives (no_docs_rx is killed). The 3-of-4 test has one reports prompt but the card still forms from the remaining 3 named prompts. The reports path is how real dispatches name a campaign (packet briefs cite `.claude/reports/<name>/`). Matters: cheap and it is a real input shape. Fix (add): 3 prompts that name `demo` only via `.claude/reports/demo/HT-01.md`, expect a `demo` inferred card.
- T5 (omitted multi-line pointer content, minor, judged as optional). Mutants `"".join(lines)` (instead of `lines[0]`) and "first raw line instead of first non-empty line" both survive. Real-world relevance is low: the pointer is a one-name file the orchestrator writes (`demo\n`); a trailing second line is not a documented shape. The leading-blank-line case (`"\n\ndemo\n"`) is plausible from an editor and is the one worth one assert, but it is not a F7 regression. I do not require it; add it only with T4 as a two-line case if the coder is already in the file.
- T6 (omitted reports-only votes). See T4: that omission does matter. The multi-line omission (T5) does not.
- T7 (nit). `test_campaign_name_is_matched_in_mixed_case` writes `.CLAUDE/State/active-campaign` while its comment says "the needle is case-sensitive": the byte-level needle `active-campaign` is lower case in both, so the test pins `norm_path` lowercasing only. Fine; the comment is accurate. The prompt `.lower()` in `tier2` (mutant `no_prompt_lower`) survives; an uppercase `Docs/Analysis/Demo/` prompt is never tested. Optional one-line case, low value (resume_file check is on lowercase names anyway).

Strong: the rewritten case 8 (unfiltered it splits 3 to 3 and shows no card, so it fails without the filter and for the right reason); 2-of-3 (sole killer of `>`); decoy-path negative (sole killer of the loosened `endswith`); MultiEdit last-edit (kills both `edits[0]` and tool-removed).

## Fix notes

1. (T3) Rework `test_only_the_last_40_prompts_vote` so the last 40, the first 40 and all prompts give three different answers (see T3 fixture); it must fail under `prompts[:LAST_PROMPTS]` and under `LAST_PROMPTS = 1000`.
2. (T4) Add one test: 3 Agent prompts naming `demo` only through `.claude/reports/demo/<file>.md`, expect `dispatches): demo.`.
3. (T5, optional) Add a pointer content `"\n\ndemo\nextra\n"` case expecting a `demo` card.

## Not checked

- NOT ASSESSED - whether the real transcript JSONL shape (tool_use layout, MultiEdit `edits`) matches the fixtures.
- NOT ASSESSED - the 200 MB seek path in `matching_lines`; `sync.py --check` and the full `test_hooks.py` run on the worktree (only `ResumeCardTest` in a scratch copy was run).
- NOT ASSESSED - CI run; only local python.

## Verdict

fix (T3: the test named "only the last 40 prompts vote" passes when the hook reads the first 40; T4: the reports regex is unpinned). The earlier T2/T3 gaps are closed and the revert proof is stronger.
