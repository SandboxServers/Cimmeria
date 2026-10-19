# Union Omega resume card

- Phase: closed for building, 2026-10-19. Everything buildable is merged; nothing has been walked in a client.
- Ledger: [omega-work-packets.md](../omega-work-packets.md) (findings F1-F35, decisions D-UO1..26, the overnight run ledger, the blockers list).
- Merged: PR #1882 (concept art), #1890 (UO-01, UO-02), #1902 (UO-00, UO-03, UO-04, UO-05, UO-06, UO-10), #1908 (UO-07, UO-08), #1911 (UO-09, UO-14), and the close-out PR (UO-13 tests, UO-99 docs).
- What exists: the beam, Modi, Human and Free Jaffa induction, the re-offer, a banker, two registrars, respawner 180 and 22 inert ambient NPCs. Docs: [mission-chains](../../../content/mission-chains.md#union-omega-worlds-18-and-80), [zone-audit](../../../content/zone-audit.md), [unified UAT](../../../guides/unified-uat.md#union-omega).
- Not built: the vendors UO-11 and UO-12 (BlockedDecision D-UO8, D-UO16). Stubs: UO-15 (U-D07, H-LBL) and UO-16 (U-D08, U-D11).
- HELD, owner to answer: D-UO8, D-UO16, D-UO17, D-UO18, D-UO19, D-UO11, the rest of U-D07, H-LBL, U-D08 and U-D11. Recommended options are in the ledger's § Blockers and owner decisions. The owner may overrule D-UO25 and D-UO26.
- Owner UAT left: UO-U1 (both factions, re-offer, services). Ask the user before any lab use. Recipe: [UO-U1.md](UO-U1.md); results go to `worknotes/UO-U1-results.md`, which does not exist yet. Owed observations: `interactions_count = 1` on every "adding dialog set" line, and "removing dialog set" before "completing mission".
- Next action: ask the owner for the D-UO8 and D-UO16 answers, then dispatch UO-11 and UO-12. After UO-U1 passes, rerun the UO-99 rows it gates (`ring-transport-system.md`, `gap-analysis/new-systems.md`).
- Doc debt: the UO-04 seed header comment still says O5645 and O5646 are the S4940 objectives to watch (older wording); the union-zone-planning README table still lists Omega as BlockedDependency.
- Rules for workers: PowerShell only, cargo only through the lane, CRLF docs, no stash, prune or push, no db/scripts migrations, ask before any lab use.
