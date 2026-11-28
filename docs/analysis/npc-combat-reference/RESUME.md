# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/.

## State (2026-11-27)

- W0 merged (#2081..#2088), W1 merged (#2089), W2 merged, W3 merged (wave PR #2091), NC-32 merged (#2092).
- W4 lab runs NC-40..44 done 2026-11-27 (server 086fc2710; [w4-lab-results.md](w4-lab-results.md)), attested in #2093.
- Decided: D-NC1..D-NC9, D-NC11, D-NC12 (a); D-NC10 record only.

## Wave 3

All six W3 packets merged (#2091); wave review pass 1 fixed (wave3-fix-1.md), pass 2 in wave3-review-2.md.

## Next action

1. Apply the NC-50 review notes (campaign-review-1.md), then run NC-51 (status docs, unified-uat NC rows).
2. Open W4 findings for NC-51 to carry: blank attacker name for PRU and ScavDog in the client combat log (known cosmetic, unified-uat K26); NID Guard 20 s respawn not exercised; T2 Z9 lethal squad not exercised; Jaffa damage high (effect 736 RECONSTRUCTED, owner tuning); GM health commands fail ('target not found'), so health refill used death plus Release.
3. Leftovers, retire only with user approval: worktree nc-10, branch NC-04-seed-trace, DB sgw_nc_04_seed_trace; W2 worktrees with `rm-worktree.ps1 --merged-into <W2 PR>`; the six W3 packet worktrees with `rm-worktree.ps1 --merged-into <W3 PR>`.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0-W2 | NC-01..07, 10..15, 13, 20, 21a-c, 23, 24 | merged |
| W3 | NC-22, 25, 30, 31, 33, 34 | merged (#2091) |
| W4 | NC-40..44, NC-32 | lab runs done 2026-11-27 (w4-lab-results.md); NC-32 merged (#2092) |
| W5 | NC-50, NC-51 | NC-50 campaign review in progress, then NC-51 close-out |
