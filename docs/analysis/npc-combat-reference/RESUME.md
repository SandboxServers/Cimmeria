# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md (rules, contract, W0),
work-packets-w1-w2.md and work-packets-w3-w5.md.

## State (2026-11-27, design done)

- The architect wrote the ledger. No worktrees, PRs or coders exist yet.
- The files are untracked in the main checkout. Ask `ops` to commit them on a
  docs branch before W0 starts.
- Verdict: feasible with re-scoping. The 592 removal is staged (`Legacy`
  resolver first), and all testing is in the Debug Area Z12 range.

## Next action

1. Ask the owner D-NC1..D-NC8 (README § Decisions). W0 needs none of them.
2. Create the board subcategory "NPC combat reference" through `ops`.
3. Dispatch W0, all 7 packets in parallel, one worktree each:
   - NC-01, NC-02, NC-04: packet-coder;
   - NC-03: rust-gameserver-dev;
   - NC-05, NC-06: game-archaeology-specialist (these need the owner's
     client tree);
   - NC-07: combat-systems-advisor.

   Merge order: NC-01, NC-03, NC-07, NC-04, NC-02, NC-05, NC-06.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0 | NC-01..07 | none |
| W1 | NC-10, 11, 12, 14, 15 | NC-01 and NC-03; NC-12 needs D-NC2 |
| W2 | NC-13, 20, 21a-c, 23, 24 | NC-12, NC-15; D-NC4, D-NC5 |
| W3 | NC-22, 30, 31, 33, 34 | D-NC1, D-NC3 |
| W4 | lab NC-40..44 (ask first, D-NC7), plus NC-32 docs | W3 merged |
| W5 | NC-50 review, NC-51 close-out | W4 |

In flight: NC-03, re-dispatch on D-NC9 (Decided: spawns 14300-14319). Open: D-NC1..D-NC8, all BlockedDecision (README).
