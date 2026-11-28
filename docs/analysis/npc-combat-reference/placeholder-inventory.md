# NPC combat reference: Pistol Shot placeholder inventory (NC-15)

Packet: NC-15. Campaign: `docs/analysis/npc-combat-reference/`. Read with
[work-packets-w1-w2.md](work-packets-w1-w2.md) § NC-15 and the resolver contract
C1 in [work-packets.md](work-packets.md). This note is inventory only: no code
changes. `PLACEHOLDER_POLICY` stays `Legacy` until D-NC1 (NC-30). Snapshot as of NC-15: NC-30 has since flipped the policy to `RequireWeaponEvidence` (D-NC1).

## Summary

- Entity templates in the seeds: 566. With a NULL `ability_set_id`: 506.
  Of those, 36 carry a `WP-Human.*` component and would keep 592 under
  `RequireWeaponEvidence` (D-NC1). After NC-50b (D-NC13) only the firearm
  whitelist keeps it: 31 do; rows 28, 92, 400, 1415 and 1465 now hold fire.
  The other 470 would hold fire.
- Production callers of `SpaceManager::spawn_npc`: 0.
  All 278 call sites sit behind a `#[cfg(test)]` module or fn, or in a test file.
- Tests that spawn an NPC and mention 592 or `NPC_DEFAULT_ABILITY`: 42 candidates (6 cell-world, 5 cell-combat, 31 other). Of these, 3 need an NC-21 fix (2 cell-world, 1 other), 6 are unaffected and 33 need no fix.

## Method

- Templates: every `INSERT INTO entity_templates` row in `db/resources/Entities/Seed/entity_templates*.sql`,
  parsed with `python -I`. Spawn counts come from every `INSERT INTO spawnlist` row in
  `db/resources/Worlds/Seed/spawnlist*.sql`. No `UPDATE` touches either table.
- Body family is from the `body_set` prefix (`BS_Human*`, `BS_Jaffa*`, `BS_Goauld*`, `MOB_*`).
  A `MOB_*` body whose name holds `Drone` or `Walker` counts as mechanical.
- Tags: `interactive` when `interaction_type` is non-zero or `interaction_set_id` is set;
  `vendor` when any buy, sell, repair or recharge list is set; `trainer` when
  `trainer_ability_list_id` is set. A scripted flag has no field in the seed, so it is not shown.
- Verdict: `keeps 592` when a component starts with `WP-Human.`, else `holds fire`
  (the C1 `RequireWeaponEvidence` rule). NC-50b (D-NC13) narrows this to the
  firearm whitelist `HUMAN_FIREARM_PREFIXES`; the rows edited for NC-50b say so.
- Callers and fallback tests: a scanner that strips comments and literals, then matches
  brace-delimited fn and `cfg(test)` module bodies. It is a heuristic, not a compile. Counts are taken at origin/main d780348e4 plus NC-10's tests.
- Evidence classes (README § Evidence classifications): the template rows, components, factions and spawn counts are EARLIEST LEGACY DATA (the `db/resources` seeds). The caller and test lists, and what `spawn_npc` adds, are CURRENT CIMMERIA IMPLEMENTATION (code, heuristic scan). The `keeps 592` / `holds fire` verdict is RECONSTRUCTED (C1 rule applied to seed components; a prediction for NC-30). The `scripted` flag is UNRESOLVED (no seed column). No row is ORIGINAL CLIENT VERIFIED or STRONGLY INFERRED.

## 1. Templates with NULL `ability_set_id`

