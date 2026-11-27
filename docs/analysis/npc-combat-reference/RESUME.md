# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/.

## State (2026-11-27)

- W0 merged (#2081..#2088), W1 merged (#2089), W2 ships as one PR from nc-w2 (ask ops whether it has merged; W3 branches sit on it).
- Decided: D-NC1..D-NC9, D-NC11 (split: 559 gets new effect 5310, 621 unchanged); D-NC10 record only.
- Open: D-NC12 (the split still changes every 559 user: players' SMG/AR and all 29 set-3 templates; recommend (a) accept). Gates NC-25.

## Wave 3 in flight (one PR from integration branch `npc-combat-reference/W3-integration`, worktree nc-w3)

| Packet | Worktree | State |
|---|---|---|
| NC-22 | nc-22 | Writing; stopped on disk full on B: |
| NC-30 | nc-30 | Writing; two `ability_range.rs` tests and the revert proof (worknotes/NC-30.md) |
| NC-31 | nc-31 | Review (fix pass 1 committed) |
| NC-33 | nc-33 | Review (fix passes committed) |
| NC-34 | nc-34 | Review |
| NC-25 | nc-25 (not cut) | BlockedDecision D-NC12; then cut from the nc-w3 head after NC-31 and NC-33 merge into it |

Gate: each packet `ship`, then `merge-noff` into nc-w3 in order NC-22, NC-31, NC-33, NC-34, NC-25, NC-30 last; one wave review `ship` over nc-w3; green exclusive workspace nextest on nc-w3; then the wave PR.

## Next action

1. Ops cuts nc-w3 from `origin/main` (W2 merged) or W2 head 2a1887107. Free disk on B:, then continue NC-22 and the NC-33 TOML lint.
2. Continue NC-30 from its worknote (item 1: the ability_range seam; item 2: close the sgw_nc_30 session, revert proof).
3. Finish the NC-31/33/34 review loops; merge each into nc-w3.
4. Ask the owner D-NC12. On (a), ops cuts nc-25 from nc-w3 and dispatches NC-25; on (b)-(d), the architect re-specs it.
5. W4 (lab NC-40..44, NC-32) after the W3 PR merges; ask before every lab run.
6. Leftovers, retire only with user approval: worktree nc-10, branch NC-04-seed-trace, DB sgw_nc_04_seed_trace; retire W2 worktrees with `rm-worktree.ps1 --merged-into <W2 PR>`.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0-W2 | NC-01..07, 10..15, 13, 20, 21a-c, 23, 24 | merged or shipped |
| W3 | NC-22, 25, 30, 31, 33, 34 | above |
| W4 | NC-40..44, NC-32 | W3 merged, D-NC7 per run |
| W5 | NC-50, NC-51 | W4 |
