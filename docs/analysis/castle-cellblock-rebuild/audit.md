# Castle Cellblock Rebuild Spec: Audit Against Cimmeria

> Type: reference. Audience: Claude Code coordinator and packet workers.
> Updated: 2026-09-17. Companions: [launch prompt and decisions](README.md), [packet ledger](work-packets.md).
> Baseline: `main` at `01ab54b0` (branch `fix/gm-teleport-navmesh-loop`), inspected 2026-09-17. Documentation only; no build, test or client run accompanies this audit.

## What The Spec Is And What It Gets Wrong

The input is `SGW_Castle_CellBlock_Rebuild_Spec.xlsx` (21 sheets). It reconstructs the Praxis Human / Loyalist Jaffa Cellblock path from character spawn to the World 8 Castle handoff, on the premise that **"original server scripts are assumed missing and must be recreated"**. That premise is false for this repository, and it changes the shape of the work:

| Spec premise | Repository reality | Confidence |
|---|---|---|
| Server scripts are missing; 25 script components must be inferred | The original Atrea-generated Python exists: `deprecated/python/cell/spaces/Castle_CellBlock.py` (555 lines) plus 13 per-mission scripts in `deprecated/python/cell/missions/Castle_CellBlock/`. The Rust content engine already ports all of them as chains 1001-1111 in [castle_cellblock_chains.sql](../../../db/resources/Content/Seed/castle_cellblock_chains.sql). | HIGH (files read) |
| Mission 688 has no script and must be rebuilt as new | Cimmeria already authored 688 (chains 1105-1111): terminal `Cellblock_TerminalX`, Cimmeria step 80688, re-tagged `Cellblock_ArmoryRingSwitch` (spawn 79) and `Cellblock_ArmoryGuard1` (spawn 27), cross-world teleport to Castle at (466.365, 70.397, 991.466). | HIGH |
| Exit transition to Castle is unresolvable | Resolved by design in chain 1109 (`cross_world_teleport`, no ring ceremony). Castle-side arrival behavior is still absent (see row 25). | HIGH |
| Aftermath barracks spawn binding unproven | Cimmeria re-tagged spawns 25/26/36 as `Barracks_Guard1/2/3` and gates step 2355 on a 3-kill counter (chains 1100-1103). | HIGH |
| Jaffa intro dialog is unresolved | `Castle_CellBlock.py` `n82_trigger_Accept` shows dialog 2982 to every archetype. Literal reproduction is the original behavior. | HIGH |
| Aftermath class rewards are unresolved (2517/3942/3943/4408/4409) | `Aftermath.py` uses two branches only: archetype `< 5` (Soldier/Commando/Scientist/Archeologist) gets dialog 3942 and items 3347/3359/3372/3387/3401/3325; archetype `== 8` (Jaffa) gets dialog 3943 and items 3482/2797. Dialogs 2517/4408/4409 were never wired. | HIGH |
| StasisBlockDoubleDoors (sequence 10000) is the lockdown alternate route | Sequence 10000 is already the start-room exit door played on mission 622 completion (chain 1004; `ArmYourself.py`). It cannot double as the Escape lockdown route. | HIGH |
| Symbiote Loss (ability 1926 / effect 2480) may apply to Jaffa on load | No Python script references 1926 or 2480. Ability 1926 has the placeholder name `NO ABILITY DISPLAY NAME!`. Never wired in the shipped build. | HIGH |
| Dialogs 2305, 4000, 2308, 2309, 5019, 4003, 2516, 2518, 5861 and sequences 1751 (StraegisAttack), 10014 (hide cover indicator) are part of the flow | None of these IDs appears in any Cellblock or Castle Python script. They are spec-only content: authored in the client data but never triggered by the 2009 server. Implementing them is new authoring, not restoration. | HIGH (grep of all scripts) |
| World 12 movement config | Matches `worlds.sql` row 12 exactly (gravity 7.5, run 8.125, walk 2.069, crouch run 5.0625, jump 6, 1440 min/day). Nothing to do. | HIGH |
| Start profiles: 10 char defs at (-334.23, 73.47, -228.03) | `char_creation.sql` agrees, and additionally places Praxis Goa'uld (char defs 10 and 19, `ARCHETYPE_Goauld`) in Castle_CellBlock. Spec excludes Goa'uld; note that Cimmeria's `archetype neq 8` gates route Goa'uld down the Human branch, whereas the Python `< 5` check gave them nothing. | HIGH |

