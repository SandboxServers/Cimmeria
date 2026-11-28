# NPC combat V1.1: resume card

Campaign folder: `docs/analysis/npc-combat-v1-1/`. Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md) (waves 0-1), [work-packets-w2-w5.md](work-packets-w2-w5.md) (waves 2-5).

## State (2026-11-27)

- Scoped by the architect. Nothing dispatched, no worktree made.
- V1 (npc-combat-reference) closes through NC-51. V1.1 touches none of its files.
- Goal: restore Horden (template 72) and the Straegis Titan (template 79) in the Debug Area, with every fact classed and no generic substitute attack.

## Next action

1. Check for a live peer on `npc-combat-v1-1/*` (campaign-packet skill § 0).
2. Ask `ops` for worktrees `nt-01` .. `nt-09`, then dispatch waves 0 and 1 together (9 agents, disjoint files).
3. While they run, ask the owner D-NT1, D-NT2, D-NT3, D-NT7 and D-NT8. Ask D-NT4 after NT-08, D-NT5 after NT-05 and D-NT6 (b) after NT-04 and NT-07.
4. When NT-04..09 merge, dispatch NT-10 (architect) for `kit-spec.md`. Wave 2 waits for it.

## In flight

| Packet | Worktree | Status |
|---|---|---|
| none | | |

## Open owner decisions (README § Decisions)

- D-NT1: Horden animation source. Recommend: record gate G1 and ask about other builds.
- D-NT2: Horden combat. Recommend: hold fire, pinned by a test.
- D-NT3: Titan kit. Recommend: new Titan-only abilities 3500-3503, RECONSTRUCTED.
- D-NT4: ability_overrides category, if NT-08 shows the client blanks new ids.
- D-NT5: the tasker as a ground-point delayed area hit, unless NT-05 finds a minion.
- D-NT6: the s/D stance; owner only if it is a 50% health phase.
- D-NT7: lab verification by screenshots (`image: true`) plus an owner watch.
- D-NT8: loot table 40 with item 2041 on the Debug Area Titan spawn only.

## Gates and lab

Gates: G1 Horden has no animation in any local build; G2 neither creature has sound; G3 the lab has no audio; G4 the Titan kit was never cooked; G5 research needs the owner's client tree. Lab: wave 4 only; ask "may I use the lab now?" and wait for a yes (D-NT10).
