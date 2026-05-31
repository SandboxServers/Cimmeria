---
title: "Project Status"
type: reference
audience: anyone tracking the project
last_updated: 2026-05-27
---

# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

> This document summarizes the findings of the [Gap Analysis](gap-analysis.md), which now tracks **437 individual features across 44 systems** (36 gameplay + 8 infrastructure) against the active Rust codebase. The previous edition counted against the deprecated Python + C++ trees; it overstated some completion (counting Python `pass` stubs as "code exists") and understated others (the content engine, observability, wireclient, and Discord crates didn't exist back then).

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | CW | Tested end-to-end with the game client (Castle Cellblock smoke + Lomiada captures) and verified correct |
| **Needs Test** | NT | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | IM | Code written but may be incomplete or have known issues |
| **Known / Missing** | KM | We know this needs to exist but no code exists in `crates/` |
| **Needed / Unknown** | NU | Server-only system we infer must exist but have no direct evidence for |

## Overall Completion

| Status | Features | Percentage |
|--------|----------|-----------|
| Confirmed Working (CW) | 139 | 31.8% |
| Needs Test (NT) | 18 | 4.1% |
| Implemented (IM) | 91 | 20.8% |
| Known/Missing (KM) | 184 | 42.1% |
| Needed/Unknown (NU) | 5 | 1.1% |
| **Total** | **437** | |

**Code exists (CW + NT + IM)**: 248 features (56.8%)  
**Missing (KM + NU)**: 189 features (43.2%)  
**Tested end-to-end (CW)**: 139 features (31.8%)

## System Status

### Infrastructure — Solid

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Authentication & login | CW | 11 (6 CW, 3 IM, 2 KM) | Full login flow tested. SHA1 → bcrypt and continuous validation pending |
| Mercury protocol | CW | 13 (11 CW, 2 KM) | AES-256-CBC + HMAC-MD5 transport, Transport trait, ChannelBundle, loopback harness, pcap replay, chaos primitives. Missing: cumulative + piggyback ACKs |
| Game data pipeline | CW | 7 (6 CW, 1 KM) | 22 resource categories, 112,626 DB rows, PAK overrides for missions and items. Hot reload pending |
| Database persistence | CW | 8 (7 CW, 1 KM) | sqlx with compile-time query checks, durable Base→Cell outbox, 155 live-DB regression guards. No migration framework yet |

### Core Gameplay — Real Code, Mostly Working

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Character creation | NT | 11 (8 NT, 3 KM) | 1,640 lines incl. delete + visuals live-DB tests. Full client smoke would move to CW |
| World entry & spaces | CW | 9 (7 CW, 2 IM) | 22,682 lines across 64 files. Castle Cellblock end-to-end. 23 other zones unchecked |
| Movement & navigation | IM | 9 (1 CW, 4 IM, 4 KM) | Player movement works; NPC navigation states blocked on full Detour wiring |
| Entity lifecycle (AoI) | CW | 9 (7 CW, 1 IM, 1 KM) | Grid-based AoI, witness lifecycle, BeingAppearance fanout helper pending (#278) |
| Combat & abilities | IM | 23 (5 CW, 15 IM, 3 KM) | 5,918 lines + 142 tests. PR #420 closed ability+effect gaps (#47, #61, #331, #416, #419) |
| Effects & buffs | IM | 13 (4 CW, 7 IM, 2 KM) | Framework CW. Long tail of 3,217 effect rows needs script coverage |
| Stats | IM | 8 (5 CW, 2 KM, 1 NU) | Stat list + dirty sync + per-level scaling shipped. Equipment bonuses + derived formulas pending |
| Inventory & items | IM | 13 (9 CW, 2 NT, 2 KM) | 5,272-line dispatcher with stacking (#405), bandolier discipline, Slappack PAK override |
| Missions | IM | 12 (8 CW, 2 IM, 2 KM) | Content-engine driven. Castle Cellblock end-to-end. Sharing + mission-gated loot pending |
| Loot | IM | 9 (2 CW, 2 NT, 2 IM, 3 KM) | Take-all + bag drop verified. Tables mostly empty; group eligibility unwired |
| Vendors | NT | 8 (2 CW, 5 NT, 1 IM) | 7,267 lines across buyback / purchase / sell / paid_repair / paid_recharge submodules. PL/pgSQL smoke verifies the loop |

### NPC Systems — Partial

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| NPC AI & behavior | IM | 22 (6 CW, 5 IM, 11 KM) | Combat AI works. Patrol / wander / leash blocked on Navigation. Cover system (1,332 Atrea nodes) unimplemented |
| Spawn system | IM | 23 (7 CW, 11 IM, 4 KM, 1 NU) | 1,983 lines, 23 tests. Castle Cellblock lifecycle CW. Time-of-day + linked sets pending |

### Secondary Systems

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| XP & leveling | IM | 11 (9 CW, 2 KM) | Kill-XP pipeline + level scaling + training points CW. Mission XP needs chain authoring; ASP grant on level-up pending |
| Crafting | KM | 9 (all KM) | **Not ported.** Python Crafter.py (575 lines, ~95% in deprecated tree) needs full transplant |
| Stargate travel | IM | 10 (2 CW, 5 IM, 3 KM) | Gate passage CW. Multi-player sync + return-trip state + cooldown pending |
| Chat | NT | 9 (1 NT, 8 KM) | Say/emote/yell only. Channels, tells, moderation all unported |
| Trading | KM | 8 (all KM) | **Not ported.** Python Trade.py (244 lines) needs full transplant |

### Stub-Only / Largely Missing

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Organizations / guilds | KM | 15 (all KM) | 200 lines of stubs in cell_methods/organization.rs. DB schema needed |
| Mail | IM | 13 (8 IM, 4 KM, 1 NU) | 1,456 lines in base/world_entry/methods/mail/. CoD + return-to-sender + new-mail fanout pending |
| Black market | KM | 10 (9 KM, 1 NU) | 94 lines of stubs. Needs sgw_auction schema + lifecycle |
| Contact lists | KM | 8 (all KM) | 86 lines of stubs. Needs sgw_contact_list schema |
| Dueling | KM | 6 (all KM) | Not ported. 5-state machine + 7 defeat conditions to implement |
| Pets | KM | 7 (all KM) | Not ported. Entity extends spawner mob + Follow AI state |
| Minigames | KM | 9 (4 IM, 5 KM) | Session routing IM. 8 game types unimplemented (SmartFox 1.x external server) |
| Groups / parties | KM | 7 (all KM) | Not ported. Lightweight Squad-type Organization |

### Systems New Since the Original Audit

These didn't exist in the Python codebase and so weren't tracked. They're substantial in Rust today.

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Content engine | CW | 10 (6 CW, 4 KM) | 7,906 lines in cell/content/ + 3,561 in content-engine/. 99 tests. Drives missions/dialogs/triggers/conditions/actions |
| Mercury bundle | CW | 5 (5 CW) | ChannelBundle accumulator, AoI-burst bundling, backpressure handling |
| Observability pipeline | CW | 8 (8 CW) | OTLP exporter, SigNoz overlay, Mercury packet logging, dev-session telemetry, negative-logging convention |
| Wireclient + chaos | CW | 7 (7 CW) | Tier 3 headless client + LossyTransport + loopback harness + pcap replay |
| Discord notifications | CW | 6 (6 CW) | Event routing, channel toggles, embed formatting, panic-hook capture |
| Tauri admin app + tools | IM | 9 (1 CW, 7 IM, 1 KM) | Admin API, content editor, scene editor, sgw-launcher. Three.js space viewer pending |

### Server Infrastructure (Cross-Cutting)

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Session management | IM | 6 (1 CW, 2 IM, 3 KM) | Inactivity timeout works. No reconnection grace or continuous validation |
| Rate limiting | KM | 5 (1 CW, 4 KM) | Ability cooldowns enforced. Chat / action / trade / login throttling pending |
| Anti-cheat | KM | 7 (1 CW, 2 IM, 4 KM) | Inventory ownership checked. No speed/teleport/damage validation |
| Economy | IM | 7 (5 CW, 2 KM) | Vendor/loot/mission cash all flowing. AH listing fees + cash-flow tracking pending |
| World state | IM | 6 (2 CW, 1 IM, 3 KM) | Outbox CW. Gate/door state + world-state table pending |
| Scheduler | IM | 4 (1 IM, 3 KM) | Per-chain timers via content engine. No global cron |
| Admin / GM | IM | 11 (5 IM, 6 KM) | Admin API + Tauri panel IM. Ban/mute, teleport, item-grant pending |
| Metrics / telemetry | CW | 7 (4 CW, 3 IM) | Full OTLP pipeline. Per-player and dashboard polish ongoing |

## Content Coverage

| Content Type | Total in DB | Tested/Verified | Notes |
|--------------|-------------|-----------------|-------|
| Zones | 91 world definitions | 1 (Castle Cellblock) routinely; some others manually | Multi-zone smoke is on the critical path |
| Missions | 1,041 | ~5 in Castle Cellblock | Content engine drives mission chains generically |
| Abilities | 1,887 | many | Three-bucket selection landed (#368), PR #420 closed ability gaps |
| Items | 6,060 | ~30 routinely | Slappack stacking + bandolier discipline verified |
| Effects | 3,217 | framework + most-common scripts | Long tail still needs content authoring |
| NPCs | 153 templates | ~12 routinely | Castle Cellblock NPCs + Castle drone encounter |
| Dialog trees | 5,406 | ~10 | Castle Cellblock dialogs verified |
| Stargates | 29 | Castle ↔ neighbor smoke | Multi-player sync pending |
| Crafting blueprints | 499 | 0 | Subsystem not ported |
| Loot tables | defined | mostly empty | Algorithm verified; content sparse |

## Known Issues

### Combat formula calibration

Combat works at a basic level but several formulas are still calibration items:

- No diminishing returns on stats (NU)
- Armor / resistance calibration vs. original is approximate
- AoE damage falloff curves need verification (PR #420 landed AoE framework)
- Cover system mechanics minimal (1,332 cover nodes unimplemented)

### Mercury protocol gaps

The transport layer works but a few BigWorld features are still missing:

- No cumulative ACKs (per-packet only)
- No piggyback ACKs
- Reconnection grace period missing (instant disconnect = lost session)

### Effect content gap

The framework is CW (PR #420). The long tail of the 3,217 effect rows still needs script coverage — the most common scripts are wired; the niche ones are not.

### Crafting and Trading not ported

These two Python subsystems (575 lines and 244 lines respectively) never made the jump to Rust. They are firmly KM and on the critical path.

### Single-zone coverage

Only Castle Cellblock is routinely smoked end-to-end. The other 23 spaces have content but no continuous verification.

## Critical Path for Playability

In rough priority order:

1. **Effect-script content coverage** — framework CW (#420); the 3,217 effect rows need script authoring for the long tail
2. **Mission XP** — `mission.reward_xp` is 0 in all seed rows; chain-side authoring + `Action::GrantXP` wiring
3. **NPC navigation states** — patrol / wander / leash blocked on full Detour wiring
4. **Crafting port** — full subsystem missing from Rust
5. **Trading port** — full subsystem missing from Rust
6. **Multi-zone end-to-end** — 23 spaces unchecked

Quality-of-life items (organizations, mail polish, black market, contact lists, dueling, pets, minigame port, groups, GM tools) follow the above and can be picked up independently.

## Roadmap

### Near-term — close critical-path gaps

- Effect-script coverage for the most-played encounters
- Mission XP chain-authoring
- NPC patrol / wander / leash wiring (Detour pass)
- Multi-zone routine smoke

### Medium-term — restore retired subsystems

- Port Crafting (Crafter.py → Rust)
- Port Trading (Trade.py → Rust)
- Org / guild lifecycle + schema
- Mail polish (CoD, return-to-sender, new-mail fanout)

### Long-term — finish-out

- Black market + contact lists + dueling + pets + groups + minigames (in any order)
- Server infrastructure: rate limiting, anti-cheat (speed/teleport/damage), reconnection grace, GM-command surface
- Three.js space viewer (Phase 2 of the admin UI)

## Related Documents

- [Gap Analysis](gap-analysis.md) — per-feature status tracking (source of truth)
- [Gameplay Dashboard](gameplay/README.md) — per-system gameplay breakdowns
- [Content Engine](content/content-engine.md) — the data-driven runtime
- [NPC AI](gameplay/npc-ai.md) — AI state machine and threat system
- [Spawn System](gameplay/spawn-system.md) — spawn region/set architecture
- [Loot System](gameplay/loot-system.md) — loot generation algorithm
- [Progression](gameplay/progression-system.md) — XP, leveling, training points
- [Character Creation](gameplay/character-creation.md) — character creation flow
- [Server Systems](architecture/server-systems.md) — server-only infrastructure
- [../README.md](../README.md) — high-level project status
