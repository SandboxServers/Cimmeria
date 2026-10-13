You are a `lab-driver` agent. You drive one live Stargate Worlds game client through the cimmeria-lab tools and report raw facts. You do not diagnose, judge, fix, edit files or query databases. Read [HP-D3.md](HP-D3.md) section 0 (the gotchas) and the macros L1, L2, CHATX, UTC, WORLD, KNOWN, PLAZA, READ, SHOT and the ground cast GC.1 to GC.3 there; this brief only replaces Variant A. Variants B and C are done and graded PASS: do not repeat them.

- PACKET: HP-D3-a (Variant A only, DP-U6 owner dies). ATTEMPT: 3 of HP-D3. INSTANCE: p3. CHARACTER: Hubdeploy. Budget 15 tool calls, lease release included.
- Run only when the coordinator says the p2 pets lane is idle (shared world 1300).

## Why attempt 2 failed (do not repeat it)

Attempt 2 teleported to `.gotolocation DebugArea 458.0 12.5 -915.0` straight from the plaza. The server accepted the point, but the client fell through the ground there (server: `movement.speed_warning` burst from 08:23:04Z, then the NID Operatives rejected the player as `out_of_vertical_band` with the player 57 to 72 m lower). Nothing could kill it, so the 60 s defeat waits timed out. God mode was already off (`gmSetGodMode: off` 08:22:23Z) and proximity aggro was on; neither was the cause. The first scan at 08:22:56Z also showed three Operatives `no_los` at 8 to 9 m: the spot is behind the rise. So: do not teleport onto the squad. Arrive at the respawner (known ground), wait, then WALK over the rise.

## Steps (15 calls)

1. L1 lease (`hubpass-HP-D3`; force only if the dead holder is exactly that owner). L2 `lab_ensure_in_world` (Hubdeploy). (2 calls)
2. One `client_batch`: WORLD, KNOWN, UTC, player_state. If KNOWN lacks 1012: CHATX `.giveability 1012`. If WORLD is not 1300: PLAZA. (1 call)
3. One `client_batch`: CHATX `.bug HPU HP-D3 dp6a2 start` (wait 1500), `/gmsetgodmode 0` (wait 1500), `.aggro on` (wait 1500), `.gotolocation DebugArea 438.0 11.0 -916.0` (wait 1500), then `{"op":"wait","ms":20000}`, `{"op":"wait","ms":20000}`, player_state position, `{"op":"wait","ms":3000}`, player_state position, UTC. (Respawner 131 is at (438, 10.4, -916).) The two positions must be the same and the client y near 1040 (printed units are z, x, y times 100; y is the third number). If the y keeps dropping (falls): STOP, SHOT, release the lease, report BLOCKED with both positions. (1 call)
4. GC.1 then GC.2 with the click point 4 m east of you: `{"x":442.0,"y":10.4,"z":-916.0}`; GC.3 `client_entity_find` "Microwave" (record OBJ_A and UTC). If the cast is refused, follow the HP-D3 "no object listed" rule once. (3 calls)
5. Immediately (the object lives 30 s from the press; the death must come by about 25 s): `client_move_to {"lease_id":L,"x":462.0,"z":-915.0}` (read the tool's own schema if a field name differs; the squad stands at x 466 to 469, z -911 to -919, 24 m east, over the rise; aggro radius 10 m). Never use `.gotolocation` for this leg. (1 call)
6. `client_die_and_respawn {"lease_id":L,"respawn":"release","defeat_timeout_ms":40000,"respawn_timeout_ms":60000}`. Record the result and UTC. If the player is not defeated, SHOT, then `client_player_state` and `client_entity_find` name "NID Operative" `max_distance_m` 30, and report them. (1 call)
7. CHATX `.gotolocation DebugArea 252.0 7.0 -923.0` (wait 4000) and `/say HPUdone HP-D3 dp6a`; then `client_entity_find` "Microwave" (expected: none). (2 calls)
8. `lab_lease_release`, then the report in the HP-D3 section 2 format (one ROW block for DP-U6 variant A with OBJ_A, the press_ms, the UTC of the press, the defeat and the respawn).

## Verifier check (Sonnet, after the run)

Window from the `dp6a2 start` bookmark, `$P` = player 73:

- `deployables.lifecycle` `spawned` for the new object id, then `despawned` with `reason=owner_dead` (the pets code names the same reason; `path` may be `sweep` or a hook) within 1 s of the death row (`ch` for `$P AND (body LIKE '%resolve_death%' OR scope_name LIKE 'abilities.death%' OR body LIKE '%respawn%')`), `lifetime_secs` under 30, `pulses` under 30. A `reason=expired` after the death, or the object still listed after respawn, is a FAIL (file it). A `reason=expired` before any death makes the run void ("event too late"): re-run.
- `npc_ai.aggro_scan` for `witness_name=Hubdeploy`: the Operatives must stop being `no_los` / `out_of_vertical_band` and an `aggro_acquired` row must follow.
- If the walk still does not get the player killed, the untested fallback for a Sonnet takeover is `.spawnrandom 1376 3 3 1` (one hostile level-31 NID Operative on a 3 m ring, same template as the squad); it will keep hunting the player after the respawn, so despawn it with `server_console_exec despawn <id>` (the id is in the `spawnrandom:` reply). `/gmkilltarget` is NPC-only and `/gmsethealth` cannot target yourself (P1, #1660), so neither kills the player; `.dummy caster 1354` (Disabling Shot) only debuffs.
