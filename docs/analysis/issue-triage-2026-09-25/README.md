# Open-issue triage (2026-09-25)

This is an evidence-backed review of every open GitHub issue (123) against origin/main @ 059d6038.
Each issue got a verdict (KEEP / REWRITE / CLOSE / NEEDS-OWNER) and a priority (P0-P3), plus
ready-to-post comment text and, for rewrites, a full replacement body.

**Status:** research is complete. The owner review is in progress: the answers so far are recorded
in [report.md §1](report.md#1-owner-answers-and-follow-up-research-2026-09-25), and the
owner-decision questions in §4 have not been reviewed yet. **No issue has been edited, labelled
or closed.** A fresh session does the execution after the owner signs off.

## Files

| File | Contents |
|---|---|
| [report.md](report.md) | Consolidated report: owner answers and follow-up research, cross-batch reconciliations, open owner decisions, new tickets to file, PRs touched, per-issue verdict index. **Wins over the findings files where they disagree.** |
| [findings/](findings/) | Per-batch research: one section per issue with verdict, evidence (file:line on main @ 059d6038, PRs, Ghidra addresses) and ready-to-post action text. |
| [snapshot-issues.tsv](snapshot-issues.tsv) | Issue number, created, **updatedAt at research time**, labels, title. |

Batches: `sec-a` / `sec-b` (security audit), `wire` (Mercury and method indices), `content`
(content engine, missions, crafting), `npc-combat` (NPC AI, movement, navmesh), `features`
(social and feature epics), `tooling` (lab, telemetry, launcher, admin API, editor), `debt`
(tests, splits, coverage, docs programs).

## Method

- Eight research agents, one per batch, read only: no GitHub writes and no builds.
- Every factual claim in an issue (paths, line numbers, constants, "X is missing") was checked
  against the code on main, `docs/`, merged PRs and, where docs were silent, read-only Ghidra
  decompiles. Colo SigNoz was used where field evidence mattered.
- The agents' outputs were then reconciled against each other (report §3).

## Executing the triage (fresh session)

1. Read [report.md](report.md) first; open a findings section only for the issue being acted on
   (`grep -n "^## #<N> " findings/*.md`).
2. Apply only verdicts the owner approved. NEEDS-OWNER items wait for answers (report §4).
3. Before touching an issue, compare its current `updatedAt` with `snapshot-issues.tsv`. If the
   issue changed, re-read it and re-verify before posting.
4. Apply report §1 and §3 on top of the per-batch action text. Notably:
   - #63/#443/#461 speed cap
   - #466 → #72
   - the #459 umbrella index
   - #684-#689 stay open
   - #439/#25/#353 priorities
5. **Rewrite:** post the comment, then `gh issue edit <N> --body-file <file>`.
   **Close:** post the comment, then `gh issue close <N> --reason completed|"not planned"`.
   In Git Bash, use `MSYS_NO_PATHCONV=1` for gh arguments that look like paths.
6. #16 and #18 were filed by another contributor; keep the reopen invitation in the #18 close.
7. File the report §5 new tickets per the body contract in
   [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
8. The approved fixes (report §1.2 buyback dupe, §1.3 respawn gate, then §1.4 ammo after its
   research, and §1.5 mail after its RE packet) are ordinary PRs. Each needs the regression test
   described in its section.
9. Update this README's status line when done.
