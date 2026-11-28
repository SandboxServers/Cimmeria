# NPC combat V2: resume card

Campaign folder: `docs/analysis/npc-combat-v2/`. Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md) (waves 0-1, NR-10), [work-packets-batches.md](work-packets-batches.md) (waves 2-6).

## State (2026-11-27)

- Scoped by the architect. Nothing dispatched.
- V1 closes through NC-51. V1.1 ([npc-combat-v1-1](../npc-combat-v1-1/README.md)) runs alongside it, and V2 depends on it in two places:
  - NR-03 waits for NX-03;
  - NR-08 waits for NX-04 and NX-05.
- Goal: give each hostile NPC family an attack backed by evidence, in evidence order. The rest hold fire, pinned by tests. The backlog is 21 placeholder rows and 266 unresolved rows.

## Next action

1. Check for a live peer on `npc-combat-v2/*` (campaign-packet skill § 0).
2. Dispatch NR-01, NR-02 and NR-04..07 together, after `ops` makes their worktrees.
3. Dispatch NR-03 after V1.1 NX-03 merges, and NR-08 after NX-04 and NX-05 merge.
4. Ask the owner D-NR1, D-NR2, D-NR3, D-NR5 and D-NR6 now. Ask D-NR7 after NR-07 and D-NR10 after NR-06.
5. When NR-01 and NR-04..07 merge, dispatch NR-10 (architect) for `batch-spec.md`. Waves 2-4 wait for it.

## In flight

| Packet | Worktree | Status |
|---|---|---|
| none | | |

## Open owner decisions (README § Decisions)

- D-NR1: Unas. Recommend hold fire until there is evidence.
- D-NR2: one set per creature. Recommend yes.
- D-NR3: Lenny's baby stays NULL.
- D-NR5: retire the 592 placeholder after B1. Recommend yes.
- D-NR6: unarmed hostile humanoids get 733 Fists Melee AA (STRONGLY INFERRED).
- D-NR7: Goa'uld kit only where there is evidence (after NR-07).
- D-NR10: Twilla Tree, research first (after NR-06).

## Gates and lab

Gates: A1 the Android has no AnimSet; A2 the only audio banks are drones and ScavDog; A3 the lab has no audio; A4 research needs the client tree; A5 new ability ids need V1.1 D-NX4. Lab: wave 5 only, 2-3 sessions, ask first each time (D-NR11).
