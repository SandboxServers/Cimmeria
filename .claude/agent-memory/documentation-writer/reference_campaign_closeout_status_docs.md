---
name: reference-campaign-closeout-status-docs
description: Where a campaign close-out must touch status (gap-analysis sections beyond the obvious one, message-catalog Impl rows, the matrix recount) and how to recount safely
metadata:
  type: reference
---

A campaign close-out's status sweep reaches further than the campaign's own gap-analysis section. Found at the organizations close-out (ORG-11, 2026-09-27):

- `docs/gap-analysis.md` often has a **second section for the same code** (organizations §23 and Groups / Parties §30 both describe the squad) and a **row in another system's section** that waits on the campaign (Chat §21 "Pre-defined channels" was IM with `Blocks: Orgs`). Grep the whole file for the system's names before recounting.
- After changing rows, **recount the section's feature table by script** (parse `| Feature | Status |` rows under its `### N.` heading) and set its Summary Completion Matrix row to match. Since docs-regen (#923, 2026-09-27) the matrix rows are the generator's input: TOTALS, Summary Percentages and "Code exists" / "Missing" sit in `<!-- gen:gap-count -->` markers that `tools/docs-gen/regen.py` rewrites on `main` after the merge, so never edit them. Still hand-maintained: the "Since" delta table and its bullets, and in `docs/project-status.md` the Overall Completion counts and percentages (no gen markers there), the system rows and the roadmap. Found at the bank close-out (BV-10, PR #968).
- `docs/protocol/message-catalog.md` has a per-message `Impl` column, a NetOut and a NetIn "Summary by System", and an "Implementation Coverage Summary" with a TOTAL row. The whole table is known-stale across campaigns (see its WARNING); update only the rows your campaign owns plus the TOTAL.
- `docs/reverse-engineering/findings/README.md` is LF while most `docs/**` is CRLF; check each file with `file` rather than assuming.

Line endings: the Edit tool keeps a file's CRLF; the Write tool writes LF, so normalize a written file to CRLF afterwards. Related: [[feedback-source-doc-override]].

Lint: `tools/lint-md.ps1 <files>` lints the whole repo anyway (the config's globs win), so filter its output to your files and diff against the base. MD029 flags an ordered list that restarts at 15 after a heading; a UAT checklist that continues its numbering needs a scoped `<!-- markdownlint-disable MD029 -->` / `enable` pair.

Checking a recount without committing generated blocks: `python tools/docs-gen/regen.py --check --skip-crate-graph` prints each stale `gen:gap-count` / `gen:gap-pct` value as `old -> new`, which is the post-merge headline; it exits 1 and writes nothing. project-status's hand-maintained "Code exists" line had drifted from its own table before the crafting close-out (CR-13, 2026-09-27), so recompute it rather than adjusting it.

Git Bash `sed -i` on a CRLF doc rewrites it with LF endings; edit docs with Python (binary read/write) or the Edit tool, and re-check with `file`.
