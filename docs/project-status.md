---
title: "Project Status"
type: reference
audience: anyone tracking the project
last_updated: 2026-07-25
---

# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

> This document summarizes the findings of the [Gap Analysis](gap-analysis.md), which tracks **443 individual features across 45 systems** (37 gameplay + 8 infrastructure) against the active Rust codebase on `main`.
>
> **Re-verified 2026-07-25** against the code, after 168 commits landed since the previous (2026-05-27) edition. Two classes of correction came out of that pass: features that had shipped but were still listed as missing (contact lists, trading, NPC movement states, the cover system, the GM command surface, movement validation, the minigame server), and features listed as working that were not (the whole wireclient replay story). The previous edition's headline numbers also did not match its own per-system table — it printed 437 / CW 139 / KM 184 / NU 5 against rows that summed to 428 / CW 151 / KM 164 / NU 4. The figures below are recomputed from the rows.
>
> **Scope note**: only work merged to `main` is counted. The black-market implementation on `feat/571-black-market-phase1` is real but unmerged, and is counted as missing until it lands.

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
| Confirmed Working (CW) | 159 | 35.9% |
| Needs Test (NT) | 18 | 4.1% |
| Implemented (IM) | 134 | 30.2% |
| Known/Missing (KM) | 128 | 28.9% |
| Needed/Unknown (NU) | 4 | 0.9% |
| **Total** | **443** | |

**Code exists (CW + NT + IM)**: 311 features (70.2%)  
**Missing (KM + NU)**: 132 features (29.8%)  
**Tested end-to-end (CW)**: 159 features (35.9%)

The gap between "code exists" (70.2%) and "confirmed working" (35.9%) is the story of this quarter: a lot shipped between June and July with unit, live-DB, and wire coverage, but without a live-client run. Trading, ring transport, movement validation, the cover system, and the minigame server are all in that bucket.

## System Status

