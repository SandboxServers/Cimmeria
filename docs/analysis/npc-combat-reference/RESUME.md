# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/.

## State (2026-11-27)

- W0 merged (#2081..#2088), W1 merged (#2089), W2 PR (ask ops whether it has merged).
- W3 integrated on `npc-combat-reference/W3-integration` (worktree nc-w3) with NC-22, NC-31, NC-33, NC-34, NC-25, NC-30 all merged, awaiting the wave review and the green exclusive nextest, then the wave PR.
- Decided: D-NC1..D-NC9, D-NC11, D-NC12 (a); D-NC10 record only.

## Wave 3

All six W3 packets integrated; reviews: wave1a, 1b (this file) pending fixes.

## Next action

1. Fix the wave review notes (wave3-review-1a.md, wave3-review-1b.md).
2. Green exclusive nextest on nc-w3, open the W3 PR.
3. W4 (lab NC-40..44, NC-32) after the W3 PR merges; ask D-NC7 before every lab run.
4. W5 NC-50, NC-51 after W4.
5. Leftovers, retire only with user approval: worktree nc-10, branch NC-04-seed-trace, DB sgw_nc_04_seed_trace; W2 worktrees with `rm-worktree.ps1 --merged-into <W2 PR>`; the six W3 packet worktrees with `rm-worktree.ps1 --merged-into <W3 PR>`.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0-W2 | NC-01..07, 10..15, 13, 20, 21a-c, 23, 24 | merged or shipped |
| W3 | NC-22, 25, 30, 31, 33, 34 | integrated, PR pending |
| W4 | NC-40..44, NC-32 | W3 merged, D-NC7 per run |
| W5 | NC-50, NC-51 | W4 |
