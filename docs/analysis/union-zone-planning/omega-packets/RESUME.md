# Union Omega resume card

- Phase: closed on the campaign branch `union-omega/decisions-2026-10-19` (worktree `uo-decisions`) after the 2026-10-19 reopening (waves W6-W11). W1-W5 are on `main`. UO-98 (close-out 2) is the last docs commit; the PR to `main` is pending. Nothing has been walked in a client.
- Ledger: [omega-work-packets.md](../omega-work-packets.md): findings F1-F44, decisions D-UO1..D-UO38, § Packets (statuses current), § HELD and blocked.
- On `main`: PRs #1882 (concept art), #1890, #1902, #1908, #1911 and the close-out PR (UO-13 tests, UO-99 docs in overnight form).
- On the campaign branch, all merged locally: W6 and W7 seals and scaffold (UO-15..UO-26), W8 vendors (UO-11, UO-12, UO-27..UO-30), W9 services (UO-31..UO-35), W10 Asgard seal, placements and stores (UO-37..UO-40), UO-98 docs. UO-36 is skipped (D-UO32 (a)); UO-41 and UO-42 are stubs, not built.
- What exists: the beam, Modi, Human and Free Jaffa induction, the re-offer, banker, registrars, respawners 180 and 181, 22 ambient NPCs, five class trainers, Human and Jaffa weapons, armor, crafting and consumables vendors, the Black Market auctioneer, the Castle Medic, eight crafting guides, the Asgard trainer and the Asgard Programs, Armor and Crafting vendors.

## Tested and not tested

- Tested: live-database tests (`live_db_union_omega_*`, `spawner_tests/union_omega`, `chain_replay_tests/union_omega`) with coder revert proofs checked by the wave reviews (a few proofs were not run; the review reports name them).
- Never tested: any lab or client row. UO-U1 R1-R5 is the owner's lab run and is pending, so the docs list every R5 row as UATPending. Asgard has no playable start (F42), so the Asgard rows cannot run at all.

## Owner calls

- D-UO32 (a) (no inn or battlemaster) and D-UO33 (6964 `Castle Medic`): decided by the owner.
- D-UO34: architect default, owner may overrule: 1522 sealed as shipped; Otin does not speak (63859, 63837 reserved, unused). Blocks nothing.
- Overrulable architect calls: D-UO28..D-UO31, D-UO34..D-UO38.
- Finding for review: every Omega vendor's sell list is list 2, so nothing bought on the hub sells back (UO-U1 S14).

## Next action

1. Open the campaign-branch PR to `main` (ship with `ship pr -C`), with the doc-update-map rows from the UO-98 report in the body.
2. The owner runs UO-U1 R1-R5 in the lab; then rerun UO-99's deferred rows (`ring-transport-system.md`, `gap-analysis/new-systems.md`, if H4 passes) and add FAIL or BLOCKED rows to the UAT known issues.
3. If D-UO34 is overruled or an Asgard start lands: the architect writes UO-41/UO-42 and the prologue packets (UO-43..UO-49).

## Rules for workers

PowerShell only; cargo only through the lane; CRLF docs; no stash, prune or push; no db/scripts migrations; agents never use the lab. F42: no playable Asgard start.