Archetype ids (`EArchetype`): 0 Any, 1 Soldier, 2 Commando, 3 Scientist, 4 Archeologist, 5 Asgard, 6 Goauld, 7 Sholva, 8 Jaffa.

## Evidence Precedence Used Here

1. Original Python scripts (`deprecated/python/`), the ground truth for what the 2009 server did.
2. Cimmeria's curated chain seed and chain-replay tests, the ground truth for what runs today.
3. The spec, for content the Python never wired (client dialogs, Kismet sequences, DB objectives) and for acceptance tests.
4. Spec rows marked INFERRED/UNRESOLVED are treated as design proposals, never as facts.

## Live Defects Found During The Audit

These are not spec gaps. They are bugs in the current seed that a rebuild pass must fix first, because every later packet's tests would otherwise assert against a double-firing baseline.

| ID | Defect | Evidence | Confidence |
|---|---|---|---|
| B1 | [space_castle_cellblock_chains.sql](../../../db/resources/Content/Seed/space_castle_cellblock_chains.sql) (chains 5000-5029, "Auto-exported by Cimmeria Content Editor") duplicates the curated seed with corrupted rows. Chain 5005 fires for a Jaffa entering Region2 and binds **both** dialog sets 5866 (Jaffa) and 2794 (Human) to Prisoner 329 and issues `accept_mission 638` twice. Chain 5015 fires for a Jaffa interacting with Marsh while step 2121 is `not_active` (before accept **and** after advancing past it) and displays 5022 three times plus Human dialog 4001. Chains 5022/5025/5027/5029 issue triple `accept_mission` for 682/684/686/687. | Seed read; `add_dialog`, `display_dialog`, `accept_mission` all have executor arms; the file is loaded by [database.sql](../../../db/database.sql) line 343. The accept guard in `cell/missions/lifecycle.rs` refuses the duplicates with a warn, but the dialog-set and dialog duplication is unguarded. | HIGH that the rows resolve; MEDIUM on exact client-visible symptom (not run in-client) |
| B2 | Chains 5000/5001 (zone load) carry contradictory conditions (`622 not_active` AND `622 completed`) and never fire. They are the only rows carrying `launch_ability 1372` (Stasis Sickness Stage 1). Net effect: Stasis Sickness is never applied. `launch_ability` also has no executor arm, so even a fixed condition would no-op. | Seed read; executor arm list in `content/executor/mod.rs`; [content-engine.md](../../content/content-engine.md) section 3. | HIGH |
| B3 | Chain 5002 (Region8 entry, `generate_threat 5000` on `ArmYourself_NIDGuard`) uses key `Castle_Cellblock.Region8`, but the point-set name is `Castle_CellBlock.Region8` (capital B, the only region with that spelling). Region matching is case-sensitive, so the scripted guard aggro never fires. The Python also called `setAggression(1)` and `threatGenerated(player, 1000)`; the chain omits aggression. | `point_sets.sql` names; `fire_enter_region` matches on the string key; linter note in content-engine.md section 12. | HIGH |
| B4 | `content_actions.delay_ms` is stored but never read by the loader or executor. Any packet that needs "play matinee, then show dialog after N seconds" (Straegis scene) has no delay primitive. | `grep delay_ms crates/content-engine crates/services/src/cell/content` returns only the DB row struct. | HIGH |
| B5 | Chain-replay coverage: no test file exists for missions 640, 680 or 681-686 even though seed comments cite `mission_681_*` / `mission_686_*` groups. | `chain_replay_tests/` listing: 622, 638, 639, 641, 687, 688, 1562, cover_demo, mod. | HIGH |

## New Evidence From `SGW_Castle_CellBlock_Dev_Master_v3.xlsx` (2026-09-17)

A collaborator ("lomiada") supplied a second, more deeply-mined spec: `SGW_Castle_CellBlock_Dev_Master_v3.xlsx`, 24 sheets vs. the original's 21. It restructures the material around evidence classes the original spec didn't separate out — `03_Minigames_Gates`, `06_Marsh_Companion_Death`, `17_Legacy_Conflicts`, `18_Server_Gaps`, `21_Raw_SourceCache_Hits`, `24_Scope_Exclusions` — and each row carries its own confidence tag (`CONFIRMED / SOURCE-BACKED`, `SOURCE-AID / CORROBORATED`, `PARTIAL / UNRESOLVED`, `USER-CONFIRMED`). It is a superset and refinement of the original, not a contradiction of it: every row checked against this repository's existing audit either corroborates a decision already made (Frost is a corpse not a rescue, Prisoner 329 is freed by hacking not an access card, the Castle handoff dialogs are 2572/2573/5861) or adds evidence this repository didn't have. One place it disagrees with this repository's own deeper research is noted below and resolved in this repository's favor, with reasoning.

