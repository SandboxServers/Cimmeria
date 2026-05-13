# Game Archaeology Specialist — Memory Index

- [Project shape — game/sgw tree](project_sgw_tree.md) — Full UE3 client tree layout; high-value artifact locations for server RE work.
- [Entity defs — canonical source](reference_entity_defs.md) — Where entity .def files live, their relationship to repo entities/defs/, and how they feed the server.
- [User profile — Steven](user_steven.md) — Role, goals, collaboration style.
- [Session-3 annotation sweep](project_annotation_sweep_s3.md) — W-rename campaign COMPLETE: 489/489 OnEvent_* renames to MemberCallbackRtti_*, 0 rejections, verification search confirmed 0 OnEvent_ remain.
- [CME emit patterns — two patterns confirmed](project_emit_patterns.md) — Pattern A (GetSystem+LookupByName+SetField) vs Pattern B (vtable-typed ctor); lower half has only 4 emit functions; SetField xref is the reliable discriminant.
- [State-flag broadcast findings — session 4](findings_state_flags_s4.md) — BSF_* flag master table, FUN_00e01c90 XOR-delta dispatch confirmed from assembly, BSF_Holster (bit 8) NOT dispatched in handler, root causes for #219/#232/#249.
- [Cover system findings — session 4](findings_cover_system_s4.md) — SGWCoverSet ServerOnly, ACoverLink/CoverInfo/CoverSpace addresses, issue #209 NPC cover implementation plan, CoverNodePrefabData layout.
- [Respawn lifecycle findings — session 7](findings_respawn_lifecycle_s7.md) — 12 functions annotated; corpse model confirmed (in-place BSF_Dead, no separate corpse entity); onBeginAidWait wire format; callForAid/GiveRespawner NetOut paths; issue #233 open (no binary evidence of per-player unlock).
- [Mission state machine findings — session 4b](findings_mission_state_s4b.md) — 14 MissionSet functions renamed/annotated; MissionSet/Entry/Step/Objective/Task field layouts; UI token table; CRITICAL: Count INT32 vs alias.xml INT8 wire mismatch to verify; task primitives not in binary.
- [World entry pipeline findings — session 4b](findings_world_entry_s4b.md) — 10 functions annotated/renamed; onClientMapLoad FIELD NAME AUDIT FINDING (clientMap/worldId wrong — actual: areaName/mapPath/WorldID/Location/Direction); BroadcastEntityActivation/PurgeAndRebuildEntityStateLists chain confirmed; BW_TO_UE3_SCALE=100.0f at DAT_018cad90.
- [Mercury layer completion — session 5b](findings_mercury_layer_s5b.md) — SCOPE COMPLETE: 145 functions renamed+annotated in [0x01576000, 0x0158efff]; full MachineGuard hierarchy (13 msg types), ChannelInternal lifecycle, UnAckedHandler, Packet chain ops, ChannelInternalPtr smart ptr, ProcessMessage vec ops all documented.
