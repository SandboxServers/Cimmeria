# `content_*.chain_id` is `integer` — an `i64` decode hides behind a vacuous guard

`db/resources/Content/Tables/content_actions.sql` (and its sibling
`content_conditions` / `content_triggers` / `content_chains` tables) declare
`chain_id integer`. Decoding it as `i64` in a `sqlx::query_as` gives

```
ColumnDecode { index: "0", source: "mismatched types; Rust type `i64`
(as SQL type `INT8`) is not compatible with SQL type `INT4`" }
```

**The trap:** a structural guard shaped "no chain in this range may use verb
X" queries for rows that must be ABSENT. With zero rows returned, sqlx never
decodes anything, so a wrong `i64` compiles, runs and PASSES forever. The day
the guard actually catches a violation it panics with the decode error above
instead of the assertion message the author wrote — which reads as a broken
test, not as the defect it just found.

Found three such guards in one file (Harset H20, 2026-09-19); one queried a
`GROUP BY` that did return rows and failed immediately, the other two were
silently vacuous.

**Rule:** when writing a "must return no rows" guard, pick the decode types
from the table DDL, not by copying a sibling test. Aggregates over an
`integer` column (`MAX(CASE ... THEN sort_order END)`) are also `INT4` → `i32`.

Pair it with an anti-vacuity assertion where the guard has a subject that
should exist — e.g. an ordering guard over `HAVING ... IS NOT NULL` should
assert `!rows.is_empty()` with a message like "this guard has lost its
subject, which means the chains were renumbered or gutted". That assert is
what tripped correctly during revert-verification.

Contrast: `dialog_set_maps.interaction_flags` IS `bigint` → `i64`, and
`dialog_id` is nullable `integer` → `Option<i32>`. Mixed widths in one row
tuple are normal here; read the DDL.

See also [[vacuous-guard-and-sentinel-collision-review]],
[[db-test-revert-verification]].
