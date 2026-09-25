---
name: finding-seed-sql-line-scan-loses-rows
description: Seed-parsing linters must not scan line-by-line — Dialogs seeds have multi-line literals and chain seeds have apostrophes in `--` comments; both lose rows silently
metadata:
  type: project
---

`crates/content-engine/tests/interact_tag_linter.rs` reads its seeds a line at a time. Do not copy that approach for a new seed linter without measuring first — it is safe only for the `content_triggers` rows it was written against.

Measured 2026-09-21:

- `db/resources/Dialogs/Seed/dialog_screens.sql` — 1,013 of 13,467 rows have a raw newline inside the `text` column. A line-at-a-time scan drops all of them.
- `db/resources/Content/Seed/castle_*_chains.sql` — apostrophes appear inside `--` comments (`Frost's`, `work-packets.md's`, `Ba'al`). A scanner that tracks `'` but not comments treats the first as an opening literal and swallows every statement after it.
- No dollar-quoted strings and no `/* */` in either tree; one `dialog_screens` literal contains `$$`, so `$` must stay ordinary text.

**Why:** both failures are silent — the linter parses less data and still reports "no violations", which is exactly the vacuous pass that makes a linter worthless. `dialog_button_linter` guards against it by asserting the parsed row count equals the raw count of `INSERT INTO <table> ` occurrences per file; that comparison is self-checking in both directions (a merged statement loses a row, a split-inside-a-literal leaves a prefixless fragment).

**How to apply:** for any new seed linter, write a scanner that understands `''`-escaped literals and `--` line comments, address fields by column NAME from the statement's own column list, and handle multi-row `VALUES (...), (...)` (the chain seeds use it heavily — dialog 5862 is reachable only through one). Then pin the row count. See `crates/content-engine/tests/dialog_button_linter/sql_scan.rs`.

Layout trap: an integration test at `tests/foo.rs` resolves a bare `mod bar;` against `tests/bar.rs`, where every `.rs` becomes its own test target. Submodules must live in `tests/foo/` and be pulled in with `#[path = "foo/bar.rs"]`; the house `foo/mod.rs` style cannot apply.

Related: [[finding-seed-null-masks-livedb-assertion]], [[feedback-revert-to-verify-regression-guards]].
