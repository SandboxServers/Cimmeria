# ai-harness-docs: resume card

Ledger: docs/analysis/ai-harness-docs/README.md. Packets: work-packets.md (rules, C1-C9), work-packets-w3b.md (AH-11 rules, C7b, C10, AH-11a..f), work-packets-w3d.md (after #1824: AH-11a-r, W3d AH-11j..o, AH-11p, the merge order, C11) and work-packets-w3.md (W3c AH-11g..i, W4).

## State (2026-10-18)

- W1 to W3 merged. #1820 (the AH-11 ledger) and #1824 (the ops git verbs, 9664e56f4) are merged.
- W3b PRs are open: #1825 (AH-11e), #1826 (b), #1827 (c), #1828 (d), #1829 (a). The W3b review-fixer may still be running in those worktrees. AH-11f has no PR yet.
- #1829 CONFLICTS with main in ops.md and ops.toml (F29); the others are mergeable.
- This amendment: worktree `.claude/worktrees/ah11-1824-amend` (branch docs/ah11-1824-amend), committed locally, not pushed.

## Next action

1. `ops`: ship ah11-1824-amend as a docs PR and merge it, so new packet worktrees contain w3d.
2. When the W3b review-fixer has returned: merge AH-11e first, then b, c and d.
3. AH-11a-r (packet-coder, worktree ah-11a). Then `ops` runs `push-branch.ps1 -C <ah-11a> -Branch ah-11a`, then ships #1829. Then ship AH-11f (dispatch it first if it has not run).
4. Once AH-11e has merged: `ops` makes worktrees ah-11j, ah-11l, ah-11m, ah-11n and ah-11o, and the five packets are dispatched in the background. AH-11k waits for AH-11j's commit and stacks on its branch.
5. W3d: one packet-review-fixer over all six, then ship (AH-11j before AH-11k).
6. W3c: AH-11g, AH-11h and AH-11p in parallel, once W3b and W3d have all merged; one review-fixer.

## Later

- AH-11i: the owner posts pr-1797-close.md with the PR numbers filled in, then closes #1797.
- W4: AH-15 (owner; ask D-AH33 first), then AH-16.

## Owner questions open

- D-AH33: is the AH-15 Linux/WSL leg required (recommended) or best effort?
- D-AH29 (default no, non-blocking): a bash path for packet workers and the lane.
- D-AH35 is closed: #1824 merged on its own.
- Owner actions: the AH-11i reply and close; the AH-15 run.

## Loop rules

- Coder returns `blocked` with reason `context`: send a fresh packet-coder, "continuation N of 3", with the same worknote; after that, one Sonnet implementer.
- Review-fixer returns `recheck`: send a fresh review-fixer for that packet, up to 3 passes, then an architect ruling.
