# TP-12 worknote: docs and status close-out

> Type: worknote. Packet TP-12 (docs half) of [#957](https://github.com/SandboxServers/Cimmeria/issues/957); ledger [README.md](../README.md). Written 2026-10-03 by the TP-12 docs worker. Branch `docs/token-usage-closeout`. The scheduled jobs (`tools/token-profile/scheduled/`) are the other TP-12 worker's.

## Done

- **Guide.** [docs/guides/token-profiling.md](../../../guides/token-profiling.md), a how-to: ingest, report, reconcile, validate, `pr_stats` for one PR and the backfill, the scheduled jobs, reading a stats comment, telemetry, privacy, failure table. Linked from `docs/readme.md` (guides table and the ledger row) and `CONTRIBUTING.md`.
- **Rules.** `development-workflow.md`: the worker-lifetime intro now says the close-out kept D-TP4; new rules "batch mechanical steps, or hand them to a small context" (TP-11's 11.2%) and "don't park a worker" (13.2%); in Running agents, "a worker's worktree is its own until it has reported" and D-TP10's tool choice; the definition of done points at the daily sweep. `rules-and-gotchas.md`: the vanished-worktree gotcha with the repair recipe, and a short "AI-assisted work and token cost" section that links rather than repeats.
- **Ledger.** Packet rows for TP-05b (#1138), the CodeRabbit fix (#1139), TP-11 (#1140) and TP-12; a Close-out section (TP-05b summary, backfill result, TP-11 summary, early before-and-after for levers A, B, D, E and H, open levers); cut-line rows for #1138 and TP-12; the backfill criterion ticked, the OTel criterion left open.
- **Status docs.** `project-status.md` gains a Development Tooling section with the profiler; `gap-analysis.md` notes the profiler under §36 without a matrix row (it is workstation tooling, not a server feature) and in "Since 2026-09-25".
- **Gap-analysis split.** `docs/gap-analysis.md` (1,476 lines) is now the index and summary (about 300 lines): header, taxonomy, a "Systems by area" table linking every section, the matrix and everything after it, so `tools/docs-gen/regen.py` still finds the matrix (`regen.py --check` passes apart from the docs count, which `main` regenerates). The sections moved unchanged into seven files under `docs/gap-analysis/` (infrastructure, core-gameplay, npc-systems, secondary-gameplay, stub-only-systems, new-systems, server-infrastructure), with relative links rewritten. The one inbound anchor (`docs/gameplay/inventory-system.md`, §15) was already off by one hyphen and now points at the new file. The status-doc rule in `CLAUDE.md`, `AGENTS.md`, `.github/copilot-instructions.md`, `development-workflow.md`, `doc-update-map.md` and `rules-and-gotchas.md` now names the area files too.
- **Doc-update map.** New row for the token profiler.

## How the before-and-after numbers were made

`python tools/token-profile/cutlines` on a copy of the profiler database, re-ingested at 17:27 UTC, plus two read-only queries on the same copy: requests that wrote a 1-hour cache, and the idle 5-minute rewrite share, by agent type either side of the TP-06 cut; and main-session spend by trigger kind either side of the TP-00 cut. The "after" samples are a few hours of work, so the ledger calls them early signals.

## Left / open

- The ledger's TP-12 rows say "this PR"; fill in the PR number and the TP-12 cut-line merge time when it merges.
- The scheduled-job details in the guide (script names, 07:30 daily, 08:00 Mondays, results outside the repo) were written from the brief before the scripts existed; check them against the tools worker's PR.
- `tools/token-profile/README.md` still says "Status: Wave 2". It belongs to the tools worker.
- Pre-existing broken links in the moved gap-analysis sections, left as they were: `crates/resources/src/base/item_overrides.rs` (§3), and two organization paths in §23 (`cell_methods/organization/`, `organization/squad/`; the code is now in `crates/cell-org/` and `crates/cell-interactions/`). Three clear ones were fixed in the move (the cover, crafting and black-market paths).
- The rule changes to `CLAUDE.md` are one parenthesis; the worktree gotcha deliberately stays out of `CLAUDE.md` to keep static context down.
