# Maintaining the test inventory

> **Type**: how-to  
> **Audience**: engineers  
> **Last updated**: 2026-07-25  
> **Index**: [README](README.md)

## When to update

A PR that adds or removes **≥5% of the workspace test count** (~150 tests against the current 3,012 baseline) updates the matching per-crate file under `docs/testing/inventory/` and the totals in [README.md](README.md) in the same PR. Smaller drifts get folded in by periodic sweep updates — a batched sweep every few weeks is cheaper than reviewing inventory churn on every PR. Renamed a test? Pick it up in the next sweep unless the PR is already in the ≥5% bucket. **Note that sweeps are currently manual**: the regeneration scripts are not in the repo (see [How to regenerate](#how-to-regenerate)), so the catalogue has drifted well past the ≥5% threshold — 1,351 rows against 3,012 tests as of 2026-07-25.

Inventory drift up to ~5% is acceptable between sweeps — see [.github/copilot-instructions.md](../../../.github/copilot-instructions.md) and [CLAUDE.md](../../../CLAUDE.md) for the doc-update map. A CI drift-check is **planned but not yet implemented**; for now, reviewers eyeball the diff against the threshold.

## How to regenerate

> **⚠ The regeneration procedure below cannot currently be run by anyone.**
> Neither `extract_tests.py` nor `generate_inventory.py` exists anywhere in the
> repository — a repo-wide search for both filenames returns nothing — and
> `docs/testing/.scratch/` is gitignored twice over
> ([`.gitignore:178`](../../../.gitignore) `docs/testing/.scratch/` and
> [`.gitignore:209`](../../../.gitignore) `.scratch/*`), so the scripts were
> never tracked. They survive only on whichever machine last ran a sweep, if
> at all.
>
> The procedure is kept here because it accurately records **what the last
> sweep did** and is the starting spec for re-authoring the tooling. It is not
> a recipe you can execute today. **Regenerating the inventory currently means
> writing a new extractor first.** When someone does, it should land in
> version control (`tools/` rather than `docs/testing/.scratch/`) so the next
> contributor inherits a working path and so conventions expressed in the
> generator can be enforced through review.

The pre-extraction script lived at `docs/testing/.scratch/extract_tests.py`. It walked `crates/`, `tools/`, `src-tauri/`, and `fuzz/` for `.rs` files, extracted every `#[test]` / `#[tokio::test]` / `#[rstest]` / `#[test_case(...)]` function, ran `git log -S 'fn <name>' -- <file>` per test for first-commit dates, and wrote `inventory.json` + `summary.json` next to itself.

```bash
# Historical — the script is not in the repo. See the warning above.
python docs/testing/.scratch/extract_tests.py
```

Then the markdown was regenerated with:

```bash
# Historical — the script is not in the repo. See the warning above.
python docs/testing/.scratch/generate_inventory.py
```

Diff the output against `git status` to see which crate files actually moved — most PRs only touch one or two.

## Conventions

- Test rows are sorted by `(file path, line)`. Keep them sorted when hand-editing.
- The `Test` column links use `../../../` to climb out of `docs/testing/inventory/` to the repo root, then `<file>#L<line>` (GFM/VS Code line anchor). For a multi-line range use `#L<start>-L<end>`.
- **Link the specific submodule file, never the parent `mod.rs`.** When a `foo.rs` grows into a `foo/` directory (the split this repo's file-organization rule mandates — see [CLAUDE.md](../../../CLAUDE.md)), point at the file that actually holds the test, e.g. `cell/combat/threat/aggro.rs`, not `cell/combat/threat/mod.rs`. A reader landing on `mod.rs` still has to search 8 files, which is the work the link exists to save.
- **Verify every `#L<line>` against the `fn` line; never carry a prior value forward.** An anchor that still resolves but points at the wrong code is worse than a broken one, because nothing surfaces it — the reader simply reads the wrong test. On any edit that moves code, re-derive the anchor rather than assuming it held.
- **This verification is scriptable and idempotent**, which is what makes line anchors a maintainable choice rather than a rot-prone one. Because the `Test` column's link text *is* the test's function name, every anchor in this directory can be re-derived mechanically: search the target file for `^\s*(pub )?(async )?fn <name>\s*[(<]` and rewrite the line number to the match. Where a name matches twice (a test sharing a name with the production fn it exercises), disambiguate by scanning up to 6 lines above for `#[test]` / `#[tokio::test]` and taking the test definition. A full re-derivation over the ~900 links here takes about a minute and produces no diff when the catalogue is already correct — so it can run as a CI drift gate rather than a manual sweep.
- Pick `Kind` from the taxonomy in [TESTING.md](../../../TESTING.md), which has grown to 12 types: `unit` / `wire-format` / `live-DB` / `smoke` / `concurrency` / `chain-replay` / `legacy-reference` / `fan-out byte` / `Mercury session` / `network chaos` / `wire-level replay` / `negative-log`. Existing catalogue rows only use the first seven plus `proptest` / `rstest` / `integration`; the newer five need classifying on the next sweep.
- `What it tests` should be one short sentence. Prefer the test's `///` doc comment verbatim. If the body is opaque and the function name doesn't tell you, write `(infer from body)` rather than fabricating intent.
- `Notes` is empty unless something is unusual: `#[ignore]`, parameterization, or a smell signal from the extractor.

## Scratch directory

`docs/testing/.scratch/` **does not exist in the repository**, and is gitignored
at two levels — [`.gitignore:178`](../../../.gitignore) (`docs/testing/.scratch/`)
and [`.gitignore:209`](../../../.gitignore) (`.scratch/*`).

It was intended to hold the extractor script and its JSON output as the
inventory's regeneration source. Because it was never tracked, that source is
not recoverable from a checkout: verified 2026-07-25 that the directory is
absent and that neither `extract_tests.py` nor `generate_inventory.py` appears
anywhere in the tree.

Practical consequences, so nobody plans around a capability that isn't there:

- **The inventory has no working regeneration path.** See the warning under
  [How to regenerate](#how-to-regenerate).
- **Conventions enforced only by the generator cannot be enforced at all**,
  because a gitignored script never appears in a diff and so can never be
  reviewed. Any convention that matters belongs in this file *and* in a tracked
  generator.
- Re-authoring the extractor into `tools/` is the standing fix. It is tracked
  as a follow-up, not done here.