### New finding: the Prison Boot movement-lock gate (v3 sheet `03_Minigames_Gates`, row "Order 0")

v3 claims, at `CONFIRMED / SOURCE-BACKED` confidence, that every Castle_CellBlock player starts wearing "Prison Boots" that lock movement until a minigame is completed — before the player can do anything else, including the already-implemented Frost/guard loot (chains 1003-1005). This is genuinely new: the original spec's Server_Scripts sheet (row 1, audited above) never mentioned it, and this repository's own audit missed it. Independently re-verified directly against this repository's DB seed (not just trusting the spreadsheet):

- **Item 3438 "Prison Boots"** (`db/resources/Items/Seed/items.sql:12382`) is a real boots-slot item (`container_sets '{1,12,17}'`).
- **Forced at character creation** for every Castle_CellBlock starting archetype except Goa'uld: `char_creation_choices.sql` binds item 3438 to 10 `vis_group_id`s, each independently confirmed via `char_creation_visgroups.sql` to be a `'Boots'`/`VIS_Forced` group on `char_def_id`s 1, 3, 5, 7, 11, 13, 15, 17, 20, 22 — exactly the 10 non-Goa'uld Castle_CellBlock char defs in `char_creation.sql`. Goa'uld (char_def 10, 19) are excluded from this forced group.
- **Ability 1597 "Prison Boot"** (`db/resources/Abilities/Seed/abilities.sql:2188`): description "Prevents the player from moving until they complete a minigame." → effect 1939 (`effects.sql:3090`): "Your security boot has activated. You cannot move until it is disabled."
- **Ability 1598 "Disable Your Prison Boot"** (`abilities.sql:2190`) → two effects: 3081 (`effects.sql:4329`) "Swaps the prison boot with a version that can be unequipped and that doesn't launch the minigame," and 1942 (`effects.sql:3092`) "This ability is launched from a minigame on success — removes the MISSION_Cellblock_PrisonBoot effect." The `MISSION_Cellblock_PrisonBoot` name in that description ties this explicitly to this zone, not a shared/generic mechanic.

**The gap, found independently of v3's own caveat:** neither the compiled Python (`deprecated/python/cell/missions/Castle_CellBlock/*.py`, `deprecated/python/cell/spaces/Castle_CellBlock.py`) nor the raw Atrea node-graph sources (`deprecated/data-scripts/scripts/missions/Castle_CellBlock/*.script`) reference "boot", "restrain", "shackle", "cuff", or "immobil" anywhere (grepped, case-insensitive, zero hits) — the same "data exists, no recovered script triggers it" shape as defect B2 (Stasis Sickness 1372), except here there is no Python call to point to at all, not even an unfired one. Whatever fired ability 1597 on spawn and reported minigame success to ability 1598 lived outside the Atrea mission/space script layer this repository has recovered — most plausibly an engine-level spawn hook, not zone content. v3's own `18_Server_Gaps` sheet independently flags the same unknown: "Exact minigame type/board is not named in recovered data."

