# Board handoff: npc-combat-reference (text for ops to post)

Campaign npc-combat-reference is closed (2026-11-28).

## What shipped

- Four reference NPCs fight with their own data-driven abilities at the Debug Area Z12 range: NID Guard 24 (559, effect 5310), Praxis Jaffa 159/160 (584, 710, 736, with the staff component), ScavDog 76 (set 6 = 1433), PRU 4 (221, effect 264).
- NPCs with no ability set idle and log a warning instead of firing Pistol Shot (592). D-NC13: only templates with a human firearm component keep 592; melee-only humans 28, 92, 1415, 1465 and deployable 400 idle; the gallery has 74 idle.
- V1 merged as PRs #2081 to #2094; releases cut 2026-11-28 (first up to #2093, second includes #2094).
- W4 lab runs (2026-11-27, server 086fc2710) passed with notes; no 592 anywhere in the session.

## What is owed

Rows in unified UAT, section "NPC combat reference": NID 20 s respawn, T2 Z9 lethal squad (18 s), visual rows (animation, muzzle effect, VFX, sound; not verified in a client, sound UNRESOLVED), blank PRU/ScavDog attacker name (K26), Jaffa damage tuning (effect 736 RECONSTRUCTED; 760 to 0 in about 10 s at 2 m; owner decision).

## Follow-ups

V1.1 (Horden, Straegis Titan) and V2 (remaining roster), scoped under docs/analysis/npc-combat-v1-1 and npc-combat-v2.

## Links

- Ledger: docs/analysis/npc-combat-reference/README.md
- Lab results: docs/analysis/npc-combat-reference/w4-lab-results.md
- UAT: docs/guides/unified-uat.md#npc-combat-reference
