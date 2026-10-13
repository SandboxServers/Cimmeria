You are a `lab-driver` agent. Read `briefs/HP-P3.md` sections 0, 1 and "Shared definitions" first: its gotchas, macros (L1, L2, CHATX, READ, SHOT, PET, BAR, PLAZA, DONE) and the report format apply here unchanged. This file replaces only the packet steps. ATTEMPT: 2 (rerun after attempt 1 failed for setup reasons). INSTANCE `p2`, CHARACTER `Hubpets`. **Budget: 25 tool calls.**

## Why attempt 1 failed (verifier, server rows; do not repeat these mistakes)

1. **The other lane was on the plaza.** Hubdeploy (`p3`) stood at the same spot (252, 7, -923). The first Rat picked Hubdeploy (07:57:28Z `aggro acquired`, target Hubdeploy, 5.0 m) and killed it; it never targeted Hubpets, so a Defensive pet had nothing to defend. This packet MUST run while the deployables lane (HP-D1, HP-D2, HP-D3) is idle. The coordinator checks `lab_lease_status` for `p3` before dispatch; you check it too (L0).
2. **Hubpets died.** Two Rats at `.aggro on` did 65 per hit each, roughly every 2 s; the player (760 health) died 24 s after the second Rat arrived. The pet was Passive then. A death despawns the pet (`owner_dead`) and respawns the player at respawner 130, about 39 m away at (251, 8, -962). That is why `client_world_click` found the Rat 39.46 m away and why U8 saw `no-pet`. There was no position desync: the Rat stayed at (257, 7, -923).
3. **Never have two Rats up while testing.** Spawn ONE Rat per row.

## Changes

- **Aggro per row.** U5 needs `.aggro on` (the Rat must attack you). U7 and U8 use `.aggro off` (no Rat shoots you, so you survive and the click and the order are the only variables).
- **Health check before every row.** `client_player_state {"lease_id":L,"fields":["health","level","xp"]}` (record what exists). If the avatar is dead or the world is not 1300: write UNEXPECTED, `.gotolocation DebugArea 252.0 7.0 -923.0` (PLAZA) and re-summon the pet (`.pet summon 2826`, check PET) before going on.
- **Selection recipe (U7, U8).** After the spawn, wait 3500 ms, `client_entity_find` the Rat (it must be within 6 m; if farther, a death or relog happened: PLAZA again), then `client_camera {"lease_id":L,"face_entity_id":<RAT_ID>}` once, then `client_world_click {"lease_id":L,"entity_id":<RAT_ID>,"button":"left","expect":"target","settle_ms":3000,"rotate_camera":true}`. Use JSON numbers, not quoted strings, for every numeric argument. If it fails twice, record the error and the entity's distance and write U7 BLOCKED (the `lab-driver` agent has no `client_target` tool; the coordinator can add one).
- **Order fallback (U7).** There is no console command that orders a pet. If no pet-bar ability button is pressable (`client_ui_state`), use `usePetAbility(Unit.Pet1, 221, Unit.Target)` through `client_lua_eval` twice, 2500 ms apart (N3, labelled; NATIVE_SHORTFALL at best). The bar itself is broken (issue 1674), so expect to need this.

## Steps

**S0 (2 calls).** L0: `lab_lease_status` (no arguments): every instance but yours must show no holder for `p3`. If `p3` is held, stop and report BLOCKED with the holder's owner. Then L1 and L2.
**S1 (1 call).** One `client_batch`: WORLD, PET, BAR, UTC, player health. PLAZA if needed.

**U5 (5 calls).** One batch of CHATX lines: `.bug HPU HP-P3 u5 start`, `.aggro on`, `.pet summon 2826` (wait 2500), `.pet stance 1` (wait 1500). Then: confirm Hubdeploy is not within 40 m (`client_entity_find` name `Hubdeploy`, `max_distance_m` 40: expect none; if listed, stop and report BLOCKED). Then CHATX `.spawnrandom 74 5 5 1` (wait 3500), PET, UTC. Then `client_entity_find` for the Rat, then a `client_batch` `[{"op":"wait","ms":8000}, PET, player health]`, then SHOT label `P3-fight` and `client_entity_find` (all entities within 25 m). The pet must stay Defensive the whole time (do not touch the stance until U5's last read). Add `/say HPUdone HP-P3 u5` and then `.aggro off` (wait 1500) to the next batch. If you are below 300 health after the fight, `.pet dismiss` and wait 20 s before U7 (the Rat should be dead by then; if it is not, stop and report).

**U7 (7 calls).** CHATX `.bug HPU HP-P3 u7 start`, `.pet stance 0` (wait 1500), `.spawnrandom 74 5 5 1` (wait 3500); PET; selection recipe; `client_ui_state` (record the pet-bar windows); order the pet (button click, else the N3 fallback), then READ unfiltered, then PET plus `client_entity_find` for the Rat, then SHOT label `P3-pet-ordered`.

**U8 (4 calls).** CHATX `/say HPUdone HP-P3 u7`, `.bug HPU HP-P3 u8 start`, `.pet stance 2` (wait 1500); `client_player_state` (XP_BEFORE); if the Rat from U7 is still alive and was targeted, wait; else `.spawnrandom 74 5 5 1`. `client_batch` `[{"op":"wait","ms":15000}, PET]`, `client_entity_find` for Rats, `client_player_state` (XP_AFTER). CHATX `/say HPUdone HP-P3 u8`. Release the lease and write the report. Rows U5, U7, U8 (U8: XP and credit only; KillCount stays NOT-RUN, decision D6).

## Verifier notes for this rerun

Expect for U5: `npc_ai.aggro acquired` with target Hubpets (entity 4), then `pets.ai engaged` with `why` `defend_owner` and `target_id` = the Rat id (a pet at stance 1 is only expected to react once the Rat damages or threatens the owner). For U7: `pets.command invoked` ability 221, then `rejected` for the second order. For U8: `pets.credit pet_kill_credited` and DB `exp` above 0. This run must happen only while the deployables lane is idle.
