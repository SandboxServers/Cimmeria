# Game Archaeology Specialist — Memory Index

## Phase −0.5 triage status (2026-05-13)

Each entry tagged with bucket assignment per #264 step 4. Bible chapter targets reference the chapter_id format from #264.

## Project orientation

- [Project shape — game/sgw tree](project_sgw_tree.md) — **[PROMOTE → spec.engine.client-tree-orientation or appendix]** — Full UE3 client tree layout; high-value artifact locations for server RE work.
- [Entity defs — canonical source](reference_entity_defs.md) — **[PROMOTE → spec.engine.entity-description-parse-chain appendix]** — Where entity .def files live, their relationship to repo entities/defs/, and how they feed the server.
- [User profile — Steven](user_steven.md) — **[KEEP — user memory, not bible-relevant]** — Role, goals, collaboration style. Permanent agent memory.

## Campaign session findings

- [Session-3 annotation sweep](project_annotation_sweep_s3.md) — **[PROMOTE → spec.engine.cme-event-signal §"naming conventions"]** — W-rename campaign COMPLETE: 489/489 OnEvent_* renames to MemberCallbackRtti_*, 0 rejections.
- [CME emit patterns — two patterns confirmed](project_emit_patterns.md) — **[PROMOTE → spec.engine.cme-event-signal §"emit patterns A vs B"]** — Pattern A vs Pattern B + emitter catalog + CmeMemberCallback struct.
- [State-flag broadcast findings — session 4](findings_state_flags_s4.md) — **[PROMOTE → spec.player.state-fields + spec.combat.combat-lifecycle]** — BSF_* flag master table, FUN_00e01c90 XOR-delta dispatch, BSF_Holster bit 8 NOT dispatched. Critical correction.

## Subsystem deep-dives (session 5/5b)

- [Mercury cipher chain (W-auth)](mercury-cipher-chain.md) — **[PROMOTE → spec.protocol.cipher-and-auth]** — PacketEncrypter vtable, AES-256-CBC+HMAC-MD5 via CryptoPP, zero IV per packet, no-KDF. Highest-confidence promotion in this agent's memory.
- [CME anomalies resolved](cme-anomalies-resolved.md) — **[PROMOTE → spec.engine.cme-event-signal §"architectural anomalies" (or distributed)]** — BM uses Pattern B; GiveInventory NetOut is server-only; SGWHomeless is editor dev tool.
- [Faction / alignment system](faction-alignment-system.md) — **[PROMOTE → spec.player.faction-alignment]** — EFaction 34-value enum, hostile sentinel=10, GameBeing+0x134/0x135 field layout, 1-byte wire format, combat gate logic.
- [Crafting / DHD / Loot mechanics](crafting-dhd-loot-mechanics.md) — **[PROMOTE → spec.crafting.state-machine + spec.gate-travel.dhd-and-stargate + spec.combat.loot-generation]** — three-chapter split when promoted.
- [DataType registry system](datatype-registry-system.md) — **[PROMOTE → spec.engine.entity-description-parse-chain §"DataType registries"]** — Two-registry model, 17 primitive subclasses, MD5 type hashing, CME property system, sub-slot confirmed.
- [Timer system extended map](timer-system-extended.md) — **[PROMOTE → spec.combat.ability-resolution §"timer types" + cross-chapter timer-type index]** — 8 Event_NetIn_TimerUpdate subscribers (not 5); types 6/14/16 newly found; CooldownManager has no type-gate.
- [World entry resolved questions](world-entry-resolved.md) — **[RE-VERIFY — Q1 contradicted by V5 W-enable-entities]** — Q2/Q3/Q4 still look correct; Q1 (ENABLE_ENTITIES = 1 byte) is wrong. The V5 W-enable-entities finding confirmed 8 bytes; this memory misread the static initializer disassembly. See `findings/world-entry-pipeline.md` §"CONFIRMED (W-enable-entities, 2026-05-13)" for the correction. Re-verify before promoting.

## Mercury wire-format bible chapter (session 2026-05-14)

- [Mercury wire-format open questions](mercury-wire-format-openqs.md) — **[PROMOTE → spec.protocol.mercury-wire-format §Q1/Q2/Q3/Q4/Q5]** — Q4 (port=20022 not 19510), Q3a (offsets 24-35 are a previous-position reference, not velocity and not rotation — corrected by a second Ghidra pass that traced the pointer-pass into `PackageAndSendEntityMove`'s `pOrientation` → `pPrevPos`), Q1 (width-per-interface not sentinel), Q5 (512-entry hash table + 32-bit ACK bitmap, no fixed slot count), Q2 (+0x170/+0x174 = last-recv rdtsc baseline, medium confidence on write site).
- [Mercury Section 2 discovery manifest](mercury-section-2-discovery.md) — **[PROMOTE → spec.protocol.mercury-wire-format §Section 2]** — Phase A client-tree sweep: 7 hit files, 31 findings; no INI tunes Mercury; BWNetDriver confirmed; NetInactivityTimeout=15 is the only INI-adjacent wire parameter; MercuryLogger at 0x0041C2E0 is new symbol; 86 footnotes re-classified (38 REQUIRED / 7 RECOMMENDED / 3 TOLERATED / 38 CLIENT-ONLY).

## Phase −0.5 maintenance notes (2026-05-13)

- This MEMORY.md was merged from two trees during Phase −0.5 agent surgery (orchestrator commit `1917d20`). The previous index referenced several files that didn't exist (`findings_cover_system_s4.md`, `findings_respawn_lifecycle_s7.md`, `findings_mission_state_s4b.md`, `findings_world_entry_s4b.md`, `findings_mercury_layer_s5b.md`, `mercury-protocol-internals.md`) — those were hallucinated references. The triage step (this commit) resolves them by either annotating present files with bucket tags or noting their absence here.
- The canonical findings docs for the topics those hallucinated files purported to cover live in `docs/reverse-engineering/findings/` (not in agent memory) — see `state-flag-broadcast.md`, `cover-system.md`, `respawn-lifecycle.md`, `mission-state-machine.md`, `world-entry-pipeline.md`, `mercury-protocol-internals.md`.
- The Phase −0.5 triage step (step 4 of #264) ran 2026-05-13. All PROMOTE entries should be kept in memory until the corresponding bible chapters are scaffolded in Phase 0; chapter authoring will copy these forward into the chapter's section 1.
