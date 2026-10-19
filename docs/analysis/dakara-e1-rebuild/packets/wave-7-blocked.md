# Dakara_E1 wave 7 (blocked): the fights

> Type: explanation and stubs. Audience: the owner (D-DK5), the orchestrator and the architect who expands this wave.
> Written 2026-10-19 (architect, autonomous). Ledger: [README](../README.md#packets). Contract: [work-packets.md](../work-packets.md).

**Do not dispatch.** Every packet here is `BlockedDecision` on **D-DK5** (how the Dakara fights work), and GDK1 also waits for the owner's M2 playtest. When D-DK5 is answered, the architect expands GDK1 first, then DK-20, DK-21 and DK-32b from GDK1's record, in the format of [wave-5.md](wave-5.md).

## Why the fights are blocked

Missions 1647 "Enemy at the Gates" and 1653 "Superweapon" need hostiles (1652 needs none: DK-31b ships it). What the engine can and cannot do today, which shapes every option:

| Fact | Consequence |
|---|---|
| `spawn_entity` on world 61 (`Dakara_E1`, not instanced) is refused unless `allow_shared: true`; with it, the spawn lands in the one shared space and its tag is a world-wide lock | Two players on the same fight step at once collide: the second player's spawn is refused while the first one's hostile (or its corpse) is live |
| Spawns are one-shot: no respawn, no relog restore | Each stage needs a step-gated restore chain |
| `entity_dead_tag` fires only for a player-credited kill, and credits the last hitter only | In a party only the killer's step advances |
| Kill counters live in memory and reset on relog or world change | A fight that spans a relog restarts its count; carry progress in steps or objectives |
| No timer trigger, no player-death trigger, no fail verb | "Defend for N seconds" or "fail if you die" cannot be authored without an engine change; a death just respawns the player with the step still active |
| World 61 has no cover data, and the hostile spawn spots PL-DK-A-10/A-11 sit near the navmesh 279/324 gap | Hostiles may path badly; GDK1 checks the spots |
| No reward (D-DK7) | The fights give nothing beyond story progress |

## D-DK5 options (owner)

| Option | What it means | Cost | Recommendation |
|---|---|---|---|
| **A. Castle precedent** | Real fights. GDK1 picks per fight between static respawning hostiles on world 61 (an ordinary MMO camp; any player's kills count through `entity_dead_tag` and a step-gated counter) and per-player `allow_shared` spawns (one player at a time). Kill objectives complete from death chains | GDK1 plus three to five packets; two engine gaps stay (no timer, no fail) | **Recommended**, after the M2 playtest shows the cast and the tent work |
| B. Peaceful placeholders | Each kill objective becomes a click or a region at the fight spot, with a narrator line; swapped for A later | Two to three packets now, rework later | Only if the owner wants the arc playable end to end before combat is designed |
| C. Stop the arc at 1646 | Ship nothing past 1646 for now; waves 5 and 6 stay dormant | Nothing | Not recommended: it strands the finished wave-5 and wave-6 chains |
| D. Skip the fights | 1647 and 1653's kill steps advance from dialogs alone (Bra'tac narrates the battle) | One to two packets | Not recommended: the story's central battle becomes a dialog box |

Whatever the answer, 1653 already ships a peaceful path (DK-32a: courtyard, supplies, radio, report); DK-32b only adds the guards.

## GDK1: encounter design record

| Field | Value |
|---|---|
| Status | BlockedDecision (D-DK5), then the M2 playtest |
| Role | `npc-ai-spawn-advisor` with `combat-systems-advisor` (read-only second pass) |
| Output | `docs/analysis/dakara-e1-rebuild/worknotes/GDK1.md`: a decision record, nothing seeded |
| Size | About 35k tokens |

It decides, for 1647 (steps 4911, 4912, 4910, 4916, 4915, 4913, 4917, 4914) and 1653 step 4938's guards: the hostile templates and their kit (template 35 as the base), counts, level band, static spawn or chain spawn per stage, each stage's trigger and how its objective completes, what happens on a relog or a death mid-fight, and the spawn points (from PL-DK-A-10 to A-16, checked against navmesh component 279). It ends with the packet list for DK-20, DK-21 and DK-32b, each about 40k tokens.

## DK-20: hostile roster

| Field | Value |
|---|---|
| Status | BlockedDecision (D-DK5), then GDK1 |
| Role | `packet-coder` |
| Ids | templates 464-479 (462 and 463 are DK-41's props); spawns from 8022 (8016-8021 are DK-41's); spawn sets and point sets from the next free ids in C2 |
| Gate | GDK1's record |

## DK-21: mission 1647 "Enemy at the Gates"

| Field | Value |
|---|---|
| Status | BlockedDecision (D-DK5), then GDK1 and DK-20 |
| Role | `packet-coder`, split per GDK1 (likely DK-21a offer and story, DK-21b fight) |
| Ids | chains 8390-8429 (reserved, D-DK22); dialog set 1834; dialogs 5982, 5822-5827, 6106, 6107; 4916's radio through `display_radio_dialog` |
| Gate | DK-20 merged. Merging DK-21 wakes waves 5 and 6: 1648's offer fires on `done 1647` |

## DK-32b: the courtyard guards (mission 1653)

| Field | Value |
|---|---|
| Status | BlockedDecision (D-DK5), then GDK1 and DK-20 |
| Role | `packet-coder` |
| Ids | chains 8450-8479 (reserved, D-DK22) |
| Scope | Guards around the two power supplies (PL-DK-A-14 to A-16). If GDK1 gates the supplies on the guards' deaths, DK-32b also edits DK-32a's chains 8354-8357 (one extra condition each) and their tests; say so in its packet |

## Not scheduled

- **The vocuum, 1653 objective O5643 (optional).** Nothing is built for it. It needs a prop and a placement the owner picks; one small packet after that.
- **DK-31c, the Ha'tak sequences for 1652.** Not blocked by the owner: the architect expands it from DK-31a's note (wave 6) into one small packet that adds `play_sequence` rows to DK-31b's chains.
