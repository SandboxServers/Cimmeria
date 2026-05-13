# Game Archaeology Specialist — Memory Index

## Project orientation

- [Project shape — game/sgw tree](project_sgw_tree.md) — Full UE3 client tree layout; high-value artifact locations for server RE work.
- [Entity defs — canonical source](reference_entity_defs.md) — Where entity .def files live, their relationship to repo entities/defs/, and how they feed the server.
- [User profile — Steven](user_steven.md) — Role, goals, collaboration style.

## Campaign session findings

- [Session-3 annotation sweep](project_annotation_sweep_s3.md) — W-rename campaign COMPLETE: 489/489 OnEvent_* renames to MemberCallbackRtti_*, 0 rejections, verification search confirmed 0 OnEvent_ remain.
- [CME emit patterns — two patterns confirmed](project_emit_patterns.md) — Pattern A (GetSystem+LookupByName+SetField) vs Pattern B (vtable-typed ctor); lower half has only 4 emit functions; SetField xref is the reliable discriminant.
- [State-flag broadcast findings — session 4](findings_state_flags_s4.md) — BSF_* flag master table, FUN_00e01c90 XOR-delta dispatch confirmed from assembly, BSF_Holster (bit 8) NOT dispatched in handler, root causes for #219/#232/#249.

## Subsystem deep-dives (session 5/5b)

- [Mercury cipher chain (W-auth)](mercury-cipher-chain.md) — PacketEncrypter vtable, AES-256-CBC+HMAC-MD5 via CryptoPP, zero IV per packet, key derivation confirmed no-KDF.
- [CME anomalies resolved](cme-anomalies-resolved.md) — BM uses Pattern B; GiveInventory NetOut is server-only (no client subscriber); SGWHomeless is class_SGWHomeless (editor dev tool, not catch-all).
- [Faction / alignment system](faction-alignment-system.md) — EFaction 34-value enum, hostile sentinel=10, GameBeing+0x134/0x135 field layout, 1-byte wire format, combat gate logic.
- [Crafting / DHD / Loot mechanics](crafting-dhd-loot-mechanics.md) — VCrafting class, EmitNetOut_onDialGate 6-glyph resolution, StargateTriggerFailed (new event), VLootables cache-warm pattern, ring transporter fields.
- [DataType registry system](datatype-registry-system.md) — Two-registry model (01f126b8/01f126b4), 17 primitive subclasses, MD5 type hashing, CME property system, sub-slot confirmed. High-half 015a8b40–015bffe0 = non-protocol editor tooling (PE debug, Win32ThreadEx, GFx importer, unrpc SpawnPoint API).
- [Timer system extended map](timer-system-extended.md) — 8 Event_NetIn_TimerUpdate subscribers (not 5); types 6/14/16 newly found; CooldownManager has no type-gate; GameEntityManager ctor is non-standard (3 data params, 0x10 bytes).
- [World entry resolved questions](world-entry-resolved.md) — Q2/Q3/Q4 resolved with binary addresses; Q5/Q6 still open. **Q1 (ENABLE_ENTITIES size) CORRECTION:** the W-misc-gaps finding "1 byte" was wrong — W-enable-entities Session 5b reconciliation confirmed SGW uses 8 bytes (uint64 dummy); only stock BigWorld 2.0.1 uses 1 byte. See `docs/reverse-engineering/findings/world-entry-pipeline.md` "ENABLE_ENTITIES payload reconciliation" section.

## Phase −0.5 maintenance notes (2026-05-13)

- This MEMORY.md was merged from two trees during Phase −0.5 agent surgery. The previous index referenced several files that didn't exist (`findings_cover_system_s4.md`, `findings_respawn_lifecycle_s7.md`, `findings_mission_state_s4b.md`, `findings_world_entry_s4b.md`, `findings_mercury_layer_s5b.md`, `mercury-protocol-internals.md`) — hallucinated references the triage step should resolve (PROMOTE / RE-VERIFY / DISCARD per #264 Phase −0.5).
- The canonical findings docs for those topics live in `docs/reverse-engineering/findings/` (not in agent memory) — see `state-flag-broadcast.md`, `cover-system.md`, `respawn-lifecycle.md`, `mission-state-machine.md`, `world-entry-pipeline.md`, `mercury-protocol-internals.md`.
