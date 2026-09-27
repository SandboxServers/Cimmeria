---
name: reference-campaign-closeout-status-docs
description: Where a campaign close-out must touch status (gap-analysis sections beyond the obvious one, message-catalog Impl rows, the matrix recount) and how to recount safely
metadata:
  type: reference
---

A campaign close-out's status sweep reaches further than the campaign's own gap-analysis section. Found at the organizations close-out (ORG-11, 2026-09-27):

- `docs/gap-analysis.md` often has a **second section for the same code** (organizations §23 and Groups / Parties §30 both describe the squad) and a **row in another system's section** that waits on the campaign (Chat §21 "Pre-defined channels" was IM with `Blocks: Orgs`). Grep the whole file for the system's names before recounting.
- After changing rows, **recount every matrix row from the feature tables by script** (parse `| Feature | Status |` rows per `### N.` section, compare to the Summary Completion Matrix). Earlier editions shipped totals that disagreed with their own tables. Then update, together: the matrix rows, TOTALS, Summary Percentages, "Code exists" / "Missing", the "Since" delta table and its bullets, the `Last updated` line, and in `docs/project-status.md` the headline count, the Overall Completion table, the system rows and the roadmap.
- `docs/protocol/message-catalog.md` has a per-message `Impl` column, a NetOut and a NetIn "Summary by System", and an "Implementation Coverage Summary" with a TOTAL row. The whole table is known-stale across campaigns (see its WARNING); update only the rows your campaign owns plus the TOTAL.
- `docs/reverse-engineering/findings/README.md` is LF while most `docs/**` is CRLF; check each file with `file` rather than assuming.

Line endings: the Edit tool keeps a file's CRLF; the Write tool writes LF, so normalize a written file to CRLF afterwards. Related: [[feedback-source-doc-override]].