| Template | Name | Body family | `WP-*` component | Faction | Class | Flags | Tags | Worlds and spawns | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| 1 | DHD | other | none | - | spawnable | 0 | interactive | w1300 x1, w15 x1, w18 x1, w19 x1, w23 x1, w57 x1, w61 x1, w72 x1, w73 x1, w77 x1 | holds fire |
| 3 | Ring Transporter Switch | other | none | 1 | being | 4 | interactive | w12 x4, w15 x11, w18 x5, w19 x2, w23 x5, w57 x5, w77 x24, w80 x1 | holds fire |
| 8 | SMG-1A | other | none | - | spawnable | 4 | interactive | w12 x1 | holds fire |
| 10 | Col Marsh (pet) | Human | WP-Human.WP_SMG_1A | 3 | being | 24 | - | w12 x1, w68 x1, w8 x1 | keeps 592 |
| 13 | Wooden crate | other | none | - | spawnable | 0 | - | w12 x1 | holds fire |
| 14 | Cpt frost corpse | other | none | 1 | being | 0 | - | w12 x1 | holds fire |
| 17 | Prisoner 329 | Goa'uld | none | 3 | mob | 0 | - | w12 x1 | holds fire |
| 18 | Ambernol | other | none | - | spawnable | 4 | - | w12 x1 | holds fire |
| 19 | View screen 2 | other | none | 1 | being | 4 | - | w12 x2 | holds fire |
| 21 | Cellblock guard corpse | other | none | - | spawnable | 0 | - | w12 x1 | holds fire |
| 22 | Door control panel | other | none | - | being | 0 | - | w12 x1 | holds fire |
| 23 | Loot debug item | other | none | 1 | mob | 4 | - | none | holds fire |
| 25 | Interaction Debug NPC - DO NOT USE | Human | none | 1 | mob | 0 | vendor, trainer | none | holds fire |
| 26 | TestAvatar | Human | none | - | mob | 0 | - | none | holds fire |
| 28 | test avatar set - DO NOT USE | Human | WP-Human.WP_HandToHand_1A | - | being | 0 | - | none | holds fire (NC-50b) |
| 29 | General Hammond | Human | none | 1 | mob | 0 | - | w58 x1 | holds fire |
| 30 | Teal'c | Jaffa | none | 1 | mob | 0 | - | w58 x1 | holds fire |
| 31 | Airman | Human | none | 1 | mob | 0 | - | w58 x2 | holds fire |
| 32 | Elevator controls | other | none | 1 | being | 0 | - | w58 x3 | holds fire |
| 33 | Sam Carter | Human | none | 1 | mob | 0 | - | w58 x1 | holds fire |
| 34 | SGC Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 10 | mob | 0 | - | w1300 x1, w58 x2 | holds fire |
| 35 | SGC Ba'al Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 10 | mob | 0 | - | w1300 x2, w58 x11 | holds fire |
| 36 | SGC Crate | other | none | - | being | 0 | - | w58 x1 | holds fire |
| 37 | SGC Gate Control | other | none | 1 | being | 4 | interactive | w58 x1 | holds fire |
| 38 | SGC_W1 Bomb | other | none | 1 | being | 4 | - | w58 x1 | holds fire |
| 39 | SGC_W1 Firearm | other | none | 1 | being | 4 | - | w58 x1 | holds fire |
| 40 | SGC_W1 Airman corpse | other | none | 1 | being | 4 | - | w58 x1 | holds fire |
| 41 | Ra | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 42 | Ba'al | Goa'uld | none | 1 | mob | 0 | - | w68 x1 | holds fire |
| 43 | Anat | Goa'uld | none | 1 | mob | 0 | - | w68 x1 | holds fire |
| 44 | Athena | Goa'uld | none | 1 | mob | 0 | - | w68 x1 | holds fire |
| 45 | Morrigan | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 46 | Lethander | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 47 | Mr Woolsey | Human | none | 1 | mob | 0 | - | none | holds fire |
| 48 | CaptCoppleman | Human | none | 1 | mob | 0 | - | w68 x1, w8 x1 | holds fire |
| 49 | Oma Desala | Human | none | 1 | mob | 0 | - | none | holds fire |
| 50 | Vala Mal Doran | Human | none | 1 | mob | 0 | - | none | holds fire |
| 51 | Daniel Jackson | Human | none | 1 | mob | 0 | - | none | holds fire |
| 52 | Jack O'Neill | Human | none | 1 | mob | 0 | - | none | holds fire |
| 53 | Nerus | Human | none | 1 | mob | 0 | - | w68 x1 | holds fire |
| 54 | Moh'Katan | Jaffa | none | 1 | mob | 0 | - | w62 x1, w68 x1, w8 x1 | holds fire |
| 55 | Warrick | Human | none | 1 | mob | 0 | - | none | holds fire |
| 56 | Goldam | Human | none | 1 | mob | 0 | - | none | holds fire |
| 57 | Major Davis | Human | none | 1 | mob | 0 | - | none | holds fire |
| 58 | Walter Harriman | Human | none | 1 | mob | 0 | - | w58 x1 | holds fire |
| 59 | Bra'tak | Jaffa | none | 1 | mob | 0 | - | w62 x1 | holds fire |
| 60 | Ra 2 | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 61 | Ra 3 | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 62 | Ra 4 | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 63 | Ra 5 | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 64 | Thor | other | none | 1 | mob | 0 | - | none | holds fire |
| 65 | Modi | other | none | 1 | mob | 0 | - | w18 x1 | holds fire |
| 66 | Nox Child Male | other | none | 1 | mob | 0 | - | none | holds fire |
| 67 | Nox Child Female | other | none | 1 | mob | 0 | - | none | holds fire |
| 68 | Blix | other | none | 1 | mob | 0 | - | none | holds fire |
| 69 | Malfunctioning Drone | mechanical Drone/Walker | none | 1 | mob | 0 | - | none | holds fire |
| 70 | BattleWalker | mechanical Drone/Walker | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 71 | Carnosaur | creature MOB_Carnosaur | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 72 | Horden | creature MOB_Horden | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 73 | Lenny | creature MOB_Lenny | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 74 | AMBRat | creature MOB_AMBRat | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 75 | Rhinolion | creature MOB_Rhinolion | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 76 | ScavDog | creature MOB_ScavDog | none | 10 | mob | 0 | - | w1300 x2 | holds fire |
| 77 | Straegis Beacon | creature MOB_StraegisBeacon | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 78 | Straegis Fighter | creature MOB_StraegisFighter | none | 10 | mob | 0 | - | w1300 x2 | holds fire |
| 79 | Straegis Titan | creature MOB_StraegisTitan | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 80 | Twilla Tree | creature MOB_TwillaTree | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 81 | Ancient Drone | mechanical Drone/Walker | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 82 | Bull Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 83 | Asian Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 84 | Cat Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 85 | Cobra Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 86 | Croc Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 87 | Demon Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 88 | Dragon Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 89 | Eagle Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 90 | Falcon Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 91 | Horse Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 92 | Hyena Jaffa | Jaffa | WP-Human.WP_HandToHand_1A | 10 | mob | 0 | - | w1300 x1 | holds fire (NC-50b) |
| 93 | Jackal Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 94 | Mayan Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 95 | Morrigan Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 96 | Naga Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 97 | Praxis Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 98 | Praxis Jaffa 2 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 99 | Ra Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 100 | Standard Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 101 | Savarog Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 102 | Tiki Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 105 | Unas_1 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 106 | Unas_2 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 107 | Unas_3 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 108 | Unas_4 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 109 | Unas_5 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 110 | Unas_6 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 111 | Viking Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 112 | Asian Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 113 | Bull Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 114 | Cat Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 115 | Cobra Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 116 | Croc Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 117 | Demon Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 118 | Dragon Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 119 | Eagle Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 120 | Falcon Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 121 | Horse Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 122 | Hyena Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 123 | Jackal Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 124 | Mayan Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 125 | Morrigan Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 126 | Naga Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 127 | Praxis Jaffa 2 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 128 | Praxis Jaffa 1 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 129 | Standard Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 130 | Svarog Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 131 | Tiki Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 132 | Unas 1 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 133 | Unas 2 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 134 | Unas 3 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 135 | Unas 4 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 136 | Unas 5 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 137 | Unas 6 Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 138 | Viking Jaffa Female | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 139 | Clothed Jaffa Female 1 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 140 | NPC Child 1 | other | none | 10 | mob | 0 | - | none | holds fire |
| 141 | NPC Child 2 | other | none | 10 | mob | 0 | - | none | holds fire |
| 142 | Ra Jaffa 2 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 143 | Ra's Officer | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 144 | Ra's Jaffa | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 147 | Castle - Access Panel | other | none | - | being | 0 | - | w8 x1 | holds fire |
| 149 | Sgt. Gerschon | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 150 | HumanMale - Not For Use | Human | none | 1 | mob | 0 | - | none | holds fire |
| 151 | Lucian - Blue Faction Scientist | Human | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 152 | Lucian - Slum Dweller | Human | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 153 | HumanFemale Template - DO NOT USE | Human | none | 1 | mob | 0 | - | none | holds fire |
| 154 | JaffaFemale Template - DO NOT USE | Jaffa | none | 1 | mob | 0 | - | none | holds fire |
| 155 | JaffaMale Template - DO NOT USE | Jaffa | none | 1 | mob | 0 | - | none | holds fire |
| 156 | Asgard Template - DO NOT USE | other | none | 1 | mob | 0 | - | none | holds fire |
| 157 | GoauldFemale Template - DO NOT USE | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 158 | GoauldMale Template - DO NOT USE | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 162 | DHD_Frost | other | none | - | spawnable | 0 | interactive | w8 x1 | holds fire |
| 163 | Petbe | Jaffa | none | 1 | mob | 0 | - | w57 x1 | holds fire |
| 164 | Merchant Basket (Giving The Walls Ears) | other | none | 1 | being | 0 | - | w57 x3 | holds fire |
| 166 | Sandbox Greeting NPC | Human | none | 1 | mob | 0 | interactive | w2 x1 | holds fire |
| 167 | Sandbox Ba'al | Goa'uld | none | 1 | mob | 0 | interactive | none | holds fire |
| 168 | Castle_Zuritska | Human | none | 1 | mob | 0 | - | w8 x2 | holds fire |
| 172 | Castle_SurrenderGuard | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 173 | Castle_CommsTerminal | other | none | - | being | 0 | - | w8 x1 | holds fire |
| 174 | Castle_OpCoreSoldier | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w8 x2 | keeps 592 |
| 175 | Castle_OpCoreSoldier_Wander | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w8 x2 | keeps 592 |
| 176 | Castle_OpCoreSoldier_Unarmed | Human | none | 1 | mob | 0 | - | w12 x2, w8 x6 | holds fire |
| 177 | Castle_Medic | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 178 | Castle_SgtStanton | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | none | keeps 592 |
| 179 | Castle_Ogilvie | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 180 | Castle_OpCoreSoldierCorpse | other | none | 1 | being | 0 | - | w8 x3 | holds fire |
| 187 | Castle_OpCoreSoldier_Standoff | Human | WP-Human.WP_SMG_1A | 3 | mob | 0 | - | w8 x4 | keeps 592 |
| 188 | Castle_SgtStanton_Standoff | Human | WP-Human.WP_SMG_1A | 3 | mob | 0 | - | w8 x1 | keeps 592 |
| 240 | Harset Camera Location | other | none | 1 | being | 4 | - | none | holds fire |
| 241 | Replitech Crate | other | none | 1 | being | 4 | - | none | holds fire |
| 242 | Harset Storage Crate | other | none | 1 | being | 4 | - | none | holds fire |
| 243 | Harset Shield Tower | other | none | 1 | being | 4 | - | w57 x3 | holds fire |
| 244 | Petbe's Quarters Search Object | other | none | 1 | being | 4 | - | w57 x1 | holds fire |
| 245 | Anat's Symbiote Tank | other | none | 1 | being | 4 | - | w68 x1 | holds fire |
| 246 | Strange Beacon Technology | other | none | 1 | being | 4 | - | none | holds fire |
| 247 | Devlin's Device | other | none | 1 | being | 4 | - | none | holds fire |
| 248 | Harset Monitoring Device Anchor | other | none | 1 | being | 4 | - | w57 x2 | holds fire |
| 261 | Harset Command Center Door Control (PROVISIONAL) | other | none | 1 | being | 0 | - | w57 x1 | holds fire |
| 300 | Debug Hub - Vendor | Human | none | 1 | mob | 0 | interactive, vendor | w12 x1, w1300 x1 | holds fire |
| 301 | Debug Hub - Trainer | Human | none | 1 | mob | 0 | interactive, trainer | w12 x1, w1300 x1 | holds fire |
| 302 | Debug Hub - Dialog | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 303 | Debug Hub - Livewire Terminal | other | none | 1 | being | 4 | interactive | w12 x1, w1300 x1 | holds fire |
| 304 | Debug Hub - Loot Crate | other | none | - | spawnable | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 305 | Debug Hub - Black Market Auctioneer | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1, w18 x1 | holds fire |
| 310 | Debug Hub - BioMedical Crafting Station | other | none | 1 | being | 30724 | - | w12 x1, w1300 x1 | holds fire |
| 311 | Debug Hub - Electronics Crafting Station | other | none | 1 | being | 30724 | - | w12 x1, w1300 x1 | holds fire |
| 312 | Debug Hub - Power Systems Crafting Station | other | none | 1 | being | 30724 | - | w12 x1, w1300 x1 | holds fire |
| 313 | Debug Hub - Materials Crafting Station | other | none | 1 | being | 30724 | - | w12 x1, w1300 x1 | holds fire |
| 314 | Debug Hub - Crafting Supplies | Human | none | 1 | mob | 0 | interactive, vendor | w12 x1, w1300 x1 | holds fire |
| 330 | Debug Hub - Team Registrar | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 331 | Debug Hub - Command Registrar | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 360 | Debug Hub - Pet Trainer | Goa'uld | none | 1 | mob | 0 | interactive, trainer | w12 x1, w1300 x1 | holds fire |
| 370 | Debug Hub - Banker | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 371 | Debug Hub - Team Banker | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 372 | Debug Hub - Command Banker | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 390 | Debug Hub - Gate Mail Clerk | Human | none | 1 | mob | 0 | interactive | w12 x1, w1300 x1 | holds fire |
| 400 | Deployable: Microwave Emitter | other | WP-Human.Dp_Standard100 | 1 | being | 0 | - | none | holds fire (NC-50b) |
| 410 | Castle supply chest | other | none | - | spawnable | 0 | interactive | w8 x1 | holds fire |
| 411 | SGC_W1 Carter desk SMG | other | none | - | spawnable | 4 | - | w58 x1 | holds fire |
| 412 | Castle_MedDispenser | other | none | - | being | 0 | - | w8 x1 | holds fire |
| 413 | Castle - Symbiote Tank | other | none | 1 | being | 4 | interactive | w8 x1 | holds fire |
| 415 | Castle_Quartermaster | Human | none | 1 | mob | 0 | interactive, vendor | w8 x1 | holds fire |
| 416 | Castle file cabinet | other | none | - | spawnable | 0 | interactive | w8 x1 | holds fire |
| 417 | Castle_Overlooked_Prisoner | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 419 | Castle_GuardPostSentry | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w8 x1 | keeps 592 |
| 424 | Castle_HubSupplyOfficer | Human | none | 1 | mob | 0 | - | w8 x1 | holds fire |
| 425 | Castle_DefenseQ3_Giver | Human | WP-Human.WP_SMG_1A | 3 | mob | 0 | - | w8 x1 | keeps 592 |
| 441 | Dakara E1 - Rak'nor | Jaffa | none | 1 | mob | 0 | - | w61 x1 | holds fire |
| 443 | Dakara E1 - Ba'al (Hologram) | Goa'uld | none | 1 | mob | 0 | - | none | holds fire |
| 445 | Dakara E1 - Tent Flap (to Command Tent) | other | none | 1 | being | 4 | interactive | w61 x1 | holds fire |
| 446 | Dakara E1 - Tent Flap (to Moh'katan's Tent) | other | none | 1 | being | 4 | interactive | w61 x1 | holds fire |
| 447 | Dakara E1 - Tent Flap (from Command Tent) | other | none | 1 | being | 4 | interactive | w62 x1 | holds fire |
| 448 | Dakara E1 - Tent Flap (from Moh'katan's Tent) | other | none | 1 | being | 4 | interactive | w62 x1 | holds fire |
| 449 | Dakara E1 - Drop Location | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 450 | Dakara E1 - SG-18 Remains (Maj. Louis) | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 451 | Dakara E1 - SG-18 Remains (Lt. Nguyn) | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 452 | Dakara E1 - SG-18 Remains (Lt. Waters) | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 453 | Dakara E1 - Command Terminal | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 454 | Dakara E1 - Moh'katan's Terminal | other | none | 1 | being | 4 | - | w62 x1 | holds fire |
| 455 | Dakara E1 - Ring Control | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 456 | Dakara E1 - Turret Power Supply | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 457 | Dakara E1 - Vocuum | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 458 | Dakara E1 - Drop Location 2 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 459 | Dakara E1 - Drop Location 3 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 460 | Dakara E1 - Drop Location 4 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 461 | Dakara E1 - Drop Location 5 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 462 | Dakara E1 - Ring Control 2 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 463 | Dakara E1 - Turret Power Supply 2 | other | none | 1 | being | 4 | - | w61 x1 | holds fire |
| 1250 | Cellblock - Prison Boot Control | other | none | 1 | being | 4 | - | w12 x1 | holds fire |
| 1300 | Debug Area - Ability Granter | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1301 | Debug Area - Ability Reset | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1302 | Debug Area - Munitions Vendor | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1310 | Debug Area - Training Dummy L1 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1311 | Debug Area - Training Dummy L10 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1312 | Debug Area - Training Dummy L25 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1313 | Debug Area - Training Dummy L50 | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1314 | Debug Area - Friendly Training Dummy | Human | none | 9 | mob | 0 | - | w1300 x1 | holds fire |
| 1330 | DebugArea - Friendly (faction 1 World Object) | Human | none | 1 | mob | 0 | - | w1300 x2 | holds fire |
| 1331 | DebugArea - Friendly (faction 9 Friendly_Ambient) | Human | WP-Human.WP_SMG_1A | 9 | mob | 0 | - | w1300 x2 | keeps 592 |
| 1332 | DebugArea - Neutral (faction 7 Neutral_Ambient) | Human | none | 7 | mob | 0 | - | w1300 x2 | holds fire |
| 1333 | DebugArea - Neutral Jaffa (faction 10, spawn pinned NEUTRAL) | Jaffa | none | 10 | mob | 0 | - | w1300 x2 | holds fire |
| 1340 | DebugArea - Patroller | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1341 | DebugArea - Wanderer | Human | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1342 | DebugArea - Leash test | Jaffa | none | 10 | mob | 0 | - | w1300 x1 | holds fire |
| 1343 | DebugArea - Assist trio | Jaffa | none | 10 | mob | 0 | - | w1300 x3 | holds fire |
| 1400 | Debug Area - Ra | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1401 | Debug Area - Ba'al | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1402 | Debug Area - Anat | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1403 | Debug Area - Athena | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1404 | Debug Area - Morrigan | Goa'uld | WP-Goauld.WP_Ribbon_Elec_1A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1405 | Debug Area - Nerus | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1406 | Debug Area - Ra's Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1410 | DebugArea Visual Lineup - Col Marsh (pet) | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1411 | DebugArea Visual Lineup - Cellblock Guard | Human | WP-Human.WP_Pistol_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1412 | DebugArea Visual Lineup - NID Guard | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1413 | DebugArea Visual Lineup - Interaction Debug NPC - DO NOT USE | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1414 | DebugArea Visual Lineup - TestAvatar | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1415 | DebugArea Visual Lineup - test avatar set - DO NOT USE | Human | WP-Human.WP_HandToHand_1A | 1 | mob | 0 | - | w1300 x1 | holds fire (NC-50b) |
| 1416 | DebugArea Visual Lineup - General Hammond | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1417 | DebugArea Visual Lineup - Airman | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1418 | DebugArea Visual Lineup - Mr Woolsey | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1419 | DebugArea Visual Lineup - Daniel Jackson | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1420 | DebugArea Visual Lineup - Jack O'Neill | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1421 | DebugArea Visual Lineup - Nerus | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1422 | DebugArea Visual Lineup - Warrick | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1423 | DebugArea Visual Lineup - Goldam | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1424 | DebugArea Visual Lineup - Major Davis | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1425 | DebugArea Visual Lineup - Walter Harriman | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1426 | DebugArea Visual Lineup - NID Guard - Castle outside | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1427 | DebugArea Visual Lineup - Sgt. Gerschon | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1428 | DebugArea Visual Lineup - HumanMale - Not For Use | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1429 | DebugArea Visual Lineup - Lucian - Blue Faction Scientist | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1430 | DebugArea Visual Lineup - Lucian - Slum Dweller | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1431 | DebugArea Visual Lineup - Castle_Zuritska | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1432 | DebugArea Visual Lineup - Castle_SurrenderGuard | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1433 | DebugArea Visual Lineup - Castle_OpCoreSoldier | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1434 | DebugArea Visual Lineup - Castle_OpCoreSoldier_Wander | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1435 | DebugArea Visual Lineup - Castle_OpCoreSoldier_Unarmed | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1436 | DebugArea Visual Lineup - Castle_SgtStanton | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1437 | DebugArea Visual Lineup - Castle_Ogilvie | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1438 | DebugArea Visual Lineup - Opheltes | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1439 | DebugArea Visual Lineup - Debug Hub - Vendor | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1440 | DebugArea Visual Lineup - Debug Hub - Dialog | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1441 | DebugArea Visual Lineup - Debug Hub - Crafting Supplies | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1442 | DebugArea Visual Lineup - HM_Mesh.HM_BodySet (no template) | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1443 | DebugArea Visual Lineup - Sam Carter | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1444 | DebugArea Visual Lineup - CaptCoppleman | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1445 | DebugArea Visual Lineup - Oma Desala | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1446 | DebugArea Visual Lineup - Vala Mal Doran | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1447 | DebugArea Visual Lineup - HumanFemale Template - DO NOT USE | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1448 | DebugArea Visual Lineup - Castle_Muelbach | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1449 | DebugArea Visual Lineup - Castle_Medic | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1450 | DebugArea Visual Lineup - Storage Lo'taur | Human | WP-Human.WP_Pistol_1A | 1 | mob | 0 | - | w1300 x1 | keeps 592 |
| 1451 | DebugArea Visual Lineup - Debug Hub - Banker | Human | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1452 | DebugArea Visual Lineup - Teal'c | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1453 | DebugArea Visual Lineup - SGC Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1454 | DebugArea Visual Lineup - Bra'tak | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1455 | DebugArea Visual Lineup - Bull Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1456 | DebugArea Visual Lineup - Asian Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1457 | DebugArea Visual Lineup - Cat Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1458 | DebugArea Visual Lineup - Cobra Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1459 | DebugArea Visual Lineup - Croc Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1460 | DebugArea Visual Lineup - Demon Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1461 | DebugArea Visual Lineup - Dragon Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1462 | DebugArea Visual Lineup - Eagle Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1463 | DebugArea Visual Lineup - Falcon Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1464 | DebugArea Visual Lineup - Horse Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1465 | DebugArea Visual Lineup - Hyena Jaffa | Jaffa | WP-Human.WP_HandToHand_1A | 1 | mob | 0 | - | w1300 x1 | holds fire (NC-50b) |
| 1466 | DebugArea Visual Lineup - Jackal Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1467 | DebugArea Visual Lineup - Mayan Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1468 | DebugArea Visual Lineup - Morrigan Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1469 | DebugArea Visual Lineup - Naga Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1470 | DebugArea Visual Lineup - Praxis Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1471 | DebugArea Visual Lineup - Praxis Jaffa 2 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1472 | DebugArea Visual Lineup - Ra Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1473 | DebugArea Visual Lineup - Standard Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1474 | DebugArea Visual Lineup - Savarog Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1475 | DebugArea Visual Lineup - Tiki Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1476 | DebugArea Visual Lineup - Unas_1 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1477 | DebugArea Visual Lineup - Unas_2 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1478 | DebugArea Visual Lineup - Unas_3 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1479 | DebugArea Visual Lineup - Unas_4 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1480 | DebugArea Visual Lineup - Unas_5 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1481 | DebugArea Visual Lineup - Unas_6 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1482 | DebugArea Visual Lineup - Viking Jaffa | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1483 | DebugArea Visual Lineup - Ra Jaffa 2 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1484 | DebugArea Visual Lineup - Ra's Officer | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1485 | DebugArea Visual Lineup - JaffaMale Template - DO NOT USE | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1486 | DebugArea Visual Lineup - Praxis Jaffa Guard | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1487 | DebugArea Visual Lineup - Petbe | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1488 | DebugArea Visual Lineup - Mala'c | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1489 | DebugArea Visual Lineup - Bra'hin | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1490 | DebugArea Visual Lineup - Ra's Jaffa Infiltrator | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1491 | DebugArea Visual Lineup - Angry Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1492 | DebugArea Visual Lineup - Petbe (hostile) | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1493 | DebugArea Visual Lineup - Debug Area - Training Dummy L1 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1494 | DebugArea Visual Lineup - DebugArea Arena Praxis Jaffa | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1495 | DebugArea Visual Lineup - AR_J_Ra.BS_RaJaff (no template) | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1496 | DebugArea Visual Lineup - Moh'Katan | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1497 | DebugArea Visual Lineup - Asian Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1498 | DebugArea Visual Lineup - Bull Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1499 | DebugArea Visual Lineup - Cat Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1500 | DebugArea Visual Lineup - Cobra Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1501 | DebugArea Visual Lineup - Croc Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1502 | DebugArea Visual Lineup - Demon Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1503 | DebugArea Visual Lineup - Dragon Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1504 | DebugArea Visual Lineup - Eagle Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1505 | DebugArea Visual Lineup - Falcon Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1506 | DebugArea Visual Lineup - Horse Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1507 | DebugArea Visual Lineup - Hyena Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1508 | DebugArea Visual Lineup - Jackal Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1509 | DebugArea Visual Lineup - Mayan Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1510 | DebugArea Visual Lineup - Morrigan Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1511 | DebugArea Visual Lineup - Naga Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1512 | DebugArea Visual Lineup - Praxis Jaffa 2 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1513 | DebugArea Visual Lineup - Praxis Jaffa 1 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1514 | DebugArea Visual Lineup - Standard Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1515 | DebugArea Visual Lineup - Svarog Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1516 | DebugArea Visual Lineup - Tiki Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1517 | DebugArea Visual Lineup - Unas 1 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1518 | DebugArea Visual Lineup - Unas 2 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1519 | DebugArea Visual Lineup - Unas 3 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1520 | DebugArea Visual Lineup - Unas 4 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1521 | DebugArea Visual Lineup - Unas 5 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1522 | DebugArea Visual Lineup - Unas 6 Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1523 | DebugArea Visual Lineup - Viking Jaffa Female | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1524 | DebugArea Visual Lineup - Clothed Jaffa Female 1 | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1525 | DebugArea Visual Lineup - JaffaFemale Template - DO NOT USE | Jaffa | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1526 | DebugArea Visual Lineup - Prisoner 329 | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1527 | DebugArea Visual Lineup - Ra | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1528 | DebugArea Visual Lineup - Ba'al | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1529 | DebugArea Visual Lineup - Ra 2 | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1530 | DebugArea Visual Lineup - Ra 3 | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1531 | DebugArea Visual Lineup - Ra 4 | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1532 | DebugArea Visual Lineup - Ra 5 | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1533 | DebugArea Visual Lineup - GoauldMale Template - DO NOT USE | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1534 | DebugArea Visual Lineup - Sandbox Ba'al | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1535 | DebugArea Visual Lineup - Haughty Goa'uld | Goa'uld | WP-Goauld.WP_Ribbon_Elec_1A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1536 | DebugArea Visual Lineup - Ashrak Assassin | Goa'uld | WP-Goauld.WP_Ribbon_Elec_1A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1537 | DebugArea Visual Lineup - Lo'Taur Servant | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1538 | DebugArea Visual Lineup - Debug Area - Ra | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1539 | DebugArea Visual Lineup - Debug Area - Ba'al | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1540 | DebugArea Visual Lineup - Anat | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1541 | DebugArea Visual Lineup - Athena | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1542 | DebugArea Visual Lineup - Morrigan | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1543 | DebugArea Visual Lineup - GoauldFemale Template - DO NOT USE | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1544 | DebugArea Visual Lineup - Debug Area - Athena | Goa'uld | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1545 | DebugArea Visual Lineup - Debug Area - Morrigan | Goa'uld | WP-Goauld.WP_Ribbon_Elec_1A | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1546 | DebugArea Visual Lineup - Thor | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1547 | DebugArea Visual Lineup - Asgard Template - DO NOT USE | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1548 | DebugArea Visual Lineup - NPC_Asgard.BS_Degenerated_Asgard (no template) | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1549 | DebugArea Visual Lineup - Nox Child Male | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1550 | DebugArea Visual Lineup - Nox Child Female | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1551 | DebugArea Visual Lineup - Blix | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1552 | DebugArea Visual Lineup - NPC Child 1 | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1553 | DebugArea Visual Lineup - NPC Child 2 | other | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1554 | DebugArea Visual Lineup - AMBRat | creature MOB_AMBRat | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1555 | DebugArea Visual Lineup - ScavDog | creature MOB_ScavDog | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1556 | DebugArea Visual Lineup - Lenny | creature MOB_Lenny | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1557 | DebugArea Visual Lineup - MOB_Lenny.BS_MOB_LennyBaby (no template) | creature MOB_Lenny | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1558 | DebugArea Visual Lineup - Horden | creature MOB_Horden | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1559 | DebugArea Visual Lineup - Carnosaur | creature MOB_Carnosaur | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1560 | DebugArea Visual Lineup - Rhinolion | creature MOB_Rhinolion | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1561 | DebugArea Visual Lineup - Twilla Tree | creature MOB_TwillaTree | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1562 | DebugArea Visual Lineup - MOB_AN_Android.BS_AN_Android (no template) | creature MOB_AN_Android | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1563 | DebugArea Visual Lineup - MOB_CA_DroneTank.BS_MOB_DroneTank (no template) | mechanical Drone/Walker | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1564 | DebugArea Visual Lineup - Prisoner retrieval unit | mechanical Drone/Walker | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1565 | DebugArea Visual Lineup - Malfunctioning Drone | mechanical Drone/Walker | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1566 | DebugArea Visual Lineup - Ancient Drone | mechanical Drone/Walker | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1567 | DebugArea Visual Lineup - Straegis Fighter | creature MOB_StraegisFighter | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1568 | DebugArea Visual Lineup - Straegis Beacon | creature MOB_StraegisBeacon | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1569 | DebugArea Visual Lineup - BattleWalker | mechanical Drone/Walker | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1570 | DebugArea Visual Lineup - Straegis Titan | creature MOB_StraegisTitan | none | 1 | mob | 0 | - | w1300 x1 | holds fire |
| 1590 | DebugArea Lineup Attendant - Humans | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1591 | DebugArea Lineup Attendant - Jaffa male | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1592 | DebugArea Lineup Attendant - Jaffa female | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1593 | DebugArea Lineup Attendant - Goa'uld, Asgard and children | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1594 | DebugArea Lineup Attendant - Creatures and machines | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1595 | DebugArea Lineup Attendant - Clear | Human | none | 1 | mob | 0 | interactive | w1300 x1 | holds fire |
| 1600 | Debug Area - Minigame Terminal CrystalGame | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1601 | Debug Area - Minigame Terminal Alignment | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1602 | Debug Area - Minigame Terminal GoauldCrystals | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1603 | Debug Area - Minigame Terminal ConverseBasicHumanoid | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1604 | Debug Area - Minigame Terminal Livewire | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1605 | Debug Area - Minigame Terminal Hack (shell) | other | none | 1 | being | 4 | interactive | w1300 x1 | holds fire |
| 1610 | Armory 0: Test Weapon to Composite Combat Knife | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1611 | Armory 1: Microblade Combat Knife to SI 3 9mm Pistol | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1612 | Armory 2: SI 4 9mm Pistol to Assault Shotgun | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1613 | Armory 3: Strike Shotgun to Papa Polecat Flamethrower | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1614 | Armory 4: Aramid Fiber Ballistic Helmet to Trinium Ballistic Gloves | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1615 | Armory 5: Tri-Naq Alloy Ballistic Gloves to Boron-Carbide Ballistic Boots | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 1616 | Armory 6: Trinium Ballistic Boots to Neutronium Ablative Boots | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w1300 x1 | keeps 592 |
| 2300 | Beta E1 - Lt. Col. Carter | Human | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2301 | Beta E1 - Dr. Felger | Human | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2302 | Beta E1 - Captain Satterfield | Human | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2304 | Beta E1 - Lt. Debeaux | Human | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2305 | Beta E1 - Mimir | other | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2306 | Beta E1 - Ras'nor | Jaffa | none | 1 | mob | 0 | - | w23 x1 | holds fire |
| 2307 | Beta E1 - Sick Nine Peaks Warrior | Jaffa | none | 1 | mob | 0 | - | w23 x3 | holds fire |
| 2308 | Beta E1 - SGC Personnel | Human | none | 1 | mob | 0 | - | w23 x9 | holds fire |
| 2309 | Beta E1 - Jaffa Non Combatant | Jaffa | none | 1 | mob | 0 | - | w23 x3 | holds fire |
| 2320 | Beta E1 - Xenobiology Sample Cage | other | none | 1 | being | 4 | interactive | w23 x5 | holds fire |
| 2321 | Beta E1 - Artifact Display Case | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2322 | Beta E1 - Beta Site Hub Computer | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2323 | Beta E1 - Admin Hub Workstation | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2324 | Beta E1 - Science Center Computer | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2325 | Beta E1 - Wrecked SGC Shipping Crates | other | none | 1 | being | 4 | interactive | w23 x2 | holds fire |
| 2326 | Beta E1 - Box of Jaffa Tools | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2328 | Beta E1 - Asgard Fire Suppressant Manager | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2329 | Beta E1 - Fire Control Hub | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2330 | Beta E1 - Environmental Reading Device | other | none | 1 | being | 4 | interactive | w23 x5 | holds fire |
| 2331 | Beta E1 - Analytical Assistance Machine | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2332 | Beta E1 - Missing Asgard Hologram Emitter | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2333 | Beta E1 - Disabled Malfunctioning Drone | mechanical Drone/Walker | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2334 | Beta E1 - Consciousness Simulator | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 2335 | Beta E1 - Note at the ruins | other | none | 1 | being | 4 | interactive | w23 x1 | holds fire |
| 5800 | SGC Failsafe Device | other | none | 1 | being | 4 | - | w58 x1 | holds fire |
| 5802 | SGC Ordinance Crate | other | none | 1 | being | 4 | - | w58 x6 | holds fire |
| 5803 | SGC Carter Terminal | other | none | 1 | being | 4 | - | w58 x1 | holds fire |
| 63800 | Union Omega - Daniel Jackson | Human | none | 1 | mob | 0 | - | w80 x1 | holds fire |
| 63801 | Union Omega - Bra'tac | Jaffa | none | 1 | mob | 0 | - | w80 x1 | holds fire |
| 63802 | Union Omega - Teal'c | Jaffa | none | 1 | mob | 0 | - | w80 x1 | holds fire |
| 63803 | Union Omega - Thor | other | none | 1 | mob | 0 | - | w80 x1 | holds fire |
| 63804 | Union Omega - Soldier Trainer | Human | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63805 | Union Omega - Scientist Trainer | Human | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63806 | Union Omega - Commando Trainer | Human | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63807 | Union Omega - Archaeologist Trainer | Human | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63808 | Union Omega - Jaffa Trainer | Jaffa | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63809 | Union Omega - Asgard Trainer | other | none | 1 | mob | 0 | interactive, trainer | w18 x1 | holds fire |
| 63810 | Union Omega - Storage Officer | Human | none | 1 | mob | 0 | interactive | w18 x1 | holds fire |
| 63811 | Union Omega - Team Registrar | Human | none | 1 | mob | 0 | interactive | w18 x1 | holds fire |
| 63812 | Union Omega - Human Weapons Tier 1 | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w18 x2 | keeps 592 |
| 63813 | Union Omega - Human Weapons Tier 2 | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w18 x1 | keeps 592 |
| 63814 | Union Omega - Human Weapons Tier 3 | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w18 x1 | keeps 592 |
| 63815 | Union Omega - Human Weapons Tier 4 | Human | WP-Human.WP_SMG_1A | 1 | mob | 0 | interactive, vendor | w18 x2 | keeps 592 |
| 63816 | Union Omega - Human Armor Tier 1 | Human | none | 1 | mob | 0 | interactive, vendor | w18 x2 | holds fire |
| 63817 | Union Omega - Human Armor Tier 2 | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63818 | Union Omega - Human Armor Tier 3 | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63819 | Union Omega - Human Armor Tier 4 | Human | none | 1 | mob | 0 | interactive, vendor | w18 x2 | holds fire |
| 63820 | Union Omega - Human Crafting Biomedical | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63821 | Union Omega - Human Crafting Electronics | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63822 | Union Omega - Human Crafting Power Systems | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63823 | Union Omega - Human Crafting Materials | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63824 | Union Omega - Common Crafting Biomedical | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63825 | Union Omega - Common Crafting Electronics | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63826 | Union Omega - Common Crafting Power Systems | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63827 | Union Omega - Common Crafting Materials | Human | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63828 | Union Omega - Consumables | Human | none | 1 | mob | 0 | interactive, vendor | w18 x8 | holds fire |
| 63829 | Union Omega - Crafting Guide | Human | none | 1 | mob | 0 | interactive | w18 x2 | holds fire |
| 63830 | Union Omega - Command Registrar | Human | none | 1 | mob | 0 | interactive | w18 x1 | holds fire |
| 63831 | Union Omega - Jaffa Weapons Tier 1 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x2 | holds fire |
| 63832 | Union Omega - Jaffa Weapons Tier 2 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63833 | Union Omega - Jaffa Weapons Tier 3 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63834 | Union Omega - Jaffa Weapons Tier 4 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63835 | Union Omega - Jaffa Armor Tier 1 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x2 | holds fire |
| 63836 | Union Omega - Jaffa Armor Tier 2 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63837 | Union Omega - Jaffa Armor Tier 3 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63838 | Union Omega - Jaffa Armor Tier 4 | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63839 | Union Omega - Jaffa Crafting Biomedical | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63840 | Union Omega - Jaffa Crafting Electronics | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63841 | Union Omega - Jaffa Crafting Power Systems | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63842 | Union Omega - Jaffa Crafting Materials | Jaffa | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63843 | Union Omega - Asgard Programs T1 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63844 | Union Omega - Asgard Programs T2 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63845 | Union Omega - Asgard Programs T3 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63846 | Union Omega - Asgard Programs T4 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63847 | Union Omega - Asgard Armor T1 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63848 | Union Omega - Asgard Armor T2 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63849 | Union Omega - Asgard Armor T3 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63850 | Union Omega - Asgard Armor T4 | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63851 | Union Omega - Asgard Crafting Biomedical | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63852 | Union Omega - Asgard Crafting Electronics | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63853 | Union Omega - Asgard Crafting Power Systems | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63854 | Union Omega - Asgard Crafting Materials | other | none | 1 | mob | 0 | interactive, vendor | w18 x1 | holds fire |
| 63855 | Union Omega - Medic | Human | none | 1 | mob | 0 | interactive | none | holds fire |
| 63860 | Union Omega - Airman | Human | none | 1 | mob | 0 | - | none | holds fire |
| 63861 | Union Omega - Free Jaffa Warrior | Jaffa | WP-Jaffa.WP_Staff_Plasma_4A | 1 | mob | 0 | - | none | holds fire |

