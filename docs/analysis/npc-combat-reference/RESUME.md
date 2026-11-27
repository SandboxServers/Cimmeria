# RESUME: npc-combat-reference (NC)

Ledger: README.md. Packets: work-packets.md, work-packets-w1-w2.md, work-packets-w3-w5.md. Rulings: worknotes/wave0.md.

## State (2026-11-27)

- W0 (NC-01..07) and plan docs merged (#2081..#2088). W1 merged as #2089 (fix 7b1d353e9).
- W2 (NC-13, 20, 21a-c, 23, 24): all `ship`; wave review `ship` (pass 2).
- Full cell-world, cell-combat and cell nextest and live-DB green on the integration tree.
- W2 ships as one PR from worktree nc-w2 (branch npc-combat-reference/W2-integration).
- Decided: D-NC1, D-NC2, D-NC4, D-NC5, D-NC9.
- Open: D-NC3 (gates NC-22), D-NC6, D-NC7 (gates W4), D-NC8 (record only).

## Next action

1. Open the W2 PR from nc-w2. After merge, retire with `rm-worktree.ps1 --merged-into <PR>`: nc-13, nc-20, nc-21a, nc-21b, nc-21c, nc-23, nc-24, nc-w2.
2. W3 = NC-22, 30, 31, 33, 34. Ask D-NC3 before NC-22.
3. NC-31 carries: the README F5/F6/F3 rewrite (ruling note 16); stale "mob notifies" rows; MeleeDistance wording; the note that NC-12 closed effects 736 and 1717; deferred D1, a shared `template_components()` helper in `reference/mod.rs` (with `world_records()` components), so NC-14's weapon-evidence assert can fail.
4. W4 (lab NC-40..44) waits on D-NC7. Ask before any lab run.
5. Leftovers, retire only with user approval: worktree nc-10, branch npc-combat-reference/NC-04-seed-trace, DB sgw_nc_04_seed_trace, orphan targets.
6. Commit or stash the owner's MEMORY.md edit in main before pulling.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W0 | NC-01..07 | merged |
| W1 | NC-10, 11, 12, 14, 15 | merged (#2089) |
| W2 | NC-13, 20, 21a-c, 23, 24 | ship; PR pending |
| W3 | NC-22, 30, 31, 33, 34 | D-NC3 |
| W4 | NC-40..44 (D-NC7), NC-32 | W3 merged |
| W5 | NC-50, NC-51 | W4 |
