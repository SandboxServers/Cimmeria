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

## Mercury wire-format bible chapter (sessions 2026-05-14/15)

- [Mercury Section 2 Track B evidence](mercury-section-2-track-b-evidence.md) — **[PROMOTE → spec.protocol.mercury-wire-format §2 — all subsections]** — R11–R15 behavior, 46 ClientInterface handler RTTI names, 37+ error strings catalogued, protocol_digest chain investigated, retry cap = 5.0f not 20, R13 sweep-timer claim UNCONFIRMED. **Investigation 6 (added 2026-05-15) contributed to Gap B closure by distinguishing two hashes that earlier passes had conflated: (a) the wire `protocol_digest` is MD5, 32 uppercase-hex chars, sourced from the CME `Event_Net_GetProtocolDigest` event listener and passed as `param_4` to `logOnBegin`; (b) a SEPARATE 40-char SHA-1 dispatch-table hash is stored at `ServerConnection+0x130` for internal commitment only — it is NOT the wire `protocol_digest`.**
- [Mercury Section 2 live-capture findings](mercury-section-2-live-capture-findings.md) — **[PROMOTE → spec.protocol.mercury-wire-format §2.5 and §2 protocol_digest subsection]** — Live x64dbg + AteraLoader session log capture. Closes Gap A (canonical 57-entry msg_id table with names + sizes) and Gap B (**protocol_digest is MD5/32-char from CME `Event_Net_GetProtocolDigest`; a separate 40-char SHA-1 lives at ServerConnection+0x130 as an internal dispatch-table hash — two distinct hashes**). Resolves static-vs-dynamic InterfaceElementVec architecture. 8-bit angle quantization confirmed from avatarUpdate variant size deltas. Live Mercury connection lifecycle traced from AteraLoader session log; tick rate = 10ms (100Hz). Atrea byte-patches fail under ASLR but symbol-based hooks (incl. MercuryLogger) still work.

## Mercury wire-format bible chapter (session 2026-05-14)

