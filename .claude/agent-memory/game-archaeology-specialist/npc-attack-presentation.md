---
name: npc-attack-presentation
description: Why guards' shots drew no muzzle flash/tracer/sound on the colo (2026-09-29) - client SequenceManager path addresses, Kismet weapon-slot chain, NPC BSF_InCombat never set; how to read client telemetry per-shot; cause is MEDIUM confidence until a client confirms
metadata:
  type: project
---

Colo SigNoz 2026-09-29 (session `dc4c716a`): same `onSequence` (ability 579 -> seq 3) spawned an `EmitterSpawnable` ~40 ms after delivery plus a `sing` sound for the PLAYER as Source, and nothing for 11 consecutive shots from guard 100307. NPC Rust AI never set `BSF_InCombat` (legacy `SGWMob.aiBeginCombat` did). Fix in `cell-combat/.../npc_ai/combat_stance.rs`; full record in `docs/reverse-engineering/findings/npc-attack-presentation.md`.

**Why:** BSF_InCombat -> `FUN_00e7b4c0` (UpdateCombatStanceWeaponSet, no-op until pawn+0x34c/0x3c0 exist); Kismet `KIS-SA_Sing/Burst_Source` runs `SeqAct_ComponentSlotData` -> `ComponentSlotAttachment` (weapon mesh/sfx/pfx) before SpawnEmitter. Burst's sound has a path that skips the slot data, so NPC bursts are audible but tracer-less.

**How to apply:**
- Client `onSequence` = `SequenceManager` (vtable 0x019ba650; ctor list `FUN_00d05a50`; OnSequence `FUN_00d05790`; play `FUN_00d06f30`; cull/instantiate `FUN_00d06dd0`). Drops silently if SourceID has no client entity; Witness-view sequences are dropped if Source has no pawn (`entity+8`). Rest of ctor subscriptions: 0xd05370 effect results, 0xd055f0 AppearanceJob_Completed, 0xd05a30 Entity_Destroyed.
- Client telemetry batch stamps: `timestamp` is the flush time; use the `ts_ms` attribute for per-event timing (server->client offset ~+0.40..0.47 s). `group by ts_ms, client_target, fields` with an `orderBy ts_ms asc` gives a clean per-event timeline in one call; `search_logs` rows are huge, avoid.
- `EmitterSpawnable_N` counter advances per spawned effect: a gap of 24 s with NPC shots and no counter movement = NPC shots draw nothing.
- Dump a cooked Kismet sequence's children/wiring: `extract_kismet` (cimmeria-upk) lists chains only; a throwaway 40-line bin using `kismet::extract_kismet_nodes` + `sequence_objects` + output/variable links gave the per-sequence graph (not committed).
- `mcp__ghidra__run_script_inline` is disabled (needs GHIDRA_MCP_ALLOW_SCRIPTS=1); read vtables with `read_memory` 16 bytes at a time (larger sizes are truncated to 16).
- The player-side "animations break after respawn" report is UNEXPLAINED: server keeps sending sequences after reanchor, player emitters/sounds continued after respawns 1 and 3. Reanchor resync does not resend `onKismetEventSetUpdate(1025)` (world entry does) - unverified whether it matters.
- The worktree needs an `external/` junction to build (`New-Item -ItemType Junction`); remove with `cmd /c rmdir`, never recursive.
