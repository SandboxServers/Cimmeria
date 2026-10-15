# Cellblock Wave-0 resume

> **Type**: Reference. **Audience**: Engineers and campaign coordinators.
> **Last updated**: 2026-10-15.
> **Companions**: [Ledger](README.md), [CB-N00 worknote](worknotes/cb-n00.md),
> [execution contract](../four-zone-planning/execution-contract.md).

- Base SHA: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
- CB-N05a catalog `94b4a5754` merged at `67d3368e9`; pre-merge Wave-0
  tip `457b4f75a` was reviewed.
- Integrated packet: CB-N00, evidence/doc gate only; client rows **UATPending**.
- Packet `63f609c4f`; integration `88e5f6d75`; reviewed fixer `b59c4a124`.
- Wave-close HEAD: resolve `git log -1 --format=%H -- RESUME.md` in this folder.
- Branch/worktree: `content/cellblock-campaign`, `.claude/worktrees/zone-cellblock`.
  Packet worktree `cellblock-cb-n00` is retired; integration worktree stays active.
- `north_review` findings on punctuation, index/memory and FZ-99 boundary
  were fixed; `north_rereview` found no further actionable issue.
- Checks: whitespace, CRLF, scoped markdownlint and local links passed;
  anchors/remote links and runtime/seed checks were not run.
- Integrated CB-N01: reviewed presentation/ownership handoff, no runtime proof.
- Next: bind Codex's `lab-server` to the client server; isolate the
  screenshot/Livewire stall and relocation; rerun fresh Human/Jaffa CB-U09.
  N05 design passed review; dispatch waits for valid pacing.
- Blockers: UNSEALED live route/build joins, normal core/optional pacing,
  cast/placement and reachability sign-off, first-proof eligibility, individual ID
  collision audits, and first-authored-quest live journal/progression proof.
  O14-04/O14-10/O14-12 delegate these bounded seals; CB-N05a infrastructure
  is on `main`, while production mission additions remain empty.
  All downstream implementation packets stay blocked pending their complete seals.
- 2026-10-15: stopping an active screensaver let p3 enter world 12. A fresh
  Human completed pistol grant/equip/reload and killed the first guard. The
  Livewire helper and screenshot stalled; the character relocated without
  mission advance. CB-U09 timing is invalid; CB-U02 lacks packet/visual proof.
  P3 was stopped and released. See [route baseline](route-baseline.md#2026-10-15-fresh-human-lab-run).
- [Standing O14 decisions](../four-zone-planning/owner-decisions-2026-10-14.md):
  coordinated lab use is approved, not passed; unavailable-lab rows may be
  `UATPending`, while observed gameplay failures block merge. Core pacing is
  30–45 minutes, with at most 10 optional minutes; N06/N07 are conditional.
