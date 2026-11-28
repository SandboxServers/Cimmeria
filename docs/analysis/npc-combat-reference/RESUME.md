# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/.

## State (2026-11-27)

- W0 merged (#2081..#2088), W1 merged (#2089), W2 PR (ask ops whether it has merged).
- W3 integrated on `npc-combat-reference/W3-integration` (worktree nc-w3) with NC-22, NC-31, NC-33, NC-34, NC-25, NC-30 all merged, awaiting the wave review and the green exclusive nextest, then the wave PR.
- Decided: D-NC1..D-NC9, D-NC11, D-NC12 (a); D-NC10 record only.

## Wave 3

All six W3 packets integrated; wave review pass 1 fixed (wave3-fix-1.md), pass 2 in wave3-review-2.md.

## Next action

1. Apply the pass-2 wave review notes (wave3-review-2.md), then the pass-3 check if the verdict is not ship.
2. Green exclusive nextest on nc-w3, open the W3 PR.
3. W4 lab runs NC-40..44 are done (2026-11-27, server 086fc2710; [w4-lab-results.md](w4-lab-results.md)). NC-32 is its own PR. Open findings: blank attacker name for PRU and ScavDog in the client combat log (new open issue, no fix packet written); NID Guard 20 s respawn not exercised; T2 Z9 lethal squad not exercised; Jaffa damage high (effect 736 RECONSTRUCTED, owner tuning); GM health commands fail ('target not found'), so health refill used death plus Release.
4. W5 NC-50, NC-51 is next.
5. Leftovers, retire only with user approval: worktree nc-10, branch NC-04-seed-trace, DB sgw_nc_04_seed_trace; W2 worktrees with `rm-worktree.ps1 --merged-into <W2 PR>`; the six W3 packet worktrees with `rm-worktree.ps1 --merged-into <W3 PR>`.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0-W2 | NC-01..07, 10..15, 13, 20, 21a-c, 23, 24 | merged or shipped |
| W3 | NC-22, 25, 30, 31, 33, 34 | integrated, PR pending |
| W4 | NC-40..44, NC-32 | lab runs done 2026-11-27; NC-32 separate PR |
| W5 | NC-50, NC-51 | W4 |