Body family and verdict counts:

- Jaffa: 169
- Human: 146
- other: 119
- Goa'uld: 41
- mechanical Drone/Walker: 9
- creature MOB_Lenny: 3
- creature MOB_Carnosaur: 2
- creature MOB_Horden: 2
- creature MOB_AMBRat: 2
- creature MOB_Rhinolion: 2
- creature MOB_ScavDog: 2
- creature MOB_StraegisBeacon: 2
- creature MOB_StraegisFighter: 2
- creature MOB_StraegisTitan: 2
- creature MOB_TwillaTree: 2
- creature MOB_AN_Android: 1

## 2. Production callers of `SpaceManager::spawn_npc`

No production caller exists. Every call is gated as shown below, so the table lists the gated call sites by crate.

| Crate | Files | Call sites | Gate |
|---|---|---|---|
| cell | 39 | 47 | inline `#[cfg(test)] mod` or test file |
| cell-combat | 31 | 66 | inline `#[cfg(test)] mod`, test file, or `test_fixtures` (`cfg(any(test, feature = "test-support"))`, dev-dependency only) |
| cell-console | 13 | 20 | test file |
| cell-content | 15 | 27 | inline `#[cfg(test)] mod` or `chain_replay_tests` (`#[cfg(test)] mod`) |
| cell-duel | 2 | 2 | test file |
| cell-interactions | 9 | 23 | inline `#[cfg(test)] mod` or test file |
| cell-methods | 20 | 34 | inline `#[cfg(test)] mod` or test file |
| cell-pets | 4 | 7 | test file |
| cell-world | 33 | 52 | inline `#[cfg(test)] mod`, `cfg(test)` `spawner_tests` or `aoi_churn_smoke` |

