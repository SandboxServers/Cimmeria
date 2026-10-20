# Four-zone review resume card

- **Campaign:** fix-up of `campaign/four-praxis-zone-restoration` (Cellblock, Tollana, Harset, Castle) before its one PR to main.
- **Ledger:** [README.md](README.md) (findings, packets, waves). Decisions: [decisions.md](decisions.md). Packets: [packets/](packets/).
- **State (2026-10-19):** FZ-01 to FZ-10, FZ-12, FZ-13 to FZ-23, FZ-25 to FZ-27 and PU-01 to PU-09 are merged into the campaign branch. Main was merged in by FZ-28 (`21afcbae7`); the campaign tip is the review-fix and ledger commits after it. Nothing is pushed.
- **Waves 11 and 12 merged.** Next: lab UAT (FZ-U1, FZ-U2), then the campaign review loop and the one PR.
- **Owner decisions:** D-FZ1 enable (owner checks after release), D-FZ3 squash (the ledger SHAs will not resolve on main; they stay valid in the campaign history), D-PU1 in this PR. Orchestrator calls: D-PU3 = B, D-PU4 yes, D-PU5 yes (leave the door open), O18-02a P0 (under the owner's delegation), O18-02b no, O18-02c not now.
- **New:** the owner decided D-PU3 = B and delegated the residual choices. FZ-26 is done ([worknote](worknotes/FZ-26-step-5226.md)); the architect sliced FZ-27a/b/c (D-FZ14, D-FZ15; ids collision-checked: chains 6018-6019, 6033-6034; dsm 120302-120303; dialog 60170; screens 200180-200181).
- **Integration:** each packet runs in `.claude\worktrees\fz-NN` on `four-zone-review/FZ-NN-<slug>` from the campaign tip; `ops` merges `--no-ff` on ship and retires `fz-NN`. Wave 12 is the exception: one worktree `fz-27`, three sequential commits, one merge.

## Next action

1. Lab UAT FZ-U1 and FZ-U2 (FZ-U2 includes the 5226 debrief, rewritten by FZ-27c) and owner end-UAT CS-07c; ask before using the lab. D-PU6 (door position) is pinned in FZ-U2.
2. Campaign review loop, then one PR. Merge main again first if it moves (FZ-28 was the last merge).

## Open decisions (owner unless marked)

- 1664: D-PU6 BlockedEvidence (door control position, placeholder at spawn 380; lab pin). Open item: does later Harset content need 1664 completed?
- D-FZ15: the two debrief bark lines are provisional; the owner or a writer may replace them (two seed rows).
- D-FZ5 (status docs) blocks the PR; the recommendation is a follow-up close-out PR, which the orchestrator must confirm. D-FZ2 shipped its default. D-PU2 (Harset prices), HA-15a, vendor faction gating, Castle OD-4/OD-5.
- Accepted risk: cross-mission soft-lock. A player stuck at 1360 step 4038 without the letter cannot credit Marsh objective 6061, so 1664 cannot finish; no GM path is verified.
- Info: D-FZ4, D-FZ6 to D-FZ15.

## Low follow-ups and cleanup

- Wave-7 lows: the FZ-15 test doc comment overstates ("one tag from each per-zone spawn file"); stale `logging/filters.rs` citations in `crates/server/src/otel.rs:34-36` and `main.rs:129-130` (comment-only; not changed in the review-fix pass, which is limited to the ledger folder); default merge messages.
- Spawn 380 sits outside the Harset 360-379 reservation, with a placeholder prop.
- `dialog_button_linter` does not scan `castle_567_chains.sql`, so dialog 2043 has no lint coverage.
- Some commit trailers say Haiku 5.5; history is not rewritten.
- Six `harset-ha-*` branches (`harset/ha-1360a1-letter-tx`, `a2a-session-token`, `a2b-turnin-route`, `a3a-action`, `a3b-request`, `1360b-marsh-receipt`) are not ancestors of the campaign branch. `git cherry` finds every commit patch-equivalent to a campaign commit (0 unique commits each), so their content is in. Their worktrees are owed retirement.
- `harset-ha-1360a2b-refusal` (`harset/ha-1360a2b-refusal`) has uncommitted `letter_turn_in_route*.rs`. Owner decides.
- F28 (`regen.py --check` drift) and F29 (CRLF offender, tracked `PP-13-coder.md`) belong to main; they need a main-side issue through `ops`.
- Worktrees retired: `fz-08` to `fz-10`, `fz-12` to `fz-27`, `pu-*`, `harset-ha-03c`, `a2c`, `a3c`, `a3c-revert`, `b1`, `b2`. Owed: the six `harset-ha-*` above and `harset-ha-1360a2b-refusal`. Never `git worktree prune`.
