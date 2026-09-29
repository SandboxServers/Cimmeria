# NPC attack presentation -- why a guard's shot draws nothing

> **Diátaxis type**: reference (evidence record)
> **Audience**: engineers debugging what a client draws for a server-sent `onSequence`
> **Last updated**: 2026-09-29
> **Confidence**: HIGH that NPC-sourced weapon sequences render nothing while identical player-sourced ones do (colo telemetry); MEDIUM that the missing `BSF_InCombat` stance is the cause (legacy parity plus client structure, not yet confirmed in a client); the player-side "animations break after respawn" report is **not** explained (see [Open questions](#open-questions))

## Summary

On the colo, a player saw no enemy shooting animation and heard no enemy gunfire. The server did send every shot: each NPC attack produced an `onSequence` (client method 1) with the seeded Ability_End sequence, and the client received and dispatched all of them. The same sequence id from the **player** spawned a muzzle-flash / tracer `EmitterSpawnable` and a weapon sound within about 40 ms of arriving. From an **NPC** it spawned nothing.

The Rust NPC AI never set `BSF_InCombat` on an NPC. The legacy server did, on entering combat, and broadcast it to the mob's witnesses (`SGWMob.aiBeginCombat`). On the client that bit re-keys the pawn's stance and weapon set, and the weapon sequences read the Source pawn's weapon component slot before they spawn effects. The fix sets the bit while an NPC is Fighting and announces it before the first shot.

## What the server sent (SigNoz, 18:47-19:30 UTC)

| Source | Ability | Event set / sequence | Shots | Witnesses |
|---|---|---|---|---|
| Player (entity 4) | 559, 579 | 15 / 15 (`KIS-SA_Burst_Source`), 3 / 3 (`KIS-SA_Sing_Source`) | 170 | 0 (self only) |
| NID Guard, Cellblock Guard, others | 559, 579, 592 | same | ~175 | 1 |
| Prisoner retrieval units (100010, 100029, 100031, 100150, 100300) | 221 (Energy Shock) | 802 / 1866 (`KIS-SA_Beam_Source`) | 36 | 1 (2 shots: 0) |

Ability 221's beam is unrelated to the guards' weapons: its NPC has a 146-byte appearance with no `WP-*` component. The guards (BeingAppearance 440 / 446 bytes) carry `WP-Human.WP_SMG_1A` / `WP_Pistol_1A`, the same weapon components player items use, so the weapon assets are not the difference.

## What the client did (`cimmeria-client` telemetry, same session)

Timestamps are the client's `ts_ms`. The server-to-client offset is a steady +0.40 to +0.47 s.

| Window | Source | Sequence | Client result |
|---|---|---|---|
| 790.9 s - 811.0 s | guard 100307, 11 consecutive shots, player silent | 3 (ability 579) | 11 `Event_NetIn_onSequence` deliveries; **0** `EmitterSpawnable`, **0** `sing`/`burst` sound |
| 813.6 s, 815.1 s, 816.7 s, 818.4 s, 820.0 s | player | 3 (ability 579) | each shot: one `EmitterSpawnable` ~40 ms after delivery; a `sing` sound at the sampled shots |
| 709438.99 s | guard 100149 | 15 (ability 559) | `burstF` sound at delivery, **no** tracer emitters in the next 1.2 s; the player's own burst just before made six, 81 ms apart |

The `EmitterSpawnable_N` counter advances only on player shots and on impacts, never on the guards' shots. Sequence 15's sound is on a Kismet path that does not need the weapon slot (`Bool` -> `SpeedSoundDelay` -> `PlaySound`), which is why the burst weapon still makes noise; sequence 3's sound and both muzzle-flash branches sit behind the slot data.

The `onTimerUpdate` drops on SGWMob (method 12) in the same log are the separate fault fixed by PR #1115; they are not the cause.

## The client's path for `onSequence` (Ghidra, `SGW.exe`)

- `Event_NetIn_onSequence` is subscribed by `SequenceManager` (`MemberCallback` vtable `0x019ba650`, registered in `FUN_00d05a50`, handler `FUN_00d05790`). The handler reads `SourceID`; **if no client entity exists for it the sequence is dropped silently** (`FUN_00dd0de0` returns 0). Otherwise it parses the request (`FUN_00d13780`: `KismetEventSetSeqID`, `SourceID`, `TargetID`, `PrimaryTarget`, `ImpactTime`, `InstanceId`, `ViewType`, `NameValuePairs`) and asks the cache library for the `CookedKismetEventSequenceData`.
- `FUN_00d06f30` (`Event_Cache_ElementReady`) plays the request. If the Source has no pawn (`entity+8 == 0`) a `Witness` (0) sequence is **dropped**; event 5001 and `ViewType` 1 or 2 are deferred to `Event_AppearanceJob_Completed` instead. `FUN_00d06dd0` culls a sequence whose nearer endpoint is beyond a view-distance cvar of the local viewer (`BW__unknown_00d00c10`), then instantiates the Kismet script on the Source pawn.
- The script (`KIS-abilities_human.upk`, dumped with `cimmeria-upk`): `KIS-SA_Sing_Source` and `KIS-SA_Burst_Source` start at `SeqEvent_ActionEnd` and run `SeqAct_ComponentSlotData` -> `SeqAct_ComponentSlotAttachment` ("Get the sfx, pfx, and mesh for the current weapon", "Equiped Weapon Mesh") before `SeqAct_SpawnEmitter` (muzzle flash, tracer) and `SeqAct_PlayRecoil`. The chain advances on the slot node's `Loaded` pin.

None of the drop conditions fits the guards: their entities and pawns exist (`SGWGamePawn_N` spawns, appearance scheduled) and the deliveries reached the dispatcher.

## Why the stance

`GameBeing` `onStateFieldUpdate` (`FUN_00e01c90`, [state-flag-broadcast.md](state-flag-broadcast.md)) calls `UpdateCombatStanceWeaponSet` (`FUN_00e7b4c0`) on a `BSF_InCombat` change. It re-keys the pawn's animation set from the stance codes at `pawn+0x3d0..0x3d2` (the last is the weapon category, [state-field-bits.md](../../architecture/state-field-bits.md)) and is a no-op until the pawn and its anim tables exist. For players the server has always sent the bit (and a weapon `BeingAppearance`) before the first shot. For NPCs it sent nothing: `npc_ai_submit`'s own comment records that "nothing ever calls `set_state_flag(BSF_IN_COMBAT)`", and the guard 100307 received one `onStateFieldUpdate` between its creation and its death 85 s later, and that one was the death.

Legacy behaviour (`deprecated/python/cell/SGWMob.py:158-163,292`, `SGWBeing.py:746-755`): `aiBeginCombat` did `setStateFlag(BSF_InCombat)`, which sent `onStateFieldUpdate` to the mob's witnesses; the idle transition cleared it. The client also carries a combat-state Kismet event (`SeqEvent_CombatStateChanged`, the engage-music system) that is designed for entities other than the local player.

## The change

`npc_ai/combat_stance.rs` (`cimmeria-cell-combat`): `BSF_InCombat` is set on an NPC exactly while `ai_state == Fighting`, and one `onStateFieldUpdate` goes to its witnesses when the announced stance changes. It runs at the top of `npc_ai_fight` (so the stance precedes the first shot on the reliable channel, including the fast-retry sweep) and once per AI tick for every other ticked state (so a leash, submit or give-up gets its clear). An NPC arriving in a witness's AoI mid-fight gets the bit from the cascade (`NpcAoIData::state_field`).

New rows: `npc_ai.stance` (DEBUG, `event = in_combat | out_of_combat`, `witness_count`) and the `abilities.sequence` WARN `outcome = stance_not_announced` for an NPC Ability_End that reached a witness before its stance did. See [negative-logging-convention.md](../../architecture/negative-logging-convention.md#npc-attack-animation-seams-na43).

## Acceptance test (colo, SigNoz)

After the release, for a guard shooting the player with the player idle:

1. Server: an `npc_ai.stance` row `event=in_combat` for the guard precedes its first `abilities.sequence ... ability_end` row.
2. Client: within ~100 ms of each `Event_NetIn_onSequence` delivery for that guard (`client.mercury.entity_method`, `msg_id = 1`, `type_id = 4`) there is a `client.engine.spawn_actor` `EmitterSpawnable` (pistol and SMG) and, for sequence 3, a `sing` `client.audio.event`.
3. If the deliveries still spawn nothing, the stance was not the missing input; the next suspect is the Source pawn's weapon slot data (`SeqAct_ComponentSlotData` on the NPC's composited weapon component) and a lab-bridge probe of `entity+0x3d0..0x3d2` on a guard is the next step.

## Open questions

- **Not proven in a client.** No client run with the fix exists. The evidence is (a) the controlled comparison above, (b) legacy parity, (c) the client's stance/slot structure. If the acceptance test fails, this finding's cause is wrong and it must be marked disputed.
- **The player's own "shooting animation breaks after respawn".** The telemetry cannot show an arm animation. What it does show: the server keeps sending the same ability sequences after every same-world reanchor (170 player shots across five respawns), and the player's `EmitterSpawnable` and weapon sounds continued after respawns 1 and 3 (tracers at 709412-709440 s). So the effect chain survives the pawn recreate; if the arm pose is what breaks, it is not in the data. The reanchor replays only `BeingAppearance`, `onEntityTint`, level, state field, stats, archetype, ability tree, hotbar, active slot and missions (`resync.rs`); it does not resend `onKismetEventSetUpdate` (1025), which world entry sends. Whether the recreated pawn needs it is unverified.
- **The SMG guard's shots and the Pistol Shot fallback.** The Cellblock guards fire 559 or 579 depending on the template; that mapping was not changed here.

## Evidence trail

| Claim | Source |
|---|---|
| Sequence ids 15, 3, 1866 and their scripts | `db/resources/Events/Seed/sequences.sql:785,3063,1269` |
| Ability 221 -> event set 802 "Energy shock ability source" | `db/resources/Events/Seed/event_sets.sql:803`, `event_sets_sequences.sql:3193` |
| Guard appearance components | `db/resources/Entities/Seed/entity_templates.sql` (templates 24, 15, 148...) |
| `BeingAppearance` sizes 440 / 446 / 146 | `client.mercury.entity_method` `msg_id = 26`, session `dc4c716a` |
| Player-sourced shots spawn emitters, NPC ones do not | `client.engine.spawn_actor` vs server `abilities.sequence` rows, above |
| Client `onSequence` path | Ghidra `FUN_00d05790`, `FUN_00d13780`, `FUN_00d06f30`, `FUN_00d06dd0`, `BW__unknown_00d00c10` |
| Kismet chain | `KIS-abilities_human.upk` `KIS-SA_Sing_Source`, `KIS-SA_Burst_Source` (`cimmeria-upk`) |
| Legacy mob combat bit | `deprecated/python/cell/SGWMob.py:158-163,292`, `SGWBeing.py:746-768` |
| Guard 100307's only state update is its death | `client.mercury.entity_method` `msg_id = 19` at 819.993 s |
