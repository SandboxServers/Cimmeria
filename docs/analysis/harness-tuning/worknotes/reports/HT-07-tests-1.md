# HT-07 tests audit, pass 1

Range e1de6742f..799a7a2fe, worktree ht-07.

## Findings

- T1. Revert proof run (read-only): the base-commit `orchestrator.md` and `packet-coder.md` were copied into a scratch tree beside the new test file and the suite was run there. Result: `Ran 4 tests, FAILED (failures=15)`, all 4 tests fail. Against HEAD: OK. So the tests are not theatre; they fail on revert.
- T2. Of the 17 possible assertions, 15 fail on the base. Two pass on base: `(part N)` (the base already had it in the context bullet) and `write your report` (never present). Both are as the spec lists them; neither is a revert guard. Harmless.
- T3. The spec's checks all pass at HEAD: new test OK, `sync.py --check` says the mirror is current.
- T4. Weakness (minor, spec-conformant): `<commit-sha|-> -` is checked on the whole file, not inside `## Reporting` as the spec text says. Base fails it anyway (base has `<commit-sha|-> <report-path>`), so the guard holds for this revert; it would not catch the phrase moving out of Reporting.
- T5. Not pinned by any test (and not in the spec's test list): the `- On a one-line return` bullet routing by `reason=`, and the `.codex` toml copies (covered only by `sync.py --check`).

## Fix notes

None required. Optional: T4 slice the `## Reporting` section before asserting; T5 assert `reason=` in the orchestrator's one-line-return bullet.

## Not checked

Prose quality of the agent edits against the 11 spec steps beyond the diff skim; `test_platform_rule.py` and the full `test_*.py` discovery were not run; no cargo.

## Verdict

ship
