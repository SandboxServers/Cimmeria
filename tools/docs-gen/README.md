# Generated doc blocks

`regen.py` owns every number in the Markdown that PRs used to bump by hand: the
workspace test counts, the RE findings count, the gap-analysis totals, the docs
count, and the crate graph. Two PRs that bump the same number conflict with each
other, so no PR bumps them. The [`regen-docs`](../../.github/workflows/regen-docs.yml)
workflow runs the script on `main` after every merge and commits whatever changed
as `github-actions[bot]`.

```bash
python tools/docs-gen/regen.py                      # rewrite stale blocks (same as --write)
python tools/docs-gen/regen.py --check              # exit 1 and list stale blocks; writes nothing
python tools/docs-gen/regen.py --skip-crate-graph   # everything except the crate graph (no cargo)
python -m unittest discover -s tools/docs-gen -p "test_*.py"
```

It needs Python 3.11+ and, for the crate graph, cargo (`cargo metadata --no-deps`).
Agents run it through the build lane: `bash tools/build-lane/lane.sh python tools/docs-gen/regen.py`.

## Markers

The script rewrites only text between markers and leaves every other byte alone,
CRLF line endings included:

```markdown
We have <!-- gen:tests-total -->7,718<!-- /gen:tests-total --> tests.

<!-- gen:tests-totals -->

| Metric | Count |
|---|---:|
| Tests (`#[test]` / `#[tokio::test]`) | 7,718 |
| Files with tests | 1,271 |
| Gated in CI (every crate but CI's exclude list) | 7,300 |
| Live-DB tests (`require_db_or_skip!` in the body) | 1,326 |
| Inventory threshold (5% of the tests) | 386 |

<!-- /gen:tests-totals -->
```

A generator whose output is one line is written inline. One whose output is a
table is written as a block, with a blank line on each side. Arguments follow the
name in the opening marker: `<!-- gen:gap-pct CW+NT+IM 0 -->79%<!-- /gen:gap-pct -->`.
The crate graph keeps its older `<!-- crate-graph:begin -->` / `<!-- crate-graph:end -->`
markers and is rendered by [`crate-graph/crate_graph.py`](../crate-graph/README.md).
A marker inside a fenced code block or an inline code span is an example, and is
left alone.

To add a generated number, wrap the existing number in a marker and run the script.
Commit its output in that PR, since the PR adds the marker. Otherwise don't commit
regenerated blocks: the workflow does that after the merge.

## Generators

| Name | Arguments | Value | Source |
|---|---|---|---|
| `tests-total` | | Workspace `#[test]` / `#[tokio::test]` count | [`tools/extract_tests.py`](../extract_tests.py) |
| `tests-files` | | Files containing a test | same |
| `tests-ci-gated` | | Tests outside CI's exclude list | same |
| `tests-live-db` | | Tests with `require_db_or_skip!` in the body | same |
| `tests-threshold` | | 5% of `tests-total`: the PR size that must update the test inventory | same |
| `tests-totals` | | The canonical totals table (in `docs/testing/inventory/README.md`) | same |
| `tests-by-crate` | | The per-crate table (same file) | same |
| `re-findings-count` | | `*.md` in `docs/reverse-engineering/findings/`, minus `README.md` | the filesystem |
| `docs-md-count` | | `*.md` anywhere under `docs/` | the filesystem |
| `section-table-rows` | | Table data rows from the marker to the next heading | the document itself |
| `gap-count` | `total` or `CW+NT+...` | Sum of those columns over the system rows | Summary Completion Matrix in `docs/gap-analysis.md` |
| `gap-pct` | statuses, then decimals (default 1) | That sum as a percentage of the total | same |
| `gap-systems` | | Number of system rows | same |

The gap-analysis generators read the matrix rows and skip its TOTALS line, so the
TOTALS line and the summary percentages can never disagree with the rows. The rows
themselves are hand-maintained, and change once per campaign, in its close-out
packet.
