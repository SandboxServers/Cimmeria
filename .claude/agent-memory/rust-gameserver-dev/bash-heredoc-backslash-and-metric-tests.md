---
name: bash-heredoc-backslash-and-metric-tests
description: Bash-tool heredocs collapse a doubled backslash, so python edit scripts break on Rust line-continuation strings; and how to test metrics (recording meter, unique world label)
metadata:
  type: reference
---

**Heredoc backslashes.** A `python - <<'EOF'` script run through the Bash tool
received `\\` as `\` (seen 2026-10-04, AB-T6/T7). A replacement string holding a
Rust `"...\` + newline continuation then became a Python line continuation and
the match failed (or a markdown `\|` came through only by luck). For any edit
whose old/new text contains a backslash, use the Edit tool or a script file
written with Write, not an inline heredoc.

**Testing a metric.** `cimmeria_observability::testing::install()` (dev-dep
feature `testing`) records counters (`counter_total`) and, since AB-T6, f64
histograms (`histogram_count`, `histogram_sum`). The table is process-wide under
`cargo test`, so assert deltas and give each test its own label value; the
ability tests build their manager in a world named for the test
(`warmup_mgr_in("Metrics_T6_...")`) so `world` isolates them. Call `install()`
before building anything that caches a label: `ability_metrics::world_of`
returns `"unknown"` while the meter is off.

Related: [[python-write-mangles-utf8-and-crlf]], [[observability-test-and-throttle-traps]].
