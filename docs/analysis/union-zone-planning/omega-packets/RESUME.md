# Union Omega resume card

- Phase: reopened 2026-10-19 for the hub services and the Asgard work (waves W6-W11). W1-W5 are merged; nothing has been walked in a client.
- Ledger: [omega-work-packets.md](../omega-work-packets.md): findings F1-F44, decisions D-UO1..D-UO38, § ID map (re-carved by D-UO27), § Waves, § HELD and blocked.
- Merged: PR #1882 (concept art), #1890, #1902, #1908, #1911 and the close-out PR (UO-13 tests, UO-99 docs in overnight form).
- What exists: the beam, Modi, Human and Free Jaffa induction, the re-offer, a banker, two registrars, respawner 180 and 22 inert ambient NPCs.
- Owner answers 2026-10-19: D-UO8 B, D-UO16 B, D-UO17 c, D-UO18 b, U-D08/U-D11 (remote hearing plus a databank prologue issuing item 3399), D-UO19 c, D-UO11 default, D-UO20/25/26 accepted, U-D07 rewards, H-LBL no label.

## Waves (each ships as one PR from integration worktree `uo-w<N>`, merge-noff in this order)

| Wave | Packets in merge order | Gate |
|---|---|---|
| W6 seals A (docs) | UO-15, UO-16, UO-17, UO-18, UO-19, UO-20 | Ready now; UO-20 starts after UO-19 returns |
| W7 stock seals + scaffold | UO-24, UO-25, UO-26, UO-21, UO-22, UO-23 | Ready now; UO-25/UO-26 branch at UO-24's commit |
| W8 vendors | UO-11, UO-12, UO-27, UO-28, UO-29, UO-30 | W6 and W7 merged |
| W9 services | UO-31, UO-32, UO-33, UO-34, UO-35, UO-36 (only on D-UO32 b) | W6, W7 merged; ships after the W8 PR |
| W10 Asgard | UO-37, UO-38, UO-39, UO-40 (UO-41, UO-42 stubs) | UO-16 merged; `uo-w10` cut after W8; UO-40 branches at UO-39's commit |
| W11 close | UO-U1 § R5 (the owner's lab run), then UO-98 | W8 and W9 merged |

## Open owner calls (BlockedDecision)

- D-UO32: inn and battlemaster, (a) build nothing (recommended) or (b) bark-only placeholders (UO-36).
- D-UO33: medic name; UO-33 uses 6964 `Castle Medic` unless answered before W9.
- D-UO34: mission 1522 sealed, replaced or offered, and does Otin speak; asked after UO-16 merges. Blocks UO-41, UO-42.
- Overrulable architect calls: D-UO28..D-UO31, D-UO35..D-UO38.

## Next action

Dispatch W6 and W7 together (12 worktrees). Then W8, W9 and W10 per the gates above.

## Rules for workers

PowerShell only; cargo only through the lane; CRLF docs; no stash, prune or push; no db/scripts migrations; agents never use the lab (the owner runs UO-U1). F42: no playable Asgard start, so the Asgard client rows wait.
