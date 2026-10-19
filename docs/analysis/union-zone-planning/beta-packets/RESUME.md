# Beta Site E1 (Union) resume card

- Phase: planned 2026-10-18 by the Beta architect. Nothing dispatched, no worktrees, no branches, nothing committed.
- Index and ledger: [../beta-work-packets.md](../beta-work-packets.md) (findings F1-F21, decisions D-UB1..15, ID block, Omega receiver contract, HELD list, waves).
- Packet files: UB-00, UB-01a, UB-01b, UB-02a, UB-02bcd, UB-10-13, UB-14-16, UB-20-22, UB-30-36, UB-40-52, UB-90-99. Every chain packet also reads chain-packet-common.md.
- Architect report: `.claude/reports/union-zone-planning/beta-architect.md` (gitignored, in the pr-1735-rebase worktree).
- Scope: the first visit to world 23 for Human, Free Jaffa and Asgard players. Visit 2 (1807-1826) and E2 (world 60) are UB-L, out of scope.

## Waves

- B0: UB-00a1/a2/b/c (seals, docs; the mission seal is split in two) and UB-01a (pilot). Gate: U-D03, or the orchestrator waives it for docs-only UB-00*.
- B1: UB-01b, UB-02a, UB-02b, UB-02c, UB-02d (UB-02d needs D-UB4).
- B2: UB-10 to UB-13 (stack), UB-20, UB-30. Gate: none for Human and Free Jaffa (D-UB2 is decided, confirmed by Omega D-UO2); the Asgard leg UB-30 stays HELD on UO-16 (U-D08/U-D11).
- B3: UB-14 to UB-16 (stack), UB-21, UB-31; UB-22 only under D-UB5 B or C.
- B4: UB-32 to UB-36 (one stack), UB-40.
- B5: UB-45a then UB-45b (stacked); UB-50/51/52 only after the owner decides, and the architect expands them first.
- B6: UB-90 lab rows (ask the user first), then UB-99 close-out.

## Open decisions (orchestrator asks the owner before the dependent wave)

- D-UB4 west-valley route (A ring pair, B walk, C hold): blocks UB-02d, UB-21, UB-22, UB-40 camp rows.
- D-UB5 offworld tails (A stop, default; B Omega/SGC receivers; C authored relays): blocks UB-22, UB-50, UB-51.
- D-UB7 FV twins 383/384/1146 (recommend out): blocks UB-52.
- D-UB8 rewards (U-D07) and H-LBL label policy: block UB-50.
- D-UB2 (Decided): Beta fires the arrival steps; Omega D-UO2 confirms it fires none.
- D-UB6: the 421 verdict is BETA_LOCAL; the orchestrator confirms it against the brief's hold.
- D-UB9: the coordinator must add the 23000-23499 chain amendment row to id-allocation.md (UB-00c files the request).

## Next action

1. Orchestrator: confirm D-UB6; ask the owner D-UB4, D-UB5, D-UB7, D-UB8 and H-LBL.
2. ops: worktrees ub-00a, ub-00b, ub-00c, ub-01a; dispatch wave B0.
3. Rewrite this card at every wave boundary.
