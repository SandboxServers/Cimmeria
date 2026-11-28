# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/.

## State (2026-11-28): campaign closed

- All packets are Integrated: W0 to W3 (#2081..#2091), NC-32 (#2092), W4 lab runs attested (#2093), campaign review NC-50 with D-NC13 (#2094), close-out NC-51 (this PR).
- V1 merged as #2081..#2094. Releases `v2026-11-28.1` (up to #2093) and `v2026-11-28.2` (with #2094), both 2026-11-28; this close-out ships with the next release.
- Decisions D-NC1..D-NC13 are final; D-NC10 is record only.
- W4 lab runs (2026-11-27, server 086fc2710) passed with notes ([w4-lab-results.md](w4-lab-results.md)): NID 559/5310, PRU 221/264, ScavDog 1433, Jaffa 584/710/736, no 592 anywhere in the session.
- Production placeholder 592 now only for templates with a human firearm (D-NC13); melee-only humans 28, 92, 1415, 1465 and deployable 400 idle; the gallery has 74 idle.

## Owed to a human

[unified UAT, NPC combat reference](../../guides/unified-uat.md#npc-combat-reference): NID 20 s respawn, T2 Z9 lethal squad (18 s), per-reference-NPC visual rows (animation, muzzle effect, VFX, sound: not verified in a client; sound UNRESOLVED), blank PRU and ScavDog attacker name (K26), Jaffa damage tuning (effect 736 RECONSTRUCTED).

## Follow-ups (new campaigns, not this one)

- V1.1: Horden and Straegis Titan, scoped under `docs/analysis/npc-combat-v1-1/`.
- V2: the remaining roster, scoped under `docs/analysis/npc-combat-v2/` (starting point: [roster-followup.md](roster-followup.md)).
- GM health commands answer "target not found" in the lab (health refill used death plus Release): not fixed here.
- Retire (ops, with user approval, rm-worktree.ps1 only): worktrees nc-10 (branch `npc-combat-reference/NC-10-wire-profile`), `npc-combat-reference/docs` and nc-51 (on merge); branch `npc-combat-reference/nc-04-seed-trace` and DB `sgw_nc_04_seed_trace` if it still exists (check with the DB list first). Keep nc-scope (branch `npc-combat-reference/scope-v1-1-v2`) until the V1.1 and V2 scoping merges.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0-W2 | NC-01..07, 10..15, 13, 20, 21a-c, 23, 24 | merged |
| W3 | NC-22, 25, 30, 31, 33, 34 | merged (#2091) |
| W4 | NC-40..44, NC-32 | lab runs done 2026-11-27; NC-32 merged (#2092) |
| W5 | NC-50, NC-51 | NC-50 merged (#2094); NC-51 integrated on merge |
