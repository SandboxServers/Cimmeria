# Harset Tag Registry

> Type: reference. Audience: every Harset seed packet and the M0 placement session. Owner: coordinator.
> Created 2026-09-18 so that mission chains authored before the M0 spawn rows exist and the spawn rows seeded after M0 agree on one tag per entity.

## Rules

- `spawnlist.tag` and `interact_tag` / `entity_dead_tag` event keys are byte-exact matches. Use the tags below verbatim; never invent a variant.
- Shared-hub NPCs (worlds 57 and 68) get one tag each. Mission-scoped instance spawns (worlds 69 and 70, created by `spawn_entity`) are tagged per mission so `entity_dead_tag` counters cannot cross missions; each NPC in a wave gets its own tag (H03: exact match, no prefix matching).
- Existing rows keep their current tags: the DHD (spawn 4), ring switches 127-130 and 37 (`HarsetRingLeftBottom`, `HarsetRingRightBottom`, `HarsetRingLeft`, `HarsetRingLeftTop`, `HarsetRingRight`), Anat 222, Petbe 223, `FirstBug` 224. Read `db/resources/Worlds/Seed/spawnlist.sql` for Anat's and Petbe's exact tags before referencing them.
- Dialog-set binding (`add_dialog_set`) targets a template id, not a tag; the tag matters for `interact_tag` chains and for restore chains only.

## World 57 Harset (hub, shared)

| Tag | Template | Notes |
|---|---|---|
| `Harset_Hansen` | 212 | left of the gate, walking outward (spec) |
| `Harset_Jacobs` | 213 | with Hansen |
| `Harset_Lorak` | 201 | bazaar |
| `Harset_Blackstock` | 214 | if placed in 57; otherwise `CmdCenter_Blackstock` |
| `Harset_FormerRaJaffa` | 204 | Jaffa Zone (1326 Lan'toc target) |
| `Harset_SuspiciousJaffa` | 205 | Jaffa Zone (1371 / 1322 arrest target) |
| `Harset_HaughtyGoauld` | 210 | Market side, 1374 |
| `Harset_AngryJaffa` | 206 | Storage side, 1374 |
| `Harset_StorageLotaur` | 219 | bank anchor, prop-like |
| `Harset_Lethander` | 46 | hub stall (H24, H46) |
| `SecondBug`, `ThirdBug` | 164 | 742 baskets (pairs with the existing `FirstBug`) |
| `Harset_ShieldTower1` .. `Harset_ShieldTower3` | 243 | 1240 |
| `Harset_ShieldControls` | 248 | 1374 anchor |
| `Harset_BankAnchor`, `Harset_BarAnchor` | 248 | 1352 / 1374 anchors |
| `Harset_PetbeQuarters` | 244 | 1244 search object, 1243 exterior anchor |
| `Harset_OpsCenterAnchor`, `Harset_ResearchAnchor`, `Harset_GuardhouseAnchor`, `Harset_ScienceTentAnchor` | 248 | 1362 objectives 4658-4661, reused by 1410 |
| `Harset_Scarab_OpsYard`, `Harset_Scarab_OverflowYard`, `Harset_Scarab_BlackstockOffice`, `Harset_Scarab_PetbeExterior`, `Harset_Scarab_Fountain`, `Harset_Scarab_Bookseller` | 240 | 1243 objectives 4182-4187 |

## World 68 Harset_CmdCenter (shared)

| Tag | Template | Notes |
|---|---|---|
| `CmdCenter_Baal` | 42 | |
| `CmdCenter_Mohkatan` | 54 | |
| `CmdCenter_Marsh` | 10 | second spawn of the Cellblock template (D-H16) |
| `CmdCenter_Copplemann` | 48 | |
| `CmdCenter_Nerus` | 53 | lab |
| `CmdCenter_Opheltes` | 215 | |
| `CmdCenter_RoyalGuard` | 209 | near Anat |
| `CmdCenter_Blackstock` | 214 | office (if not in 57) |
| `CmdCenter_SymbioteTank` | 245 | 1353 / 741 |
| `CmdCenter_Athena` | 44 | 1363 tag target |

## Worlds 69 / 70 (per-player instances, `spawn_entity` tags)

| Tag | Template | Mission |
|---|---|---|
| `Rinla_Malac` | 200 | 1325 |
| `Enemies_Infiltrator1`, `Enemies_Infiltrator2` | 203 | 1343 |
| `Assault_Lethander`, `Assault_Attacker1` .. `Assault_AttackerN` | 46 / 208 | 1348 |
| `Murder_Crogan` | 222 | 1352 |
| `Tollan_Trooper1` .. `Tollan_Trooper3`, `Tollan_Dawson` | 217 / 223 | 1365 |
| `Moles_Lethander`, `Moles_NID1` .. `Moles_NIDN` | 46 / 218 | 1375 |
| `Moles2_Grogan`, `Moles2_NID`, `Moles2_Container1` .. `Moles2_Container3` | 222 / 218 / 242 | 1580 |
| `Replitech_Device`, `Replitech_Crate1` .. `Replitech_Crate3` | 247 / 241 | 1377 |
| `Profiling_Brahin` / `Vendetta_Brahin` | 202 | 1371 / 1322 |
| `Spy_Dawson`, `Spy_Witness1`, `Spy_Witness2` | 223 / 217 | 741 |
| `Infiltrators_Ashrak` | 211 | 1240 |
| `Invasion_Infiltrator1` .. `Invasion_InfiltratorN`, `Invasion_Beacon` | 203 / 246 | 1241 |
| `Storage_Petbe` | 221 | 1245 (ledger name kept) |
| `Patsy_Lethander` | 46 | 1247 |
| `Murderer_Contact` | 220 | 1246 |

Add rows here before using a new tag anywhere; the coordinator merges this file.
