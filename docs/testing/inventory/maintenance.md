# Maintaining the test inventory

> **Type**: how-to  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Index**: [README](README.md)

## When to update

A PR that adds or removes **≥5% of the workspace test count** (~68 tests against the current 1351 baseline) updates the matching per-crate file under `docs/testing/inventory/` and the totals in [README.md](README.md) in the same PR. Smaller drifts get folded in by periodic sweep updates — running the regenerate scripts below every few weeks is cheaper than reviewing inventory churn on every PR. Renamed a test? Pick it up in the next sweep unless the PR is already in the ≥5% bucket.

Inventory drift up to ~5% is acceptable between sweeps — see [.github/copilot-instructions.md](../../../.github/copilot-instructions.md) and [CLAUDE.md](../../../CLAUDE.md) for the doc-update map. A CI drift-check is **planned but not yet implemented**; for now, reviewers eyeball the diff against the threshold.

## How to regenerate

The pre-extraction script lives at `docs/testing/.scratch/extract_tests.py`. It walks `crates/`, `tools/`, `src-tauri/`, and `fuzz/` for `.rs` files, extracts every `#[test]` / `#[tokio::test]` / `#[rstest]` / `#[test_case(...)]` function, runs `git log -S 'fn <name>' -- <file>` per test for first-commit dates, and writes `inventory.json` + `summary.json` next to itself.

```bash
python docs/testing/.scratch/extract_tests.py
```

Then regenerate the markdown:

```bash
python docs/testing/.scratch/generate_inventory.py
```

Diff the output against `git status` to see which crate files actually moved — most PRs only touch one or two.

## Conventions

- Test rows are sorted by `(file path, line)`. Keep them sorted when hand-editing.
- The `Test` column links use `../../../` to climb out of `docs/testing/inventory/` to the repo root, then `<file>#L<line>` (GFM/VS Code line anchor). For a multi-line range use `#L<start>-L<end>`.
- Pick `Kind` from the 7-type taxonomy in [TESTING.md](../../../TESTING.md): `unit` / `wire-format` / `live-DB` / `smoke` / `concurrency` / `chain-replay` / `legacy-reference`. The codebase also has `proptest` (in `mercury/`) and may grow `rstest` and `integration` rows; those are listed too.
- `What it tests` should be one short sentence. Prefer the test's `///` doc comment verbatim. If the body is opaque and the function name doesn't tell you, write `(infer from body)` rather than fabricating intent.
- `Notes` is empty unless something is unusual: `#[ignore]`, parameterization, or a smell signal from the extractor.

## Scratch directory

`docs/testing/.scratch/` is gitignored (see the repo `.gitignore`). It holds the extractor script and its JSON output — the inventory's regeneration source. Don't delete it; future PRs will re-run the extractor here.
