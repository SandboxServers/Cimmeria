# Union Omega resume card

- Phase: reopened 2026-10-19 for the hub services and the Asgard work (waves W6-W11). W1-W5 are merged to `main`; nothing has been walked in a client.
- Ledger: [omega-work-packets.md](../omega-work-packets.md): findings F1-F44, decisions D-UO1..D-UO38, § ID map (re-carved by D-UO27), § Waves, § HELD and blocked.
- Merged to `main`: PR #1882 (concept art), #1890, #1902, #1908, #1911 and the close-out PR (UO-13 tests, UO-99 docs in overnight form).
- Campaign branch `union-omega/decisions-2026-10-19` (worktree `uo-decisions`): W6-W9 and UO-37..UO-40 merged into it (UO-36 skipped, D-UO32 a). Not yet a PR to `main`.
- What exists on the branch: the beam, Modi, Human and Free Jaffa induction, the re-offer, banker, registrars, respawners 180 and 181, 22 ambient NPCs, five class trainers, Human and Jaffa weapons, armor, crafting and consumables vendors, the Black Market auctioneer, the Castle Medic, eight crafting guides, the Asgard trainer and Asgard vendors.

## Waves

| Wave | Packets | State |
|---|---|---|
| W6, W7 seals | UO-15..UO-26 | merged on the campaign branch |
| W8 vendors, W9 services | UO-11, UO-12, UO-27..UO-35 (UO-36 skipped) | merged on the campaign branch |
| W10 Asgard | UO-37..UO-40 merged; UO-41, UO-42 stubs; UO-43..UO-49 reserved for the prologue | UO-41/UO-42 written in the W10 packet pass |
| W11 close | UO-U1 (the owner's lab run, § R5 refreshed 2026-10-19), then UO-98 | R5 Asgard rows wait on F42 |

## Owner calls

- D-UO32 (a) and D-UO33 (6964 `Castle Medic`): decided by the owner.
- D-UO34: architect default, owner may overrule: 1522 sealed as shipped; Otin does not speak (63859, 63837 reserved, unused). Blocks nothing.
- Overrulable architect calls: D-UO28..D-UO31, D-UO34..D-UO38.
- Finding for review: every Omega vendor's sell list is list 2 (designs 21, 55, 3437 only), so nothing bought on the hub sells back (UO-U1 S14).

## Next action

1. The architect writes the W10 packet pass: the prologue packets (UO-43..UO-49, with UO-48's P5 rows), then UO-41 and UO-42 in full.
2. The owner runs UO-U1 R1-R5 in the lab (agents never use it).
3. UO-98, then the campaign-branch PR to `main`.

## Rules for workers

PowerShell only; cargo only through the lane; CRLF docs; no stash, prune or push; no db/scripts migrations; agents never use the lab (the owner runs UO-U1). F42: no playable Asgard start, so the Asgard client rows wait.
