---
title: "Project Status"
type: reference
audience: anyone tracking the project
last_updated: 2026-09-25
---

# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

> This document summarizes the findings of the [Gap Analysis](gap-analysis.md), which tracks **471 individual features across 45 systems** (37 gameplay + 8 infrastructure) against the active Rust codebase on `main`.
>
> **Re-verified 2026-09-25** against the code at `acbcc22e`, after about 160 PRs landed since the previous (2026-07-25) edition. Every row was re-read, and a feature counts as Confirmed Working only when there is a written record of an in-client test: the 2026-09-18 colo playtest, the 2026-09-25 NPC AI UAT, a PR or issue note, or a recorded confirmation. That stricter bar moved some rows down (vendors, spawn population, mission cash, mail sending, effect clear-flags) while the playtest moved others up (character creation, minigames, ring trips, damage, loot). The previous edition's headline also did not match its own table: it printed 443 / CW 159 / KM 128 against rows that summed to 444 / CW 164 / KM 124. The figures below are recomputed from the rows.
>
> **Scope note**: only work merged to `main` is counted. The black-market implementation on `feat/571-black-market-phase1` (PR #586) is real but unmerged, and is counted as missing until it lands.

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | CW | Tested end-to-end with the game client and verified correct, with a written record of the test |
| **Needs Test** | NT | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | IM | Code written but may be incomplete or have known issues |
| **Known / Missing** | KM | We know this needs to exist but no code exists in `crates/` |
| **Needed / Unknown** | NU | Server-only system we infer must exist but have no direct evidence for |

## Overall Completion

| Status | Features | Percentage |
|--------|----------|-----------|
| Confirmed Working (CW) | 169 | 35.9% |
| Needs Test (NT) | 58 | 12.3% |
| Implemented (IM) | 98 | 20.8% |
| Known/Missing (KM) | 142 | 30.1% |
| Needed/Unknown (NU) | 4 | 0.8% |
| **Total** | **471** | |

**Code exists (CW + NT + IM)**: 325 features (69.0%)  
**Missing (KM + NU)**: 146 features (31.0%)  
**Tested end-to-end (CW)**: 169 features (35.9%)

The story of this quarter is the Needs Test column, which tripled from 18 to 58. Castle Cellblock and Castle were rebuilt and played end to end in the 2026-09-18 colo playtest, Harset was rebuilt, and the NPC AI restoration campaign replaced aggro, leash, cover and line of sight. Most of that merged with unit, live-DB and wire-format coverage but has not yet been run in a client. A tester working through the NT rows is now the fastest way to move the headline.

## System Status

### Infrastructure — Solid

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Authentication & login | CW | 13 (8 CW, 1 NT, 2 IM, 2 KM) | Full login flow tested. TLS listener + cert hot-reload (#566/#577). Server-side passwords are argon2id with opportunistic migration; the stock client still sends SHA-1 and cannot move off it without a patch. Duplicate login evicts the older session. Continuous validation pending |
| Mercury protocol | CW | 15 (10 CW, 3 IM, 2 KM) | v1 AES-256-CBC + HMAC-MD5 is the client-compatible default. **v2 shipped** (per-packet IV, HKDF-split keys, truncated HMAC-SHA256, downgrade defense, key rotation) but is opt-in and **untested against a live client**. Cumulative ACKs implemented; piggyback ACKs still missing |
| Game data pipeline | CW | 9 (6 CW, 2 NT, 1 KM) | 21 wire categories (client 1–21), 112,626 DB rows, PAK overrides for missions and items. New: per-key Kismet sequence overrides (#755) and dialog override patch mode (#767). Hot reload pending |
| Database persistence | CW | 8 (6 CW, 2 KM) | sqlx 0.9 with runtime-checked queries (there are no compile-time `query!` macros), durable Base→Cell outbox, 775 live-DB regression guards. No migration framework yet |

### Core Gameplay — Real Code, Mostly Working

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Character creation | CW | 11 (4 CW, 4 NT, 1 IM, 2 KM) | **Promoted.** Two characters (Human Soldier, Jaffa) created and played in the 2026-09-18 colo playtest: list, preview, archetype and starting equipment are CW. Delete and visuals remain NT |
| World entry & spaces | CW | 10 (7 CW, 2 NT, 1 IM) | About 32,500 lines across 95 files. Castle Cellblock and Castle end-to-end. Same-world respawn resync added (#756). Open: fresh clients logging straight into Castle or SGC_W1 hang (recorded in #756, no issue filed yet) |
| Movement & navigation | IM | 11 (1 CW, 3 NT, 7 IM) | **Every world now has a navmesh** (#794), with tiled meshes for the large exteriors (#796) and per-world containment modes. Four-layer movement validation (#437/#478). Open: the speed check divides by per-packet time and can produce Inf; Castle's navmesh does not connect its interior to its exterior |
| Entity lifecycle (AoI) | IM | 10 (6 CW, 2 NT, 1 IM, 1 KM) | Grid-based AoI and witness lifecycle work, but an entity a witness was correctly introduced to can still fail to render (invisible corpse until relog). The first-login cinematic hold (#747) is the experiment on it and **has not been observed in game**. Player-to-player introduction (#737) is implemented and **awaiting two-client validation** — see [architecture/player-ghost-aoi-cascade.md](architecture/player-ghost-aoi-cascade.md) |
| Combat & abilities | IM | 24 (6 CW, 14 IM, 4 KM) | About 11,400 production lines and 194 tests. Damage application is CW (26 kills and 19 player deaths in the colo playtest). Line of sight is enforced on the NPC side but **not** on player `useAbility`; no facing check, min range, prerequisite monikers or threat decay. Two #673 divergences (`EF_DONT_USE_QR` value, damage-type wire values) are still open |
| Effects & buffs | IM | 13 (3 CW, 5 IM, 5 KM) | Framework works. The clear-on-damage, clear-on-revive and clear-on-bandolier-swap flags do not exist; permanent vs non-permanent stat tracking does not exist. Long tail of 3,216 effect rows needs script coverage |
| Stats | IM | 8 (5 CW, 2 KM, 1 NU) | Stat list + dirty sync + per-level scaling shipped. Equipment bonuses + derived formulas pending |
| Inventory & items | IM | 13 (8 CW, 3 NT, 1 IM, 1 KM) | About 5,600-line dispatcher with stacking (#405), bandolier discipline, Slappack PAK override. Item binding is respected by trade, sell and stack |
| Missions | IM | 12 (7 CW, 3 IM, 2 KM) | Content-engine driven. About 30 missions have content chains and about 22 were played in the client (Castle Cellblock, Castle 701-706). Completion works, but **missions pay no cash or items** (#310) and mission XP is 0 in the data. #657 (objectives lost on relog) is fixed on `main` (#682) and awaiting a relog test |
| Loot | IM | 9 (4 CW, 1 NT, 4 KM) | Loot generation, take-all and bag drop verified in the colo playtest; looter distance re-validated per item (#446). Tables mostly empty; there is **no** per-player eligibility list in Rust |
| Vendors | NT | 8 (1 CW, 6 NT, 1 IM) | About 7,450 lines across buyback / purchase / sell / paid_repair / paid_recharge. #609 found that the vendor window had been routed to the mission handler, so earlier vendor testing was void, and no client has tested the store since. **No world currently spawns a vendor** (`.spawn 25` is the only way to reach one) |

### NPC Systems — Partial

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| NPC AI & behavior | IM | 26 (8 CW, 6 NT, 9 IM, 3 KM) | **NPC AI restoration campaign complete** (NA00–NA33, PRs #774–#797): faction proximity aggro, assist aggro (CW in the 2026-09-25 UAT), a leash rewrite, grounding and stop hygiene, world-space cover from the client maps (236 nodes in Cellblock, 3,788 in Castle) and line of sight over per-world collision geometry. Everything merged after UAT-1 is unseen in a client. Remaining gaps: hearing radius, mob groups, kill-credit tapping |
| Spawn system | IM | 23 (7 CW, 1 IM, 14 KM, 1 NU) | **Corrected down.** Castle Cellblock and Castle lifecycles and 120 s respawn timers are CW. SpawnRegion/SpawnSet activation, population tracking, set cooldowns, weighted spawn tables and level ranges had been credited to `spawner/regions.rs`, which loads a different kind of region (client-hinted trigger regions); they are not implemented (#62) |

### Secondary Systems

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| XP & leveling | IM | 11 (9 CW, 1 IM, 1 KM) | Kill-XP pipeline + level scaling + training points CW. Mission XP has a `GrantXP` action (#618) but `reward_xp` is 0 on all 1,041 missions and the formula needs a maintainer decision |
| Crafting | KM | 9 (2 IM, 7 KM) | **Phase 1 only** (#427): `CraftingState` + transactional persistence + expertise grants. Every player-facing verb (craft / research / reverse-engineer / alloy / ASP-spend / respec) still logs `UNIMPLEMENTED` |
| Stargate travel | IM | 10 (2 CW, 4 NT, 3 IM, 1 KM) | Gate passage CW. DHD interaction, gate cancel, address discovery and multi-player gate sync are NT (#662, #663, #682); stargate open/cross events are now emitted. Return-trip state is IM |
| Chat | NT | 10 (1 NT, 2 IM, 7 KM) | Say/emote/yell broadcast, now reaching other players via #737. All 8 canonical channels are registered and auto-joined, but nothing routes traffic on the non-spatial ones; tells and moderation unported |
| Trading | IM | 8 (all IM) | **Ported 2026-06** (#438): full propose → lock → confirm → atomic item+cash swap, with disconnect unwind and live-DB commit guards. Needs a two-client smoke to reach CW |
| Ring transport | IM | 9 (3 CW, 4 NT, 2 IM) | About 5,850 lines. **Four in-client Cellblock ring trips** in the colo playtest make region loading, the destination list and the state machine CW. Stall timeouts and the mission 688 client-patch ceremony are new rows; the patched map passed its Phase 0 in-client check (2026-09-19) and Phase 1 awaits a test |
| Contact lists | CW | 10 (all CW) | **Shipped 2026-06-20**, confirmed working in-client (#572/#574/#578/#579/#581/#583). Schema, list CRUD, member add/remove, and presence fanout for LoggedInStatus / GainLevel / Death / GateTravel. `eventId` is a bitfield (LoggedInStatus = 1) |

### Stub-Only / Largely Missing

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Organizations / guilds | KM | 15 (all KM) | 200 lines of stubs in cell_methods/organization.rs. DB schema needed |
| Mail | IM | 13 (2 NT, 2 IM, 8 KM, 1 NU) | **Corrected down.** The read side works (list, read body, delete). Sending, attaching items or cash, taking attachments, return-to-sender and COD are stubs that log "unimplemented". Archiving sets a flag the inbox query ignores |
| Black market | KM | 10 (9 KM, 1 NU) | Still 94 lines of stubs **on `main`**. A full Phase 1 is on the unmerged `feat/571-black-market-phase1` (PR #586), and the client window additionally needs a client patch (#587) |
| Dueling | KM | 6 (all KM) | Not ported. 5-state machine + 7 defeat conditions to implement |
| Pets | KM | 7 (all KM) | Not ported. Entity extends spawner mob + Follow AI state |
| Minigames | IM | 9 (5 CW, 1 IM, 3 KM) | **Livewire is client-verified**: 12 in-client sessions in the colo playtest, 11 wins that fired their follow-on chains. The SmartFox server is in-process (about 3,400 lines). Six games still run on an accept-anything placeholder; Alignment and GoauldCrystals are open TODOs |
| Groups / parties | KM | 7 (all KM) | Not ported. No group code exists (an unwired `game/src/social/groups.rs` sketch was deleted in #699) |

### Systems New Since the Original Audit

These didn't exist in the Python codebase and so weren't tracked. They're substantial in Rust today.

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Content engine | CW | 11 (6 CW, 2 NT, 1 IM, 2 KM) | About 11,200 non-test lines in cell/content/ plus the content-engine crate, with about 960 tests. Drives missions / dialogs / triggers / conditions / actions. New: NPC barks (NT) |
| Mercury bundle | CW | 5 (5 CW) | ChannelBundle accumulator, AoI-burst bundling, backpressure handling |
| Observability pipeline | CW | 13 (10 CW, 3 NT) | OTLP exporter, SigNoz overlay, Mercury packet logging, dev-session telemetry, negative-logging convention. New and used in real sessions: the `.bug` playtest bookmark and the NPC AI telemetry and detectors. New and untested: stuck-player detectors, the NPC AI dashboard (#782), the `cimmeria-trace` log index (#792) |
| Wireclient + chaos | IM | 7 (3 CW, 3 IM, 1 KM) | `crates/wireclient` has no UDP socket, no `connect()`, and no replay engine. What works: the SOAP auth leg, phase-3 handshake builders/parsers, and a JSONL trace loader with a diff policy. The 3 CW rows are the Mercury-side LossyTransport, loopback harness and chaos scenarios |
| Discord notifications | CW | 6 (6 CW) | Event routing, channel toggles, embed formatting, panic-hook capture |
| Tauri admin app + tools | IM | 13 (2 CW, 2 NT, 6 IM, 3 KM) | Admin API, content editor, scene editor, sgw-launcher. New: the UPK patcher (CW, Phase 0 in-client 2026-09-19) and the live research lab. JWT auth for remote access and the WebSocket entity stream are still TODO stubs. Three.js space viewer pending |

### Server Infrastructure (Cross-Cutting)

| System | Status | Features | Notes |
|--------|--------|----------|-------|
| Session management | IM | 7 (4 IM, 3 KM) | Two-layer inactivity timeout (#711). Cross-IP session binding (#738) warns only. Developer mode does not skip duplicate-login eviction, despite a config comment promising multi-login. No reconnection grace or continuous validation |
| Rate limiting | KM | 6 (1 CW, 1 NT, 4 KM) | Ability cooldowns enforced; dev-session token mint quota (#740). Chat / action / trade / login throttling pending |
| Anti-cheat | IM | 7 (1 CW, 5 IM, 1 KM) | Four-layer movement validation (#437/#478): bounds/NaN/Z-clip, speed (warn-only pending calibration), teleport (hard reject + snap-back), navmesh containment. Ability range enforced server-side. Remaining gap: no max-damage cap |
| Economy | IM | 7 (4 NT, 3 KM) | **Corrected down.** Vendor-priced sinks and faucets are NT until vendors are re-tested after #609. **Mission cash rewards do not exist** (#310). AH listing fees + cash-flow tracking pending |
| World state | IM | 6 (1 CW, 1 NT, 1 IM, 3 KM) | Outbox CW. Player position on logout was never persisted until #756, which has no in-client relog test yet. Gate/door state + world-state table pending |
| Scheduler | IM | 4 (1 IM, 3 KM) | Per-chain timers via content engine. No global cron |
| Admin / GM | IM | 13 (4 CW, 1 NT, 5 IM, 3 KM) | Teleport and item-grant via the client's native `/` console (the SGWGmPlayer class flip, #518). About 6,070 lines of GM handlers plus a 12,730-line dev/authoring `.`-console with 89 commands (#523). The legacy command-parity campaign has integrated 12 of 49 packets. Access-level gate enforced server-side; GM surface confirmed working 2026-06-20. **Ban/mute is still missing** |
| Metrics / telemetry | CW | 9 (4 CW, 3 NT, 2 IM) | Full OTLP pipeline. New: the NPC AI health dashboard (#782) and disk-to-SigNoz log parity (#792) |

## Content Coverage

| Content Type | Total in DB | Tested/Verified | Notes |
|--------------|-------------|-----------------|-------|
| Zones | 91 world definitions | 2 (Castle Cellblock, Castle) played in client; Harset rebuilt, unplayed | Every world has a navmesh (#794) |
| Missions | 1,041 | About 30 with content chains, about 22 played in client | Content engine drives mission chains generically; missions pay no cash or items yet (#310) |
| Abilities | 1,887 | many | Three-bucket selection landed (#368), PR #420 closed ability gaps |
| Items | 6,060 | ~30 routinely | Slappack stacking + bandolier discipline verified |
| Effects | 3,216 | framework + most-common scripts | Long tail still needs content authoring |
| NPCs | 153 templates | Castle Cellblock and Castle populations | 2026-09-18 playtest and 2026-09-25 NPC AI UAT |
| Dialog trees | 5,406 | Castle Cellblock and Castle | Dialog UI cleanup (buttons, barks) merged 2026-09-25, awaiting a client test |
| Stargates | 29 | Castle ↔ neighbor smoke | Open/cross events emitted (#663); multi-player sync awaiting a two-observer test |
| Crafting blueprints | 499 | 0 | Blueprint ids persist per player, but no crafting verb consumes them yet |
| Loot tables | defined | mostly empty | Generation verified in the colo playtest; content sparse |

## Known Issues

### AoI entity-introduction drop (open)

A witness can be correctly introduced to an entity and still never render it — the reproducible case is a Castle Cellblock GuardBody corpse (a `class_id 0` static mesh) that stays invisible until the player relogs. Two hypotheses are retired. The 2026-06-20 colo repro **disproved** the address-gate hypothesis (the expected warnings never fired). The 2026-09-19 colo repro retired Mercury delivery: the introduction went out as reliable packets with no `aoi.create_send_failed`, and only one reliable packet was retransmitted in the session's first 75 s, so the client ACKed every create before its RTO. **The drop is inside the client, after delivery.**

That repro also found why the `aoi.create_emit` seam had never appeared in SigNoz — the OTLP `EnvFilter` did not name the target, so its DEBUG events inherited `info`. That is fixed and pinned by a unit test, so the next repro will carry the seam either way.

The one thing that differed between the failing and succeeding session was the first-login cinematic (played in full vs. dismissed after 1.5 s), which is a single observation, not a finding. On that lead, a first-login cinematic AoI hold (#747) now buffers entity introductions until the intro movie ends. It is shipped as an **experiment**, and as of 2026-09-25 nobody has recorded an in-game look since, which is why Entity Lifecycle (AoI) is still not CW. Design and the alternatives if it fails: [architecture/first-login-cinematic-aoi-hold.md](architecture/first-login-cinematic-aoi-hold.md).

### Player-to-player visibility (implemented, unvalidated)

Two players in a shared world (Castle, Harset) used to introduce each other with the NPC-shaped `createOnClient` cascade — no `BeingAppearance`, no nameplate, placeholder stats, `stateField = 0` — and a player could be introduced during its own map load, when its cell entity exists but is still blank. Both are fixed: a dedicated `SGWPlayer` ghost cascade joins the cell's live state with the base session's identity at emit time, and an `is_introducible` gate keeps a loading player out of everyone's AoI until it can be introduced properly. Test coverage is wire-format, fan-out byte and negative-log only — **nobody has stood two clients next to each other yet**, so this is `NT`. Design, known gaps and the two-client UAT checklist: [architecture/player-ghost-aoi-cascade.md](architecture/player-ghost-aoi-cascade.md). Separate from the introduction drop above.

### Missions pay nothing beyond completion

Mission completion works in the client, but no code grants cash or items on completion (#310), and `reward_xp` is 0 on every mission. The `GrantXP` content action exists (#618); the reward formula needs a maintainer decision.

### Combat formula calibration

Combat works at a basic level but several formulas are still calibration items:

- No diminishing returns on stats (NU)
- Armor / resistance calibration vs. original is approximate
- AoE damage falloff curves need verification (PR #420 landed AoE framework)
- Line-of-sight is enforced on the NPC firing path but **not** on player `useAbility` (which checks range only)
- Two #673 divergences are open: `EF_DONT_USE_QR` is 32 (the original is 16) and nothing reads it, and damage-type wire values are 0-4 where the original uses 13-18

### Mercury protocol gaps

The transport layer works; the remaining BigWorld gaps are narrower than they were:

- Cumulative ACKs are now implemented (they drain the TX window and the unsent queue in one pass)
- No piggyback ACKs
- Reconnection grace period missing (instant disconnect = lost session)
- Mercury v2 encryption ships but no client speaks it — it is back-compatible and opt-in, and **has never been exercised against a live client**

### Effect content gap

The framework works (PR #420), but the clear-on-damage, clear-on-revive and clear-on-bandolier-swap flags are not implemented, and the long tail of the 3,216 effect rows still needs script coverage. `cell/effects/scripts.rs` has grown to 1,648 lines.

### Crafting half-ported

Phase 1 (#427) landed the state layer: disciplines, blueprints, applied-science points, and racial paradigm levels persist transactionally, and expertise can be granted. Every player-facing crafting verb still logs `UNIMPLEMENTED`.

### Seeded cinematic data never reaches the client

`sequences_nvp` seeds 2,042 NameValuePair rows (sound-bank names and similar), but no Rust code reads the table — all six `onSequence` emit sites hardcode a NameValuePairs count of 0. Cinematics fire, but without their authored parameters.

### September landings await client verification

58 rows are Needs Test: the NPC AI changes merged after the 2026-09-25 UAT, gate dial/open/cross with a second observer, the dialog-UI buttons and barks, two-client chat and player visibility, relog position and objectives, and vendors (untested since #609). The per-row tester actions are in the [Gap Analysis](gap-analysis.md).

## Critical Path for Playability

Re-ranked 2026-09-25.

1. **Client-test the September landings** — the 58 NT rows above; the cheapest way to move the headline
2. **Effect-script content coverage** — the 3,216 effect rows need script authoring for the long tail, plus the missing clear-on flags
3. **AoI entity-introduction drop** — needs an in-game look at the #747 hold, not more code
4. **Mission rewards** — a reward formula for XP, and cash and item dispatch (#310)
5. **Crafting Phase 2** — the crafting verbs on top of the Phase 1 state layer
6. **Multi-zone end-to-end** — Harset's first playtest; content campaigns for the next zones
7. **Two-client verification** — trading, player-to-player introduction, chat between players

Quality-of-life items (organizations, mail sending, black market merge, dueling, pets, remaining minigame ports, groups) follow the above and can be picked up independently. Contact lists and GM tooling have shipped.

## Roadmap

### Near-term — close critical-path gaps

- Client-test the NT rows, starting with the NPC AI post-UAT changes, gate travel with two observers, and vendors
- Effect-script coverage for the most-played encounters
- Observe the AoI first-login hold in game
- Mission reward formula and cash/item dispatch
- Harset's first playtest

### Medium-term — restore retired subsystems

- Crafting Phase 2 (the verbs, on top of the shipped state layer)
- Org / guild lifecycle + schema
- Mail sending, attachments, CoD, return-to-sender, new-mail fanout
- Merge `feat/571-black-market-phase1`
- Spawn population control (SpawnRegion / SpawnSet, #62)

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
- [Server Infrastructure Proposals](architecture/server-infrastructure-proposals.md) — the five unbuilt server-only systems (session resume, rate limiting, world-state persistence, scheduler, economy instrumentation)
- [../README.md](../README.md) — high-level project status