Files without `test` in the name that call `.spawn_npc(` (for example `trainer.rs`, `vendor.rs`, `dhd.rs`, `being.rs`, `aoi.rs`, `shutdown_flush.rs`) all call it below an inline `#[cfg(test)]` module. `despawn_npc` and `gm_spawn_npc` are different functions and are not counted. None spawns anything in production.

## 3. Tests that rely on the fallback

Heuristic candidates: a `#[test]` fn whose body mentions `592` or `NPC_DEFAULT_ABILITY`
and creates an NPC (a `spawn` call or an `npc` name in the body). A test that relies on the
fallback without naming 592 is not found by this scan. The fix column is a starting point;
confirm it per test when NC-21 lands. `add_ability(` of 592 or `NPC_DEFAULT_ABILITY` in the body means the fixture is already explicit.

| Crate | File | Test | Lines | Kind | Fix |
|---|---|---|---|---|---|
| cell-catalog | cell-catalog/src/cell/spawner/tests/npc_ability_animation.rs | live_db_every_npc_combat_ability_resolves_one_ability_end_sequence | 81 | mirrors the fallback in test code | none |
| cell-catalog | cell-catalog/src/cell/spawner/tests/npc_ability_animation.rs | live_db_a_hostile_template_fires_its_weapons_ranged_auto_attack | 56 | mirrors the fallback in test code | none |
| cell-combat | cell-combat/src/cell/abilities/death/tests.rs | dying_player_own_auto_cycle_clears_and_broadcasts | 52 | not an NPC-fallback test | none |
| cell-combat | cell-combat/src/cell/combat/auto_cycle.rs | arm_and_clear_no_op_for_non_player | 11 | not an NPC-fallback test | none |
| cell-combat | cell-combat/src/cell/combat/threat/aggro.rs | npc_default_ability_is_pistol_shot | 4 | not an NPC-fallback test | none |
| cell-combat | cell-combat/src/cell/service/npc_ai/pet/tests/ability.rs | a_pet_with_no_abilities_honours_a_toggled_off_default | 10 | asserts the fallback itself | none |
| cell-combat | cell-combat/src/cell/spawner_tests/harset/templates.rs | live_db_every_harset_mob_template_carries_a_non_default_ability_set | 60 | not an NPC-fallback test | none |
| cell-console | cell-console/src/cell/console/tests/ab_l2/effects.rs | ab_l2_effects_reads_the_callers_state_without_a_target | 40 | not an NPC-fallback test | none |
| cell-console | cell-console/src/cell/console/tests/ab_l2/mod.rs | ab_l2_non_gm_is_refused_with_feedback_and_nothing_changes | 39 | not an NPC-fallback test | none |
| cell-console | cell-console/src/cell/console/tests/bookmark.rs | playtest_bug_reports_abilities_weapon_and_current_target | 21 | unaffected (spawn_npc or direct fixture) | none |
| cell-console | cell-console/src/cell/console/tests/bookmark.rs | playtest_bug_writes_the_ability_snapshot_on_the_bookmark_id | 48 | not an NPC-fallback test | none |
| cell-world | cell-world/src/cell/space_manager/tests/npc_spawn.rs | spawn_npc_gets_default_ability | 9 | asserts the fallback itself | none |
| cell-world | cell-world/src/cell/space_manager/tests/npc_spawn.rs | spawn_npc_from_record_sets_template_fields | 67 | needs fix (record, empty ability_ids) | `592` in `ability_ids` |
| cell-world | cell-world/src/cell/spawner_tests/spawn_records.rs | spawn_record_with_ability_ids_skips_default_fallback | 22 | not an NPC-fallback test | none |
| cell-world | cell-world/src/cell/spawner_tests/spawn_records.rs | spawn_record_with_empty_ability_ids_falls_back_to_default | 20 | asserts the fallback itself | none |
| cell-world | cell-world/src/cell/spawner_tests/spawn_records.rs | spawn_record_with_empty_ability_ids_knows_exactly_592 | 11 | asserts the fallback itself (added by NC-10) | none |
| cell-world | cell-world/src/cell/combat/combat_profile/report.rs | placeholder_profile_logs_once_per_template_through_spawn | 22 | needs fix: expects the Legacy placeholder row (added by NC-10) (fixed by NC-30) | none until D-NC1 flips the policy; then assert the unresolved WARN instead |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_with_short_max_range_holds_fire_when_target_outside | 47 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_with_short_max_range_fires_when_target_inside | 28 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_max_range_zero_falls_back_to_npc_attack_range | 29 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_target_inside_min_range_schedules_backup_waypoint | 47 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_missing_ability_def_falls_back_to_npc_attack_range | 33 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/ability_range.rs | npc_ai_fight_stationary_does_not_back_off_inside_min_range | 31 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/lab_dummy.rs | ab_l2_a_lab_dummy_never_attacks_its_threat_target | 29 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/lab_dummy.rs | ab_l2_a_caster_lab_dummy_gets_no_ai_turn_either | 10 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/npc_vs_npc.rs | an_npc_only_kill_through_the_tick_pays_nothing | 81 | unaffected (spawn_npc or direct fixture) | none |
| cell | cell/src/cell/service/tests/npc_ai/npc_vs_npc.rs | an_npc_killed_by_another_npc_gets_no_turn_that_tick | 69 | unaffected (spawn_npc or direct fixture) | none |
| cell | cell/src/cell/service/tests/npc_ai/occluder_los.rs | the_drone_sees_over_the_med_station_desk_and_fires_on_the_occluder | 37 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/selector.rs | selector_skips_cooling_ability | 16 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/selector.rs | selector_skips_multiple_cooling_abilities | 22 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/selector.rs | selector_returns_none_when_all_cooling | 13 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/selector.rs | selector_falls_back_to_default_when_no_abilities | 14 | asserts the fallback itself | none |
| cell | cell/src/cell/service/tests/npc_ai/selector.rs | selector_picks_ammo_bearing_ability_for_npc | 36 | unaffected (spawn_npc or direct fixture) | none |
| cell | cell/src/cell/service/tests/npc_ai/state_machine.rs | npc_ai_leashing_snaps_to_spawn_restores_health_and_idles | 46 | unaffected (spawn_npc or direct fixture) | none |
| cell | cell/src/cell/service/tests/npc_ai/state_machine.rs | npc_ai_leashing_with_follow_target_skips_spawn_snap | 54 | unaffected (spawn_npc or direct fixture) | none |
| cell | cell/src/cell/service/tests/npc_ai/stationary_los.rs | the_stationary_drone_fires_across_the_med_station_desk | 41 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/stationary_los.rs | a_mobile_npc_still_walks_around_the_desk_instead_of_firing | 27 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/step_back.rs | a_melee_npc_never_steps_back | 20 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/step_back.rs | a_dead_zone_during_the_cooldown_holds_fire | 17 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai/training_dummy.rs | a_seeded_training_dummy_never_attacks_its_threat_target | 32 | needs fix (record, empty ability_ids) | `592` in `ability_ids` |
| cell | cell/src/cell/service/tests/npc_ai/zero_health_guard.rs | npc_killed_by_an_effect_bleed_does_not_shoot_back | 108 | already explicit | none |
| cell | cell/src/cell/service/tests/npc_ai_cover_behaviour.rs | melee_only_npc_never_takes_cover | 17 | already explicit | none |

Excluded: 4 NC-01 resolver unit tests in `combat_profile/tests.rs`. They call
`resolve_npc_combat_profile` directly and spawn no NPC.

### Groups for NC-21a, b and c

Counts cover `needs fix` rows only. `Unaffected` counts the rows NC-21 leaves alone.

| Group | Tests | Files | Lines (sum of test fn bodies) | Unaffected |
|---|---|---|---|---|
| cell-world | 2 | 2 | 89 | 0 |
| cell-combat | 0 | 0 | 0 | 0 |
| other | 1 | 1 | 32 | 6 |

`ability_select*` files under `crates/cell-combat/src/cell/service/npc_ai/`: flagged for NC-20.
Hits: `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs` (no fallback test in it;
it is a selector source file that NC-20 owns, so it gets no NC-21 fix).

## Findings

- The `RequireWeaponEvidence` verdict is a seed-level read. A template's components can
  differ at spawn time, so the verdict is a prediction for NC-30, not a runtime check.
- `spawn_npc` has no production caller. Production NPCs reach the fallback through their
  template or spawn records, not through this call, so NC-21 scopes the test fixtures only.
- `scripted` cannot be read from the seed: it has no column, so the table omits it.