**Verdict:** the mechanic itself is CONFIRMED — the item/ability/effect/char-creation chain is too specific and too mutually consistent (forced boot item → named "Prison Boot" ability → movement-lock effect text → named "disable" ability → named removal effect referencing this zone by name) to be coincidental or spec inference. What minigame implements the removal, and what triggers ability 1597 on spawn, are not recovered and need a decision, not a guess. See new packet **C00** in [work-packets.md](work-packets.md#c00) and decision **D-CB14** in [README.md](README.md#decision-answers).

### Other v3 findings, briefly

- **Marsh's full arc corroborated, not contradicted:** v3's `06_Marsh_Companion_Death` sheet independently confirms Marsh is meant to be an active combat companion through Mess Hall/Hallway (dialog 5019: "Let's move out... flank... Straegis can...", matching mission 681's flank objective) and dies/is removed at the Straegis scene (dialog 2516, already this repository's D-CB07/C08b plan) with a previously-uncaptured follow-up line, dialog 5859: "Find a way out of the Cellblock. Without Marsh." This strengthens, rather than conflicts with, the user's D-CB13 answer (full escort) and GC1's existing child-packet split — GC1b-2 already scopes clearing the follow at the Straegis scene C08b handles. Worth folding dialog 5859 into GC1a's dialog list as the post-death "you're on your own" beat.
- **New nuance for C02/C07:** v3's `17_Legacy_Conflicts` sheet flags that Ring transport event sets exist in at least three generations in the client data (365-368, 872-875, 10000) and warns not to "blindly fire all generations." Worth a one-line check in C02's ring-route regression tests that only the one physically-wired generation resolves.
- **New nuance for GC1a:** v3 flags a speaker-id ambiguity — Marsh's "future" introduction dialogs (4001/3999/5022/5023) use speaker 256, but his companion-phase dialogs (2309/4003) use speaker 261, which other evidence in this repository ties to "Col. Marsh." Treat as a likely phase/entity swap rather than a bug; GC1a's writer should preserve whichever speaker_id each dialog's own seed row already carries rather than normalizing them to match.
- **v3 disagrees with this repository's own deeper research on one point, and this repository's finding wins:** v3's `18_Server_Gaps` sheet still lists Aftermath crate item ids (dialog 2517) as an open reconstruction target, at the same confidence level as its other rows. This repository's GC2 archaeology pass (2026-09-17, see [README.md](README.md#decision-answers)) went one level deeper than v3's sources here — it found and read `Aftermath.script`, the raw Atrea node-graph source, and confirmed via the original designer's own node comments ("Human"/"JAffa") that the two-branch reward was deliberate, complete, shipped design, not truncated content. GC2 stays closed; no action from this v3 row.
- **New, minor:** v3's `23_Exit_To_Castle` sheet flags a "character-creation color bug" as a reason state persistence across the World-12-to-World-8 transfer is a P0 test — not evidence of a missing packet, just a UAT note to carry into C09's milestone testing (M4).

## Row-By-Row Audit Of The Spec's Server_Scripts Sheet

Status vocabulary: **DONE** (Cimmeria already does this), **BUG** (see table above), **RESTORE** (Python did it, Cimmeria does not), **NEW** (spec-only content never wired by the original; needs a design decision), **OUT** (excluded by decision). Packet IDs refer to [work-packets.md](work-packets.md).

| Spec row | Spec asks for | Python evidence | Cimmeria today | Status | Packet |
|---|---|---|---|---|---|
| 1 World init | Detect archetype; apply Stasis Sickness 1372/1634; consider Symbiote Loss; accept 622; intro 2982 | `player.loaded`: accept 622 if not active, dialog 2982 (all archetypes), add dialog set 5229 to template 14, **launch ability 1372 on every load**; play sequence 10000 if 622 already complete | Chains 1001/1002/1006/1007 cover accept, dialog, bindings, relog. Ability 1372 never applied (B2). Symbiote Loss never existed. | RESTORE (1372); OUT (1926) | C03 |
| 2 Arm Yourself | Guard corpse 2141, Frost 3995, grant 55 + 3730, complete 622 on firearm; start 1360 | `ArmYourself.py`: dialog 3995 grants both items to bags 3 and 0, plays 10000, completes 622. Never touches 1360. Dialog 2141 has no server hook (client-side inspect). | Chains 1003-1005 split loot Frost (letter) then Guard (pistol, Cimmeria dialog 3996), manual-equip step 80622/80623, sequence 10000, complete. Mission 1360 never accepted. | DONE; NEW (1360) | C04 |
| 3 Prisoner intro | Human 2300 / Jaffa 5021; advance 2114 | Space script binds 2794 or 5866 on Region2 by archetype | Chains 1011-1015. Duplicated and cross-bound by B1. | DONE + BUG | C01 |
| 4 Door hack | Livewire on `329_CellDoorButton`; EventSet 745 event 6000 (sequence 1749); advance 2115 | `Prisoner_329.py`: Livewire, sequence 1749 | Chains 1016/1017 | DONE | - |
| 5 Prisoner follow-up | 2299 / 5020; complete 638, accept 639 | Same, plus blurb 2298 | Chains 1018-1021 | DONE | - |
| 6 Ambernol prompt/pickup | 2298 prompt; pickup 2297; grant 19; steps 2117 then 2145 | `FindAmbernol.py`: Region11 advances 2145; vial interact grants 19, destroys vial, aggros drone, dialog 2297, sequence 10001, advance 2144 | Chains 1031/1032 | DONE | - |
| 7 Cover tutorial | Drone activation; indicator show (10001) and hide (10014); objective 2484 cover; objective 2482 kill; step 2144 needs both | Python only tracks the drone death (advance 2343). Never plays 10014; never tests cover. | Chain 1033 = Python. Chain 1035 is a demo counter on any cover set. Engine has `player_entered_cover`, `player_in_cover_duration`, `complete_objective`, `objective_status`. Cover set catalog has `_CA-CellBlock_Int00-15-15` (id 425); whether a med-bay desk set exists is unverified. | NEW | C05 |
| 8 Cure | Item 19 use fires ability 1374 / effect 1636; consume; complete 639; Stage 2 timer optional | `FindAmbernol.py`: `item.use::19` removes item, completes 639, accepts 640. | Chain 1034. `items_event_sets` binds item 19 to ability 1374 (event 5), so the client may request the ability itself; unverified whether the server path runs 1374/1636. Stage 2 needs timers (engine Tier 1.2, absent). | DONE (consume); RESTORE-verify (1374); OUT (Stage 2) | C03 |
| 9 Hack the Rings | Blurb 2305; Livewire on `HackTheRings_Switch`; ring region 1 to 2; complete 640 | `HackTheRings.py` + space `teleport::in` region 2 completes 640. No 2305. | Chains 1041-1046 and 1044. Ring FSM in `cell/ring_transport/`. | DONE; NEW (2305) | C07 |
| 10 Marsh intro | Blurb 4000; 4001 / 5022; accept 641 | Space script: interact `Preparation_ColMarsh` gated on step 2121 not active shows 4001 or 5022; dialog choice accepts 641. No 4000. | Chains 1051-1054, 1063 (with the loop fix). B1 chain 5015 corrupts the Jaffa branch. | DONE + BUG; NEW (4000) | C01, C07 |
| 11 Locker / SMG | `Preparation_SMG1A` grants 21; 3999 / 5023; no duplicate grant | `Preparation.py` | Chains 1055-1059, 1066, 1062, 1064 with manual-equip step 80641 | DONE | - |
| 12 Stasis terminal | Livewire on `Preparation_Terminal`; 3998; pods open; complete 641, accept 680 | `Preparation.py`: Livewire victory, dialog 3998, complete 641, accept 680. No pod actors driven server-side. | Chains 1060/1061/1065. Pod visuals are client Kismet with no recovered event id. | DONE; pods OUT (no evidence) | - |
| 13 Escape move out | Marsh escort; 2308/2309 or 5019; ring 2 to 3 | `EscapeTheCellblock.py`: ring switch triggers transporter 2; teleport-in region 3 advances 2345; Region9 accepts 681. No escort, no dialogs. | Chains 1071-1074. Marsh stays at his Preparation position for the rest of the zone. `set_follow_target` executor arm exists (0 seed rows). | DONE (mechanics); NEW (escort, dialogs) | GC1 |
| 14 Lockdown | Energy field; 4003; alternate route; StasisBlockDoubleDoors | Nothing. Step text "Lockdown!" is just step 2345's log text. | Nothing. Sequence 10000 is already the 622 exit door. Energy-field actor and event are unrecovered. | NEW, blocked on evidence | GC1 |
| 15 Mess Hall | Hidden 681; guards; flank objective 2725 | `MessHall.py`: 2-kill counter completes 681; leaving Region9 completes 680 | Chains 1073 (680 completes on Region9 **enter**, a deliberate deviation), 1085-1087. No flank objective. | DONE; NEW (2725) | C06 |
| 16-19 Hallway01-04 | Hidden controllers keyed on region entry and guard death | Space script accepts on Region3 exit / Region4 / Region5; mission scripts complete on death | Chains 1081-1083, 1088-1091 | DONE | - |
| 20 Hallway05 | Two guards plus flank objective 2731 | `Hallway05Controller.py` counter | Chains 1092-1094. No flank objective. | DONE; NEW (2731) | C06 |
| 21 Straegis scene | EventSet 747 event 6000 (sequence 1751, 10.0 s camera); remove Marsh; blood; 2516 | Nothing | Nothing. `play_sequence` and `destroy_entity` exist; no delay (B4); no blood asset; creature template unresolved. | NEW | C08 |
| 22 Aftermath crate | Class loot dialog and rewards; Scientist/Archeologist mapping unresolved | `Aftermath.py`: two branches only (see table above) | Chains 1097-1099, 1104 reproduce the Python exactly. | DONE; per-class mapping NEW | GC2 |
| 23 Barracks | Track NID guards; complete 687 | No Python for step 2355 | Chains 1100-1103 (Cimmeria design) | DONE | - |
| 24 Secure the Armory | Blurb 2518; terminal; objective 2734; guards 4647; complete 688 | No Python | Chains 1105-1111. Objective 4647 is optional with one tagged guard. No 2518. | DONE; NEW (2518) | C07 |
| 25 Exit handoff | Transition to Castle; preserve 1360; Gerschon 2573 / 5861 | Cellblock: nothing. `Castle.py` (343 lines) drives 2573 through dialog set 3062 on `Castle_SgtGerschon` and mission 701. No 5861 anywhere. | Chain 1109 teleports to Castle. There is **no Castle chain seed at all**; spawn 112 `Castle_SgtGerschon` exists but is inert. | DONE (transition); RESTORE (2573 via Castle.py port); NEW (5861) | C09 |

## Spec Items With No Server Work

- Dialog 2141 (guard corpse inspect) and 2297 text, Kismet ring prefab sequences, map-level atmosphere, animation packages, UI Lua behavior: client-side; nothing for the server beyond what already fires.
- Ring Transport regions 1-3 and event sets 10000/874/875: seeded in `ring_transport_regions.sql` and driven by the ring FSM. Respawner 5 "Level 7: Ring Transporters" is seeded.
- Level_Files and Source_Index sheets: provenance only.

## Acceptance Test Mapping (spec Validation_Tests sheet)

| Spec test | Automated guard today | Gap |
|---|---|---|
| T01/T02 spawn | `chain_replay_tests/mission_622.rs` (accept on load); char_creation seed | In-client UAT only for coordinates |
| T03/T04 Frost and Guard | mission_622.rs (loot split, relog restore) | 1360 not asserted (C04) |
| T05/T06 Prisoner archetype | mission_638.rs (`assert_region_enter_resolves_dialog_set`) | Must add a "exactly one dialog set" assertion that fails while B1 exists (C01) |
| T07 cell door | mission_638.rs (Livewire, 1749) | - |
| T08 Ambernol combat | mission_639.rs | Cover objective 2484 and indicator hide (C05) |
| T09 cure | mission_639.rs (`RemoveItem 19` exactly once) | Ability 1374 path unverified (C03) |
| T10 ring route 1 | none | C02 |
| T11/T12 Preparation | mission_641.rs | Jaffa duplicate-dialog guard against B1 (C01) |
| T13 stasis pods | mission_641.rs (complete 641, accept 680) | Pod visuals out of scope |
| T14 Mess Hall | none | C02, C06 |
| T15 hallway chain | none | C02 |
| T16/T17 Straegis | none | C08 |
| T18/T19 Aftermath loot | mission_687.rs | Per-class mapping is a decision (GC2) |
| T20 Armory | mission_688.rs | - |
| T21/T22 exit handoff | mission_688.rs (cross-world action shape) | Castle-side (C09) |
| T23 relog safety | per-mission restore chains and tests | Add for any new chain (every packet) |
| T24 legacy isolation | n/a | 642 and Frost-alive content are not in the seed; nothing to guard |

## Audit Validation Record

| Check | Outcome |
|---|---|
| All 21 spreadsheet sheets exported and read (openpyxl, data_only) | Done; 176 KB of cell text |
| Python scripts grep for spec-only IDs (2305, 4000, 2308, 2309, 5019, 4003, 2516, 2518, 5861, 10014, 1751, 1360, 642, 1926) | Zero hits across `missions/Castle_CellBlock/*.py`, `spaces/Castle_CellBlock.py`, `spaces/Castle.py` |
| Chain seed read end to end (1706 + 542 lines) | Done |
| Executor arm inventory | 32 arms; no `LaunchAbility`, `ApplyEffect`, `MoveEntity`, `GrantXP`, no `delay_ms` |
| Build, test, live DB, client | Not run |
