---
name: harset-zone-evidence
description: What the repo actually has for Harset (world 57/68/69/70) — one script (mission 742), zero chains, near-empty spawn table. Plus mission 742's full chain shape.
metadata:
  type: project
---

# Harset evidence baseline (established 2026-09-17, read-only audit)

## Script inventory — COMPLETE, nothing else exists

Exhaustive grep of `deprecated/` for Harset + 15 NPC names + the 36 spec
mission ids found exactly **four** Harset files:

| File | What it does |
|---|---|
| `deprecated/python/cell/spaces/Harset.py` (70 ln) | 5 ring transporters (tags `HarsetRingLeftBottom/RightBottom/Left/LeftTop/Right` → `transporters.get(4..8)`), + region `Harset.CommandCenterTransition` → `moveTo(0,0.355,-20, world='Harset_CmdCenter')` |
| `deprecated/python/cell/spaces/Harset_CmdCenter.py` (30 ln) | region `Harset_CmdCenter.HarsetTransition` → `moveTo(0,-67.600,-231, world='Harset')`. Nothing else. |
| `deprecated/python/cell/missions/Harset/GivingTheWallsEars.py` (261 ln) | mission 742, the ONLY Harset mission script |
| `deprecated/data-scripts/scripts/.../Harset*.script` | the Atrea node-graph sources for the above (566 ln for 742, with designer section comments) |

The whole Atrea tree is 18 mission scripts + 10 space scripts. Harset gets 1 + 2.
`deprecated/db-monolithic-sql/db-deprecated/resources.sql` has **no scripts
table** — mission scripts were never in the DB, so there is no second place to
look. **The "scripts are lost" premise is TRUE for Harset** (unlike Castle
Cellblock, where it was false).

## Mission 742 "Giving the Walls Ears" — full chain shape

Steps 2502→2506; objectives 2913/2914/2915. Section comments from the Atrea
source (`GivingTheWallsEars.script:486-490,565`): "Accepting the mission" →
"Equip Jaffa disguise" → "Talking to Petbe" → "Plant listening devices" →
"Talk with Anat again" → "Give the Device Map to Nerus".

1. `mission.accepted::742` → addDialog(tpl **163** Petbe, set 3129) + give 3x item 2820
2. `dialog.choice::2638` → give item 2819 (disguise), removeDialog(163, 3129), advance 2503
3. `item.use::2819` (`once=True`) → advance 2504, displayDialog 2637, addDialog(tpl **164**, set **1000000**)
4. `dialog_set.open::1000000` → read target's tag, compare to `FirstBug`/`SecondBug`/`ThirdBug`,
   completeObjective 2913/2914/2915 (gated on `getObjectiveStatus == Active`),
   counter→3 → advance 2505 + addDialog(tpl **43** Anat, set 3130). RemoveItems(2820) node
   is orphaned in the compiled python (`n30_trigger_In` has `if None and ...`).
5. `dialog.choice::2639` → removeDialog(43,3130), give item 2864 (map), advance 2506, addDialog(tpl **53** Nerus, set 3131)
6. `dialog.choice::2640` → removeItems(2864), complete 742

Template ids (`db/resources/Entities/Seed/entity_templates.sql`):
163 Petbe, **164 = "Merchant Basket (Giving The Walls Ears)" prop, not an NPC**,
43 Anat (speaker 944), 53 Nerus (speaker 943).

## Why 742 cannot run today even with chains

`db/resources/Worlds/Seed/spawnlist.sql` — world 57 has **22 spawns total**:
5 ring switches (tpl 3), 1 DHD (tpl 1), Petbe (tpl 163, spawn 223),
**one** basket tagged `FirstBug` (tpl 164, spawn 224), a loot-debug item (tpl 23),
an "Interaction Debug NPC - DO NOT USE" (tpl 25), 4 Praxis Jaffa Lieutenants
(tpl 159), 8 Praxis Jaffa Guards (tpl 160).

Missing from world 57: `SecondBug` + `ThirdBug` baskets, Nerus, and every other
named Harset NPC. **Anat is spawned in world 68 (Harset_CmdCenter), spawn 222** —
the only spawn in worlds 68/69/70; worlds 69 and 70 have **zero** spawns.

Entity templates that exist for spec-named NPCs (all with
`interaction_set_id = NULL`): 54 Moh'Katan (spk 945), 42 Ba'al (spk 942),
43 Anat (spk 944), 46 Lethander (spk 978), 53 Nerus (spk 943), 163 Petbe (spk NULL).
**No template at all:** Mala'c, Copplemann, Blackstock, Hansen, Lo'rak,
Opheltes, Grogan, Dawson, Bra'hin — though all of them have authored dialog
screens and most have `speakers` rows.

Spec's NPC numbers (945/957/941/944/3219) are **`speakers.speaker_id`**, not
template ids: 945 Moh'katan, 957 Mal'ac, 941 Colonel Marsh, 944 Anat,
3219 = a row with an empty name. 941 (Marsh) is *Castle Cellblock*, not Harset.

## The mission-offer path barely exists anywhere in the seed

- `dialogs.accepts_mission_id` is non-NULL for **exactly 1 of 5,412 rows**:
  dialog 2636 → mission 742 (`db/resources/Dialogs/Seed/dialogs.sql:10815`).
- `entity_interactions` has **exactly one row in the whole seed**
  (`db/resources/Worlds/Seed/entity_interactions.sql:7`): template 43 (Anat) →
  dialog_set_map 3127 → dialog 2636, gated `missions_not_accepted = '{742}'`.
- `dialog_set_maps.missions_completed` / `missions_not_accepted` are `'{}'` on
  **all 4,671 rows**.

So every mission accept in Cimmeria today comes from a chain's `accept_mission`
action. Don't expect DB-driven offers.

Harset dialog *content* is richly authored but orphaned: sets 525 (`CSI Harset`),
560 (`Harset Turn In`), 656/661 (`Dial Harset`), 947 (`Harset Supplies`) have
~95 fully-written screens with correct speakers and **no entity binding of any
kind**.

## What already exists that a restoration can lean on

- `point_sets.sql:67,69` — `Harset.CommandCenterTransition` (2078, world 57) and
  `Harset_CmdCenter.HarsetTransition` (2079, world 68) exist → `enter_region`
  chains can key on them today.
- `ring_transport_regions.sql` — regions 4-8 on world 57 with the right tags;
  ring FSM is data-driven and `trigger_transporter` has a working executor arm.
  So `Harset.py`'s 5 ring bindings are a 5-chain mechanical port.
- World ids: **57 Harset, 68 Harset_CmdCenter, 69 Harset_Market, 70 Harset_StorageRm**.