### Infrastructure — Solid

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Authentication & login | CW | 12 (6 CW, 4 IM, 2 KM) | Full login flow tested. TLS listener + cert hot-reload added (#566/#577). SHA1 → bcrypt and continuous validation pending |
| Mercury protocol | CW | 15 (10 CW, 3 IM, 2 KM) | v1 AES-256-CBC + HMAC-MD5 is the client-compatible default. **v2 shipped** (per-packet IV, HKDF-split keys, truncated HMAC-SHA256, downgrade defense, key rotation) but is opt-in and **untested against a live client**. Cumulative ACKs now implemented; piggyback ACKs still missing. The "pcap replay" row moved to KM — see Wireclient |
| Game data pipeline | CW | 7 (6 CW, 1 KM) | 22 resource categories, 112,626 DB rows, PAK overrides for missions and items. Hot reload pending |
| Database persistence | CW | 8 (7 CW, 1 KM) | sqlx with compile-time query checks, durable Base→Cell outbox, 259 live-DB regression guards. No migration framework yet |

### Core Gameplay — Real Code, Mostly Working

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Character creation | NT | 11 (8 NT, 1 IM, 2 KM) | 1,640 lines incl. delete + visuals live-DB tests. SGWGmPlayer now ported (#473/#518). Full client smoke would move to CW |
| World entry & spaces | CW | 9 (7 CW, 2 IM) | 22,682 lines across 64 files. Castle Cellblock end-to-end. 23 other zones unchecked |
| Movement & navigation | IM | 9 (1 CW, 8 IM) | **Detour is wired.** Pathfinding, LOS raycast, and navmesh containment all live; four-layer server-side movement validation shipped (#437/#478) |
| Entity lifecycle (AoI) | IM | 9 (6 CW, 2 IM, 1 KM) | **Downgraded from CW.** Grid-based AoI and witness lifecycle work, but there is a known-open entity-introduction drop (invisible corpse until relog); #582 added instrumentation, awaiting repro |
| Combat & abilities | IM | 23 (5 CW, 16 IM, 2 KM) | 5,918 lines + 142 tests. PR #420 closed ability+effect gaps. LOS primitive now exists and is enforced NPC-side, not yet on player `useAbility` |
| Effects & buffs | IM | 13 (4 CW, 7 IM, 2 KM) | Framework CW. Long tail of 3,217 effect rows needs script coverage |
| Stats | IM | 8 (5 CW, 2 KM, 1 NU) | Stat list + dirty sync + per-level scaling shipped. Equipment bonuses + derived formulas pending |
| Inventory & items | IM | 13 (9 CW, 2 NT, 2 KM) | 5,272-line dispatcher with stacking (#405), bandolier discipline, Slappack PAK override |
| Missions | IM | 12 (8 CW, 2 IM, 2 KM) | Content-engine driven. Castle Cellblock end-to-end. Sharing + mission-gated loot pending |
| Loot | IM | 9 (2 CW, 2 NT, 1 IM, 4 KM) | Take-all + bag drop verified; looter distance re-validated per item (#446). Tables mostly empty; there is **no** per-player eligibility list in Rust at all |
| Vendors | NT | 8 (2 CW, 5 NT, 1 IM) | 7,267 lines across buyback / purchase / sell / paid_repair / paid_recharge submodules. PL/pgSQL smoke verifies the loop |

### NPC Systems — Partial

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| NPC AI & behavior | IM | 22 (6 CW, 11 IM, 5 KM) | **Patrol / wander / investigate / follow / leash / despawn all shipped** (#428) — 1,818 lines split per state. **Cover system implemented** (#429) — 4,095 lines with node loading, scoring, per-node reservation, and flanking. Remaining gaps: hearing radius, mob groups, kill-credit tapping |
| Spawn system | IM | 23 (7 CW, 11 IM, 4 KM, 1 NU) | 2,448 lines. Castle Cellblock lifecycle CW. Time-of-day, detection radius, and linked sets still pending |

### Secondary Systems

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| XP & leveling | IM | 11 (9 CW, 2 KM) | Kill-XP pipeline + level scaling + training points CW. Mission XP needs chain authoring; ASP grant on level-up pending |
| Crafting | KM | 9 (2 IM, 7 KM) | **Phase 1 only** (#427): `CraftingState` + transactional persistence + expertise grants. Every player-facing verb (craft / research / reverse-engineer / alloy / ASP-spend / respec) still logs `UNIMPLEMENTED` |
| Stargate travel | IM | 10 (2 CW, 5 IM, 3 KM) | Gate passage CW. Multi-player sync + return-trip state + cooldown pending |
| Chat | NT | 10 (1 NT, 2 IM, 7 KM) | Say/emote/yell broadcast. All 8 canonical channels are registered and auto-joined, but nothing routes traffic on the non-spatial ones; DND flag wired, tells and moderation unported |
| Trading | IM | 8 (all IM) | **Ported 2026-06** (#438, closes #54): full propose → lock → confirm → atomic item+cash swap, with disconnect unwind and live-DB commit guards. Needs a two-client smoke to reach CW |
| Ring transport | IM | 7 (all IM) | **Newly tracked.** 2,791 lines — cross-region and cross-world transporter rings with a multi-second Kismet-driven FSM |
| Contact lists | CW | 10 (all CW) | **Shipped 2026-06-20**, owner-confirmed working (#572/#574/#578/#579/#581/#583). 2,851 lines: schema, list CRUD, member add/remove, and presence fanout for LoggedInStatus / GainLevel / Death / GateTravel. `eventId` is a bitfield (LoggedInStatus = 1) |

### Stub-Only / Largely Missing

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Organizations / guilds | KM | 15 (all KM) | 200 lines of stubs in cell_methods/organization.rs. DB schema needed |
| Mail | IM | 13 (8 IM, 4 KM, 1 NU) | 939 lines in base/world_entry/methods/mail/ plus the cell-side handlers. CoD + return-to-sender + new-mail fanout still pending — no match for any of the three in `crates/` |
| Black market | KM | 10 (9 KM, 1 NU) | Still 94 lines of stubs **on `main`**. A full Phase 1 (sgw_auction schema, create/bid/cancel FSM, expiry sweep, search) exists on the unmerged `feat/571-black-market-phase1` branch and is not counted here |
| Dueling | KM | 6 (all KM) | Not ported. 5-state machine + 7 defeat conditions to implement |
| Pets | KM | 7 (all KM) | Not ported. Entity extends spawner mob + Follow AI state |
| Minigames | IM | 9 (6 IM, 3 KM) | **SmartFox server is in-process, not external** — 2,262 lines incl. the SFS codec and a 250 ms tick loop. Livewire fully implemented; six games run on an accept-anything placeholder; Alignment + GoauldCrystals are open TODOs |
| Groups / parties | KM | 7 (all KM) | Not ported. `game/src/social/groups.rs` is a 97-line struct with zero call sites |

### Systems New Since the Original Audit

These didn't exist in the Python codebase and so weren't tracked. They're substantial in Rust today.

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Content engine | CW | 10 (6 CW, 4 KM) | 7,906 lines in cell/content/ + 3,561 in content-engine/. 99 tests. Drives missions/dialogs/triggers/conditions/actions |
| Mercury bundle | CW | 5 (5 CW) | ChannelBundle accumulator, AoI-burst bundling, backpressure handling |
| Observability pipeline | CW | 8 (8 CW) | OTLP exporter, SigNoz overlay, Mercury packet logging, dev-session telemetry, negative-logging convention |
| Wireclient + chaos | IM | 7 (3 CW, 3 IM, 1 KM) | **Corrected — the previous "7 CW / Tier 3 replay" claim was wrong.** `crates/wireclient` has no UDP socket, no `connect()`, and no replay engine; the socket loop is deferred to an unbuilt "Phase 1.5". What works: the SOAP auth leg, phase-3 handshake byte builders/parsers, and a JSONL trace loader with a diff policy (~30 tests). The 3 CW rows are the Mercury-side LossyTransport, loopback harness, and chaos scenarios, which are real |
| Discord notifications | CW | 6 (6 CW) | Event routing, channel toggles, embed formatting, panic-hook capture |
| Tauri admin app + tools | IM | 9 (1 CW, 7 IM, 1 KM) | Admin API, content editor, scene editor, sgw-launcher. Three.js space viewer pending |

### Server Infrastructure (Cross-Cutting)

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Session management | IM | 6 (1 CW, 2 IM, 3 KM) | Inactivity timeout works. No reconnection grace or continuous validation |
| Rate limiting | KM | 5 (1 CW, 4 KM) | Ability cooldowns enforced. Chat / action / trade / login throttling pending |
| Anti-cheat | IM | 7 (1 CW, 5 IM, 1 KM) | **Four-layer movement validation shipped** (#437/#478): bounds/NaN/Z-clip, speed (warn-only pending calibration), teleport (hard reject + snap-back), navmesh containment. Ability range enforced server-side. Remaining gap: no max-damage cap |
| Economy | IM | 7 (5 CW, 2 KM) | Vendor/loot/mission cash all flowing. AH listing fees + cash-flow tracking pending |
| World state | IM | 6 (2 CW, 1 IM, 3 KM) | Outbox CW. Gate/door state + world-state table pending |
| Scheduler | IM | 4 (1 IM, 3 KM) | Per-chain timers via content engine. No global cron |
| Admin / GM | IM | 13 (4 CW, 5 IM, 4 KM) | **Teleport and item-grant shipped** via the client's native `/` console — the SGWGmPlayer class flip (#473, merged in #518 on 2026-06-17) makes a GM enter the world as entity class `0x03`. 5,104 lines of GM handlers plus a 4,370-line dev/authoring `.`-console (#523). Access-level gate enforced server-side. Owner-confirmed working 2026-06-20. **Ban/mute is still genuinely missing** |
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
| Crafting blueprints | 499 | 0 | Blueprint ids persist per player, but no crafting verb consumes them yet |
| Loot tables | defined | mostly empty | Algorithm verified; content sparse |

## Known Issues

### AoI entity-introduction drop (open)

A witness can miss an entity introduction entirely — the reproducible case is a Castle Cellblock GuardBody corpse that stays invisible until the player relogs. The 2026-06-20 colo repro **disproved** the address-gate hypothesis (the expected warnings never fired), which puts the fault downstream in create + appearance delivery. PR #582 added `aoi.create_emit` / `aoi.create_send_failed` seams to localise it on the next repro. This is why Entity Lifecycle (AoI) is no longer CW.

### Combat formula calibration

Combat works at a basic level but several formulas are still calibration items:

- No diminishing returns on stats (NU)
- Armor / resistance calibration vs. original is approximate
- AoE damage falloff curves need verification (PR #420 landed AoE framework)
- Line-of-sight is enforced on the NPC firing path but **not** on player `useAbility` (which checks range only)

### Mercury protocol gaps

The transport layer works; the remaining BigWorld gaps are narrower than they were:

- Cumulative ACKs are now implemented (they drain the TX window and the unsent queue in one pass)
- No piggyback ACKs
- Reconnection grace period missing (instant disconnect = lost session)
- Mercury v2 encryption ships but no client speaks it — it is back-compatible and opt-in, and **has never been exercised against a live client**

### Effect content gap

The framework is CW (PR #420). The long tail of the 3,217 effect rows still needs script coverage — the most common scripts are wired; the niche ones are not. `cell/effects/scripts.rs` has grown to 1,648 lines.

### Crafting half-ported

Phase 1 (#427) landed the state layer: disciplines, blueprints, applied-science points, and racial paradigm levels persist transactionally, and expertise can be granted. Every player-facing crafting verb still logs `UNIMPLEMENTED`. Trading, previously listed alongside crafting here, was ported in #438.

### Seeded cinematic data never reaches the client

`sequences_nvp` seeds 2,042 NameValuePair rows (sound-bank names and similar), but no Rust code reads the table — all six `onSequence` emit sites hardcode a NameValuePairs count of 0. Cinematics fire, but without their authored parameters.

### Single-zone coverage

Only Castle Cellblock is routinely smoked end-to-end. The other 23 spaces have content but no continuous verification.

### Large June/July landings await client verification

Trading, ring transport, the cover system, movement validation, and the in-process minigame server all shipped with unit, live-DB, and wire-format coverage but no live-client run. They account for most of the 134 IM features.

## Critical Path for Playability

Re-ranked 2026-07-25. NPC navigation states and the trading port are **done** and have left this list.

1. **Effect-script content coverage** — framework CW (#420); the 3,217 effect rows need script authoring for the long tail
2. **AoI entity-introduction drop** — needs a repro against the #582 instrumentation, not more code
3. **Mission XP** — `mission.reward_xp` is 0 in all seed rows; chain-side authoring + a `GrantXP` executor arm
4. **Crafting Phase 2** — the crafting verbs on top of the Phase 1 state layer
5. **Multi-zone end-to-end** — 23 spaces unchecked
6. **Client verification of the June/July landings** — trading, ring transport, cover, movement validation, minigames

Quality-of-life items (organizations, mail polish, black market merge, dueling, pets, remaining minigame ports, groups) follow the above and can be picked up independently. Contact lists and GM tooling have shipped.

## Roadmap

### Near-term — close critical-path gaps

- Effect-script coverage for the most-played encounters
- Reproduce and fix the AoI entity-introduction drop against the #582 seams
- Mission XP chain-authoring + a `GrantXP` executor arm
- Multi-zone routine smoke
- Client smokes for the June/July landings (trading, ring transport, cover, movement validation)

### Medium-term — restore retired subsystems

- Crafting Phase 2 (the verbs, on top of the shipped state layer)
- Org / guild lifecycle + schema
- Mail polish (CoD, return-to-sender, new-mail fanout)
- Merge `feat/571-black-market-phase1`

### Long-term — finish-out

- Dueling + pets + groups + the remaining minigame ports (in any order)
- Server infrastructure: rate limiting, damage sanity checking, promoting speed validation from warn-only to enforcing, reconnection grace
- Ban/mute on top of the shipped GM command surface
- Mercury v2 verification against a patched client
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
