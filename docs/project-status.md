# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

> This document summarizes the findings of the [Gap Analysis](gap-analysis.md), which tracks 369 individual features across 30 gameplay systems and 8 infrastructure systems.

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | CW | Tested end-to-end with the game client and verified correct |
| **Needs Test** | NT | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | IM | Code written but may be incomplete or have known issues |
| **Known / Missing** | KM | We know this needs to exist but no code exists |
| **Needed / Unknown** | NU | Server-only system we infer must exist but have no direct evidence for |

## Overall Completion

| Status | Features | Percentage |
|--------|----------|-----------|
| Confirmed Working (CW) | 31 | 8.4% |
| Needs Test (NT) | 49 | 13.3% |
| Implemented (IM) | 95 | 25.7% |
| Known/Missing (KM) | 191 | 51.8% |
| Needed/Unknown (NU) | 3 | 0.8% |
| **Total** | **369** | |

**Code exists (CW + NT + IM)**: 175 features (47.4%)
**Missing (KM + NU)**: 194 features (52.6%)

## System Status

### Infrastructure — Solid

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Authentication & login | CW | 8 (5 CW, 3 IM) | Full login/auth flow tested and reliable |
| Mercury protocol | CW | 7 (5 CW, 2 KM) | Reliable UDP transport works. Missing cumulative ACKs, piggyback |
| Game data pipeline | CW | 4 (3 CW, 1 KM) | 22 resource categories, 112,626 DB rows served |
| Database persistence | CW | 5 (4 CW, 1 IM) | SOCI ORM, player/inventory/mission state persisted |
| Python embedding | CW | — | Boost.Python 3.4 embedding works, 164 scripts load |

### Core Gameplay — Real Code Exists

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Character creation | NT | 11 (9 NT, 2 KM) | Account.py ~300 lines, full flow with validation. Needs client test |
| World entry & spaces | CW | 8 (5 CW, 3 IM) | 1 zone tested (Castle Cellblock). Others need verification |
| Movement & navigation | IM | 9 (1 CW, 5 IM, 3 KM) | Player movement works. No server-side speed validation |
| Entity lifecycle (AoI) | CW | 7 (5 CW, 2 KM) | Grid-based AoI works. No LOD system |
| Combat & abilities | IM | 21 (14 IM, 7 KM) | 1,090-line AbilityManager. Single-target works. AoE/deploy missing |
| Effects & buffs | IM | 13 (8 IM, 5 KM) | Framework works. 4 of 3,217 effects have scripts |
| Stats | IM | 8 (4 IM, 3 KM, 1 NU) | Stat class works. Missing: derived formulas, level scaling |
| Inventory & items | NT | 11 (1 CW, 8 NT, 2 KM) | 21K-line Inventory.py. Full bag/equip/move logic |
| Missions | IM | 12 (1 CW, 8 IM, 3 KM) | 29K-line MissionManager. 20 mission scripts. 1 zone tested |
| Loot | NT | 9 (5 NT, 1 IM, 3 KM) | Algorithm works (221 lines). Loot tables mostly empty |
| Vendors | NT | 7 (6 NT, 1 KM) | Vendor.py complete (buy/sell/repair/recharge/buyback) |

### NPC Systems — Partial/Server-Only

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| NPC AI & behavior | IM | 26 (11 IM, 15 KM) | 2 of 12 AI states work (Spawning, Fighting). No patrol/wander/leash |
| Spawn system | KM | 19 (18 KM, 1 NU) | 100% empty Python stubs. Rich .def properties reveal design |

### Secondary Systems — Code Exists, Untested

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| XP & leveling | IM | 11 (6 IM, 5 KM) | giveExperience() works. Placeholder XP table. No stat scaling |
| Crafting | NT | 9 (8 NT, 1 KM) | Crafter.py 575 lines, ~95% implemented. Untested |
| Stargate travel | IM | 9 (6 IM, 3 KM) | Dial/timer/passage work. Missing multi-player sync, return trips |
| Chat | NT | 10 (6 NT, 4 KM) | Chat.py 352 lines. Messaging works. 11 admin methods = stubs |
| Trading | NT | 8 (7 NT, 1 KM) | Trade.py 244 lines. P2P trade slots. Untested |

### Stub-Only Systems — No Real Implementation

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Organizations/guilds | KM | 15 (all KM) | All 15+ RPC methods are `pass`/`trace()` |
| Mail | KM | 13 (2 IM, 10 KM, 1 NU) | sendMailMessage = `pass`. Some read-only partial |
| Black market | KM | 10 (all KM) | All 6 handler methods = `pass` |
| Contact lists | KM | 8 (all KM) | All 6 methods = empty stubs |
| Dueling | KM | 6 (all KM) | All methods = `pass`/`trace()` |
| Pets | KM | 7 (2 IM, 5 KM) | Entity + basic init exists. No behavior |
| Minigames | IM | 9 (4 IM, 5 KM) | 3 of 9 have logic, 6 are placeholders. None tested |
| Groups | KM | 7 (all KM) | SGWPlayerGroupAuthority.py = empty shell |

