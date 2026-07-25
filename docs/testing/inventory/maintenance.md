# Maintaining the test inventory

> **Type**: how-to  
> **Audience**: engineers  
> **Last updated**: 2026-07-25  
> **Index**: [README](README.md)

## When to update

A PR that adds or removes **≥5% of the workspace test count** (~150 tests against the current 3,012 baseline) updates the matching per-crate file under `docs/testing/inventory/` and the totals in [README.md](README.md) in the same PR. Smaller drifts get folded in by periodic sweep updates — a batched sweep every few weeks is cheaper than reviewing inventory churn on every PR. Renamed a test? Pick it up in the next sweep unless the PR is already in the ≥5% bucket.

Sweeps are scripted: run [`tools/extract_tests.py --write`](#how-to-regenerate). The catalogue has nonetheless drifted well past the ≥5% threshold — 1,351 rows against 3,012 tests as of 2026-07-25 — because for a long stretch the generator was not in the repo at all. That is fixed; the backfill sweep is still outstanding.

Inventory drift up to ~5% is acceptable between sweeps — see [.github/copilot-instructions.md](../../../.github/copilot-instructions.md) and [CLAUDE.md](../../../CLAUDE.md) for the doc-update map. A CI drift-check is now *possible* (`--check` and `--verify-links` both exit non-zero on drift) but is **not yet wired into a workflow**; for now, reviewers eyeball the diff against the threshold.

## How to regenerate

The generator is [`tools/extract_tests.py`](../../../tools/extract_tests.py) — stock Python 3, no dependencies, runs from a bare checkout. It reads the `members` list out of the root `Cargo.toml` (so `exclude`d paths like `fuzz/` and `tools/SGWLauncher/src-tauri` are never visited), finds every `#[test]` / `#[tokio::test]` / `#[tokio::test(flavor = …)]` function, and records the crate, file, function name, the line the `fn` is actually on, and whether the body invokes `require_db_or_skip!()`.

```bash
python tools/extract_tests.py                 # totals only — writes nothing
python tools/extract_tests.py --list-crates   # per-crate tests / files / live-DB
python tools/extract_tests.py --write         # regenerate docs/testing/inventory/*.md
python tools/extract_tests.py --check         # exit 1 if --write would change anything
python tools/extract_tests.py --verify-links  # re-derive every #L anchor; exit 1 on drift
python tools/extract_tests.py --json out.json # machine-readable dump
```

**Only `--write` touches the repository.** Every mode is idempotent — writing twice in a row produces a byte-identical tree — so it is safe to re-run.

After `--write`, diff against `git status` to see which crate files actually moved; most PRs only touch one or two. Update the totals in [README.md](README.md) to match the figures the tool prints.

### What regeneration preserves

`Kind`, `System / Feature`, `Added`, `What it tests`, and `Notes` are human-curated. The generator keys existing rows by `(file path, test name)` and carries those cells forward **verbatim** — it only rewrites the `#L` anchor and the set of rows. So a sweep repairs drifted anchors and adds or drops tests without flattening the prose you wrote last time.

New rows are seeded with defaults you are expected to refine:

- **`Kind`** — `live-DB` when the body invokes the guard, else `unit`. Reclassify against the taxonomy in [TESTING.md](../../../TESTING.md).
- **`System / Feature`** — derived from the first module segment under `src/`.
- **`What it tests`** — the test's `///` doc comment, first sentence, capped at 160 characters. Blank when the test has no doc comment.
- **`Added`** — left blank. Populating it needs `git log -S 'fn <name>' -- <file>` per test, which is minutes of subprocess churn across thousands of tests; supply it by hand for rows you care about and it survives every later sweep.

The `Last updated` line is likewise never rewritten — stamping it on every run would make `--check` fail a day after each sweep for no reason. Bump it by hand when you publish one.

### Two drift gates, and which to adopt first

`--check` is the strict gate: it regenerates in memory and fails if anything differs. It is the right end state, but it fails loudly today because the catalogue is missing more than half the suite — adopt it only after a backfill sweep.

`--verify-links` is the incrementally adoptable one. It ignores catalogue membership entirely and asks only whether each existing `#L` anchor still resolves to its own test's `fn` line. Verified 2026-07-25: **1,121 links, 0 stale anchors** — the anchor repair holds. The 44 dangling rows it reports are all in `launcher.md` and `tools-sgwlauncher.md`, which still point at `tools/SGWLauncher/src-tauri/` (a workspace `exclude`, and duplicated by `crates/launcher`); resolving those two files is a prerequisite for turning this gate on.

### Reconciling the live-DB count

The tool reports two numbers, and they legitimately differ:

- **live-DB tests** (245) — test functions whose own body invokes `require_db_or_skip!()`.
- **live-DB call sites** (247) — every invocation, including the two in the shared `assert_region_enter_*` helpers in `cell/content/chain_replay_tests/mission_638.rs`. The four tests calling those helpers are live-DB *transitively*, which a per-body scan cannot see; the tool lists the orphan sites explicitly so the gap is never a mystery.

Earlier counts of "247 live-DB guards" were counting call sites.

## Conventions

- Test rows are sorted by `(file path, line)`. Keep them sorted when hand-editing.
- The `Test` column links use `../../../` to climb out of `docs/testing/inventory/` to the repo root, then `<file>#L<line>` (GFM/VS Code line anchor). For a multi-line range use `#L<start>-L<end>`.
- **Link the specific submodule file, never the parent `mod.rs`.** When a `foo.rs` grows into a `foo/` directory (the split this repo's file-organization rule mandates — see [CLAUDE.md](../../../CLAUDE.md)), point at the file that actually holds the test, e.g. `cell/combat/threat/aggro.rs`, not `cell/combat/threat/mod.rs`. A reader landing on `mod.rs` still has to search 8 files, which is the work the link exists to save.
- **Verify every `#L<line>` against the `fn` line; never carry a prior value forward.** An anchor that still resolves but points at the wrong code is worse than a broken one, because nothing surfaces it — the reader simply reads the wrong test. On any edit that moves code, re-derive the anchor rather than assuming it held.
- **This verification is scriptable and idempotent**, which is what makes line anchors a maintainable choice rather than a rot-prone one, and it is now implemented: `python tools/extract_tests.py --verify-links`. Because the `Test` column's link text *is* the test's function name, every anchor is re-derived from the test attribute that precedes the `fn` — so a test sharing a name with the production function it exercises resolves unambiguously. A full pass over the 1,121 links here takes a couple of seconds and produces no output when the catalogue is correct, so it can run as a CI drift gate rather than a manual sweep. Names wrapped in backticks in the link text are matched the same as bare ones, and normalized to bare on the next `--write`.
- Pick `Kind` from the taxonomy in [TESTING.md](../../../TESTING.md), which has grown to 12 types: `unit` / `wire-format` / `live-DB` / `smoke` / `concurrency` / `chain-replay` / `legacy-reference` / `fan-out byte` / `Mercury session` / `network chaos` / `wire-level replay` / `negative-log`. Existing catalogue rows only use the first seven plus `proptest` / `rstest` / `integration`; the newer five need classifying on the next sweep.
- `What it tests` should be one short sentence. Prefer the test's `///` doc comment verbatim. If the body is opaque and the function name doesn't tell you, write `(infer from body)` rather than fabricating intent.
- `Notes` is empty unless something is unusual: `#[ignore]`, parameterization, or a smell signal from the extractor.

## Why the generator lives in `tools/`

The original extractor lived at `docs/testing/.scratch/extract_tests.py`, which
is gitignored at two levels — [`.gitignore:178`](../../../.gitignore)
(`docs/testing/.scratch/`) and [`.gitignore:209`](../../../.gitignore)
(`.scratch/*`). It was therefore never tracked, is not recoverable from a
checkout, and the procedure documented here was unrunnable by anyone for as long
as it took the catalogue to fall behind by more than half the suite.

That is the failure mode the current placement guards against: **a gitignored
generator never appears in a diff, so it can never be reviewed** — and any
convention expressed only in that script is enforced by nothing. Two rules
follow, and they are the reason this section exists rather than being deleted
along with the problem:

- The generator stays in version control, under `tools/`.
- Any convention that matters is written down **here** *and* implemented in the
  generator. One without the other is how the last one rotted.

Its JSON output (`--json`) is scratch data and should not be committed.