- [Mercury wire-format open questions](mercury-wire-format-openqs.md) — **[PROMOTE → spec.protocol.mercury-wire-format §Q1/Q2/Q3/Q4/Q5]** — Q4 (port=20022 not 19510), Q3a (offsets 24-35 are a previous-position reference, not velocity and not rotation — corrected by a second Ghidra pass that traced the pointer-pass into `PackageAndSendEntityMove`'s `pOrientation` → `pPrevPos`), Q1 (width-per-interface not sentinel), Q5 (512-entry hash table + 32-bit ACK bitmap, no fixed slot count), Q2 (+0x170/+0x174 = last-recv rdtsc baseline, medium confidence on write site).
- [Mercury Section 2 discovery manifest](mercury-section-2-discovery.md) — **[PROMOTE → spec.protocol.mercury-wire-format §Section 2]** — Phase A client-tree sweep: 7 hit files, 31 findings; no INI tunes Mercury; BWNetDriver confirmed; NetInactivityTimeout=15 is the only INI-adjacent wire parameter; MercuryLogger at 0x0041C2E0 is new symbol; 86 footnotes re-classified (38 REQUIRED / 7 RECOMMENDED / 3 TOLERATED / 38 CLIENT-ONLY).

## Entity property sync (2026-05-16)

- [Entity-property-sync OQ-1 CLOSED](entity-property-sync-oq1.md) — **[PROMOTE → spec.protocol.entity-property-sync §1.15 OQ-1 + OQ-X + F1 + G39]** — 0x3C/0x3D thresholds are server-side only; client uses UE3 FArchive uint32_t via FNetworkPropertyChange__vfunc_0 @ 0x015652d0. (Earlier sub-claim that 0x00dd0bb0 was misnamed as RemoveEntityListener has been retracted — Ghidra name is correct per audit Appendix E.)

## Auto-cycle / auto-fire system (2026-05-20)

- [Auto-cycle system findings](auto-cycle-findings.md) — **[PROMOTE → spec.combat.auto-cycle or doc-reference from gameplay/combat-system.md]** — Full wire path confirmed: cell method 83, 2-byte payload, server-driven loop, `BSF_AutoCycling` bit 1, cooldown-expiry re-fire pattern. Implementation gap identified.

## UE3 terrain serializer RE (2026-05-27)

- [UTerrain::Serialize binary layout](ue3-terrain-serialize.md) — **[PROMOTE → issue #46 appendix DONE; unblocks Phase 1.3]** — ATerrain__vfunc_12 @ 0x007517C0, full trailer layout, None-scan gotcha, 92% confidence.

## x64dbg session discipline

- [x64dbg session liveness check protocol](feedback-x64dbg-session-liveness-check.md) — **[KEEP — process-health check BEFORE any cave writes; 2026-06-22 crash postmortem]** — get_debugger_status + get_latest_event must both be clean before proceeding. Second-chance AV = dead process, no recovery.

## Black Market createAuction send-side (2026-06-22)

- [createAuction send wiring — NOP patch confirmed](bm-create-auction-send-wiring.md) — **[PROMOTE → docs/reverse-engineering/findings/black-market-client-window-patch.md §createAuction-send]** — Entity-guard JZ at 0x00e599a8 NOP'd (6 bytes), send BP hit confirmed, wire layout vs server decode verified. Full Lua binding map + reversibility bytes.
- [createAuction FUN_00A372F0 dispatch diagnosis](bm-create-auction-dispatch-diagnosis.md) — CME dispatch FULLY RECOVERED + STATIC PASS 2026-06-22 COMPLETE. Path 3 (vtable swap) DEAD on 3 grounds: hash key=TypeDesc not EventSignal vtable; type-equality guard rejects non-matching subscribers; method index from subscriber EventHandler->+0x04 not emitted TypeDesc. **THE FIX**: bucket pre-init (12 bytes: `8B 47 24 8B 4F 30 89 41 08 89 41 0C`) before `FUN_00A37790`. Bucket slot for TypeDesc `0x01E660B0` = **SLOT 2** (static for all realistic table sizes). fake_eh struct provides `method_index=0x3E` and channel_ptr=`[0x01EF2264]` at init. **FIXED CAVE: `0x01674420`** (208 bytes CC in .text, survives restarts, no ASLR). Complete byte layout in memory file.
- [createAuction registration key mismatch — SUPERSEDED](bm-create-auction-registration-key-bug.md) — FUN_00D46F70 hardcodes SellItems vtable (archaeological finding). Manual-callback_obj approach overridden by team-lead; approved path is FUN_00D6CE00 + SSO structs.
- [createAuction FINAL execution plan](bm-create-auction-next-session-plan.md) — **USE THIS on next relaunch.** Byte-exact cave (107 bytes at 0x01674420), SSO layouts verified. BLOCKER: FUN_00C6EA70 throws on first-registration (empty slot); awaiting team-lead decision on substitute (FUN_00A37790 direct). Dispatch thunk + NOP + rendezvous check all confirmed ready.

## Black Market fork-B session (2026-06-21)

- [BM fork-B crash notes + correct registration pattern](bm-fork-b-session-crash-notes.md) — Client crashed from flag3 handler calling 0x00403EC0 without first pushing _G. Correct sequence: 0x00403BB0(L,NULL) then 0x00403EC0. All cave addresses, JMP patches, and Lua shadow problem documented. All work survives client restart ONLY if caves were in fixed-VA exe space — they were not (heap allocs), so next session needs full rebuild.

## SGWHomeless full recovery (2026-05-25)

- [SGWHomeless full recovery](sgwhomeless-full-recovery.md) — **[PROMOTE → docs/reverse-engineering/findings/atrea-editor.md §SGWHomeless]** — Complete v5 RE: 30 CME subscriptions confirmed, all handler VAs, singleton at `0x01ef23fc` (CORRECTS prior `cme-anomalies-resolved.md`), GLevel ToD/Wind/Weather layout, Ghidra renames applied.

## Phase −0.5 maintenance notes (2026-05-13)

- This MEMORY.md was merged from two trees during Phase −0.5 agent surgery (orchestrator commit `1917d20`). The previous index referenced several files that didn't exist (`findings_cover_system_s4.md`, `findings_respawn_lifecycle_s7.md`, `findings_mission_state_s4b.md`, `findings_world_entry_s4b.md`, `findings_mercury_layer_s5b.md`, `mercury-protocol-internals.md`) — those were hallucinated references. The triage step (this commit) resolves them by either annotating present files with bucket tags or noting their absence here.
- The canonical findings docs for the topics those hallucinated files purported to cover live in `docs/reverse-engineering/findings/` (not in agent memory) — see `state-flag-broadcast.md`, `cover-system.md`, `respawn-lifecycle.md`, `mission-state-machine.md`, `world-entry-pipeline.md`, `mercury-protocol-internals.md`.
- The Phase −0.5 triage step (step 4 of #264) ran 2026-05-13. All PROMOTE entries should be kept in memory until the corresponding bible chapters are scaffolded in Phase 0; chapter authoring will copy these forward into the chapter's section 1.