### Server Infrastructure — Mostly Missing

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Session management | IM | 6 (3 IM, 3 KM) | Basic timeout exists. No reconnection grace, no concurrent limit |
| Rate limiting | KM | 5 (1 IM, 4 KM) | No action throttling. Exploitable |
| Anti-cheat | IM | 7 (3 IM, 4 KM) | Post-login challenge only. No runtime validation |
| Economy | IM | 6 (3 IM, 3 KM) | Vendor prices static. No sink/faucet tracking |
| World state | IM | 5 (1 CW, 2 IM, 2 KM) | Space scripts reset on restart |
| Scheduler | KM | 4 (1 IM, 3 KM) | No timed events, no cron-like system |
| Admin/GM tools | IM | 10 (4 IM, 6 KM) | 22 GM commands are stubs in SGWPlayer.py |
| Metrics/telemetry | KM | 5 (1 IM, 4 KM) | Logging categories exist but no game metrics |

## Content Coverage

| Content Type | Total in DB | Tested/Verified | Notes |
|--------------|-------------|-----------------|-------|
| Zones | 91 world definitions | 1 (Castle Cellblock) | Other zones have space scripts but are untested |
| Missions | 1,041 | ~5 in Castle Cellblock | Most have scripts that exist but are unverified |
| Abilities | 1,887 | ~10-20 | Basic attacks tested, vast majority untested |
| Items | 6,060 | ~20-30 | Quest items and basic gear tested |
| Effects | 3,217 | 4 have scripts | Massive content gap — generator tool created |
| NPCs | 153 templates | ~10 | Castle Cellblock NPCs tested |
| Dialog trees | 5,406 | ~10 | Castle Cellblock dialogs tested |
| Stargates | 29 | 0 end-to-end | DHD UI works, gate passage does not |
| Crafting blueprints | 499 | 0 | System untested |
| Loot tables | defined | mostly empty | Algorithm works, tables need population |

## Known Issues

### Combat Formula Gaps

Combat works at a basic level but many formulas are unvalidated:
- No diminishing returns on stats
- Armor/resistance calculations may not match original
- Ability scaling with level is undefined
- Area-of-effect abilities not implemented
- Cover system mechanics are minimal

### Mercury Protocol Gaps

The transport layer works but lacks some BigWorld features:
- No cumulative ACKs (individual only)
- No piggyback packet support
- Native little-endian byte ordering (works because both sides are x86)
- Untested with multiple simultaneous clients

### Effect Content Gap

Only 4 of 3,217 effects have Python scripts. The effect framework works but virtually no effect content exists. A generator tool (`tools/generate_effect_stubs.py`) has been created to bootstrap stubs.

### Spawn System Empty

SGWSpawnRegion.py and SGWSpawnSet.py are 100% empty shells. Without the spawn system, NPCs only exist via space scripts (static placement). Dynamic population management is completely missing.

## Critical Path for Playability

The minimum viable gameplay loop requires these systems in order:

1. **Spawn System** (KM) — Without it, no mobs exist in the world dynamically
2. **NPC Navigation** (KM) — Without it, mobs stand still during combat
3. **XP from Kills** (KM) — Primary progression driver
4. **Stat Scaling** (KM) — Makes leveling meaningful
5. **Loot Table Content** (KM) — Makes combat rewarding
6. **Effect Scripts** (KM) — 4 of 3,217 effects have scripts

Once these are addressed, the core combat→loot→level loop is functional.

## Roadmap

### Near-term: Validate What Exists

- Test character creation end-to-end with a live client
- Test chat, crafting, trading, vendor systems with a live client
- Verify space scripts beyond Castle Cellblock
- Validate combat formulas against original client behavior

### Medium-term: Fill Critical Gaps

- Implement spawn system (SGWSpawnRegion/Set population management)
- Implement NPC patrol, wander, leash, and flee AI states
- Define real XP curves and stat scaling per level
- Populate loot tables from game data
- Generate effect script stubs and implement key effects
- Complete AoE and deploy ability targeting modes

### Long-term: Build Missing Systems

- Organizations/guilds (15+ RPC methods)
- Mail system (send, receive, attachments, COD)
- Black market/auction house
- Contact lists, dueling, pets
- Complete minigame implementations
- Server infrastructure (rate limiting, anti-cheat, session management)

## Related Documents

- [Gap Analysis](gap-analysis.md) — Per-feature status tracking (source of truth)
- [Gameplay Dashboard](gameplay/README.md) — Per-system gameplay breakdowns
- [NPC AI](gameplay/npc-ai.md) — AI state machine and threat system
- [Spawn System](gameplay/spawn-system.md) — Spawn region/set architecture
- [Loot System](gameplay/loot-system.md) — Loot generation algorithm
- [Progression](gameplay/progression-system.md) — XP, leveling, training points
- [Character Creation](gameplay/character-creation.md) — Character creation flow
- [Server Systems](architecture/server-systems.md) — Server-only infrastructure
