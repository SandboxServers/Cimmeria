---
title: "Project Status"
type: reference
audience: anyone tracking the project
last_updated: 2026-08-02
---

# Project Status

Where the Cimmeria server emulator stands today and what's ahead.

> **Status reset — 2026-08-02.** Every status in this document has been cleared pending a ground-up **human re-verification campaign** using the live game client. Previous editions assigned statuses from code audits and partial client smokes; this edition records nothing as working until a human has verified it at the client per the [Project Status Validation Plan](project-status-validation-plan.md).
>
> The previous (2026-07-25) edition, with its code-audit-derived statuses, is preserved in git history. The per-feature [Gap Analysis](gap-analysis.md) still carries the 2026-07-25 code-audit statuses as a *reference for what the code claims to do* — it is **not** re-verified either, and its statuses will be updated as validation sessions complete.
>
> **Scope note**: only work merged to `main` is counted. The black-market implementation on `feat/571-black-market-phase1` is real but unmerged, and is counted as missing until it lands.

## Status Taxonomy

All rows currently carry **UV**. The other statuses are the verdicts a validation session can assign.

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Unverified** | UV | Status cleared 2026-08-02; awaiting a human validation session per the [validation plan](project-status-validation-plan.md) |
| **Confirmed Working** | CW | A human tested it end-to-end with the game client during the current campaign and verified correct behavior, with evidence recorded |
| **Needs Test** | NT | Code exists and a validation procedure is defined, but the session hasn't run yet |
| **Implemented** | IM | Validation session ran; feature partially works or has recorded defects |
| **Known / Missing** | KM | Validation session confirmed the feature is absent (or code exists but is inert from the client's perspective) |
| **Needed / Unknown** | NU | Server-only system we infer must exist but cannot observe from the client; verified by other means noted in the plan |

## Overall Completion

| Status | Features | Percentage |
|--------|----------|-----------|
| Unverified (UV) | 443 | 100% |
| Confirmed Working (CW) | 0 | 0% |
| Needs Test (NT) | 0 | 0% |
| Implemented (IM) | 0 | 0% |
| Known/Missing (KM) | 0 | 0% |
| Needed/Unknown (NU) | 0 | 0% |
| **Total** | **443** | |

**Verified with the client this campaign**: 0 of 443 features (0%).

The feature inventory (443 features across 45 systems — 37 gameplay + 8 infrastructure) is unchanged from the [Gap Analysis](gap-analysis.md); only the verdicts have been cleared. As validation sessions complete, this table and the per-system tables below get repopulated with evidence-backed statuses.

## System Status

The **Plan** column links each system to its validation procedure in the [Project Status Validation Plan](project-status-validation-plan.md). Notes describe the code footprint on `main` (factual, from the 2026-07-25 audit) — not a claim that any of it works.

### Infrastructure

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| Authentication & login | UV | 12 | [P0.1](project-status-validation-plan.md#p01--authentication--login) | `crates/services/src/auth/` — SOAP login, shard key exchange, TLS listener + cert hot-reload (#566/#577), login audit |
| Mercury protocol | UV | 15 | [P0.2](project-status-validation-plan.md#p02--mercury-protocol) | v1 AES-256-CBC + HMAC-MD5 default; v2 opt-in (never exercised against a client); cumulative ACKs; no piggyback ACKs |
| Game data pipeline | UV | 7 | [P0.3](project-status-validation-plan.md#p03--game-data-pipeline) | 22 resource categories, 112,626 DB rows, PAK overrides for missions and items |
| Database persistence | UV | 8 | [P0.4](project-status-validation-plan.md#p04--database-persistence) | sqlx with compile-time checks, durable Base→Cell outbox, 224 live-DB regression guards |

### Core Gameplay

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| Character creation | UV | 11 | [P1.1](project-status-validation-plan.md#p11--character-creation) | 1,640 lines incl. delete + visuals; SGWGmPlayer class flip (#473/#518) |
| World entry & spaces | UV | 9 | [P1.2](project-status-validation-plan.md#p12--world-entry--spaces) | 22,682 lines across 64 files |
| Movement & navigation | UV | 9 | [P1.3](project-status-validation-plan.md#p13--movement--navigation) | Detour wired: pathfinding, LOS raycast, navmesh containment; four-layer movement validation (#437/#478) |
| Entity lifecycle (AoI) | UV | 9 | [P2.1](project-status-validation-plan.md#p21--entity-lifecycle-aoi) | Grid-based AoI + witness lifecycle; known-open entity-introduction drop, #582 instrumentation awaiting repro |
| Combat & abilities | UV | 23 | [P1.4](project-status-validation-plan.md#p14--combat--abilities) | 5,918 lines + 142 tests; PR #420 ability+effect work; LOS enforced NPC-side only |
| Effects & buffs | UV | 13 | [P1.5](project-status-validation-plan.md#p15--effects--buffs) | Effect framework + common scripts; 3,217 effect rows in DB |
| Stats | UV | 8 | [P1.6](project-status-validation-plan.md#p16--stats) | Stat list + dirty sync + per-level scaling; equipment bonuses pending |
| Inventory & items | UV | 13 | [P1.7](project-status-validation-plan.md#p17--inventory--items) | 5,272-line dispatcher with stacking (#405), bandolier discipline, Slappack PAK override |
| Missions | UV | 12 | [P1.8](project-status-validation-plan.md#p18--missions) | Content-engine driven; sharing + mission-gated loot absent |
| Loot | UV | 9 | [P1.9](project-status-validation-plan.md#p19--loot) | Take-all + bag drop + per-item distance re-validation (#446); tables mostly empty; no per-player eligibility list |
| Vendors | UV | 8 | [P1.10](project-status-validation-plan.md#p110--vendors) | 7,267 lines across buyback / purchase / sell / paid_repair / paid_recharge |

### NPC Systems

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| NPC AI & behavior | UV | 22 | [P1.12](project-status-validation-plan.md#p112--npc-ai--behavior) | Patrol / wander / investigate / follow / leash / despawn (#428); cover system (#429, 4,095 lines); no hearing radius, mob groups, kill-credit tapping |
| Spawn system | UV | 23 | [P1.13](project-status-validation-plan.md#p113--spawn-system) | 2,448 lines; time-of-day, detection radius, linked sets absent |

### Secondary Systems

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| XP & leveling | UV | 11 | [P1.11](project-status-validation-plan.md#p111--xp--leveling) | Kill-XP pipeline + level scaling + training points; `mission.reward_xp` is 0 in all seed rows |
| Crafting | UV | 9 | [P1.14](project-status-validation-plan.md#p114--crafting) | Phase 1 state layer only (#427); every player-facing verb logs `UNIMPLEMENTED` |
| Stargate travel | UV | 10 | [P1.15](project-status-validation-plan.md#p115--stargate-travel) | Gate passage code; multi-player sync + return-trip state + cooldown absent |
| Chat | UV | 10 | [P1.16](project-status-validation-plan.md#p116--chat) | Say/emote/yell broadcast; 8 channels registered/auto-joined but non-spatial ones route nothing; tells unported |
| Trading | UV | 8 | [P2.2](project-status-validation-plan.md#p22--trading) | Ported 2026-06 (#438): propose → lock → confirm → atomic swap with disconnect unwind |
| Ring transport | UV | 7 | [P1.17](project-status-validation-plan.md#p117--ring-transport) | 2,791 lines — cross-region and cross-world rings, Kismet-driven FSM |
| Contact lists | UV | 10 | [P1.18](project-status-validation-plan.md#p118--contact-lists) / [P2.4](project-status-validation-plan.md#p24--contact-presence-fanout) | 2,851 lines: schema, list CRUD, presence fanout (#572–#583) |

### Expected-Missing Systems

The 2026-07-25 audit found these stub-only or absent. The campaign confirms that from the client rather than assuming it.

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| Organizations / guilds | UV | 15 | [P3.1](project-status-validation-plan.md#p31--organizations--guilds) | 200 lines of stubs; no DB schema |
| Mail | UV | 13 | [P1.19](project-status-validation-plan.md#p119--mail) / [P2.8](project-status-validation-plan.md#p28--mail-between-players) | 939 lines base-side + cell handlers; CoD, return-to-sender, new-mail fanout absent |
| Black market | UV | 10 | [P3.2](project-status-validation-plan.md#p32--black-market) | 94 lines of stubs on `main`; full Phase 1 unmerged on `feat/571-black-market-phase1` |
| Dueling | UV | 6 | [P2.6](project-status-validation-plan.md#p26--dueling) | Not ported |
| Pets | UV | 7 | [P3.3](project-status-validation-plan.md#p33--pets) | Not ported |
| Minigames | UV | 9 | [P1.20](project-status-validation-plan.md#p120--minigames) | In-process SmartFox server (2,262 lines); Livewire implemented; six games accept-anything; Alignment + GoauldCrystals TODO |
| Groups / parties | UV | 7 | [P2.7](project-status-validation-plan.md#p27--groups--parties) | 97-line struct, zero call sites |

### Systems New Since the Original Audit

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| Content engine | UV | 10 | [P1.21](project-status-validation-plan.md#p121--content-engine) | 7,906 lines in cell/content/ + 3,561 in content-engine/; 99 tests |
| Mercury bundle | UV | 5 | [P5.1](project-status-validation-plan.md#p51--mercury-bundle) | ChannelBundle accumulator, AoI-burst bundling, backpressure |
| Observability pipeline | UV | 8 | [P0.6](project-status-validation-plan.md#p06--observability-metrics-and-discord) | OTLP exporter, SigNoz overlay, packet logging, dev-session telemetry |
| Wireclient + chaos | UV | 7 | [P5.2](project-status-validation-plan.md#p52--wireclient--chaos-harness) | SOAP auth leg + handshake byte builders + JSONL trace loader; no UDP socket loop; Mercury-side LossyTransport + loopback harness + chaos scenarios |
| Discord notifications | UV | 6 | [P0.6](project-status-validation-plan.md#p06--observability-metrics-and-discord) | Event routing, channel toggles, embeds, panic-hook capture |
| Tauri admin app + tools | UV | 9 | [P5.3](project-status-validation-plan.md#p53--tauri-admin-app--tools) | Admin API, content editor, scene editor, sgw-launcher |

### Server Infrastructure (Cross-Cutting)

| System | Status | Features | Plan | Code footprint (unverified) |
|--------|--------|----------|------|-----------------------------|
| Session management | UV | 6 | [P0.5](project-status-validation-plan.md#p05--session-management) | Inactivity timeout; no reconnection grace or continuous validation |
| Rate limiting | UV | 5 | [P2.9](project-status-validation-plan.md#p29--anti-cheat--rate-limiting) | Ability cooldowns only; chat / action / trade / login throttling absent |
| Anti-cheat | UV | 7 | [P2.9](project-status-validation-plan.md#p29--anti-cheat--rate-limiting) | Four-layer movement validation (#437/#478); server-side ability range; no max-damage cap |
| Economy | UV | 7 | [P1.22](project-status-validation-plan.md#p122--economy) | Vendor/loot/mission cash paths; AH listing fees + cash-flow tracking absent |
| World state | UV | 6 | [P5.4](project-status-validation-plan.md#p54--world-state--scheduler) | Outbox; gate/door state + world-state table absent |
| Scheduler | UV | 4 | [P5.4](project-status-validation-plan.md#p54--world-state--scheduler) | Per-chain timers via content engine; no global cron |
| Admin / GM | UV | 13 | [P1.23](project-status-validation-plan.md#p123--admin--gm-commands) | 5,104 lines of GM handlers + 4,370-line dev `.`-console (#523); server-side access-level gate; ban/mute absent |
| Metrics / telemetry | UV | 7 | [P0.6](project-status-validation-plan.md#p06--observability-metrics-and-discord) | Full OTLP pipeline |

## Content Coverage

Content-coverage claims are cleared along with system statuses. The DB-side totals are facts; the "verified" column restarts at zero.

| Content Type | Total in DB | Verified this campaign | Plan |
|--------------|-------------|------------------------|------|
| Zones | 91 world definitions | 0 | [P4](project-status-validation-plan.md#phase-4--multi-zone-sweep) |
| Missions | 1,041 | 0 | [P1.8](project-status-validation-plan.md#p18--missions) |
| Abilities | 1,887 | 0 | [P1.4](project-status-validation-plan.md#p14--combat--abilities) |
| Items | 6,060 | 0 | [P1.7](project-status-validation-plan.md#p17--inventory--items) |
| Effects | 3,217 | 0 | [P1.5](project-status-validation-plan.md#p15--effects--buffs) |
| NPCs | 153 templates | 0 | [P1.12](project-status-validation-plan.md#p112--npc-ai--behavior) |
| Dialog trees | 5,406 | 0 | [P1.21](project-status-validation-plan.md#p121--content-engine) |
| Stargates | 29 | 0 | [P1.15](project-status-validation-plan.md#p115--stargate-travel) |
| Crafting blueprints | 499 | 0 | [P1.14](project-status-validation-plan.md#p114--crafting) |
| Loot tables | defined (mostly empty) | 0 | [P1.9](project-status-validation-plan.md#p19--loot) |

## Observations Carried Forward

These findings from the 2026-07-25 edition are kept as **candidate repro cases and watch items** for the campaign — they are inputs to test sessions, not current status claims. Each has a re-check step in the validation plan.

- **AoI entity-introduction drop** — a witness can miss an entity introduction entirely (reproducible case: a Castle Cellblock GuardBody corpse invisible until relog). #582 added `aoi.create_emit` / `aoi.create_send_failed` seams. Re-check in [P2.1](project-status-validation-plan.md#p21--entity-lifecycle-aoi).
- **Combat formula calibration** — no diminishing returns on stats; armor/resistance approximate; AoE falloff unverified; LOS enforced on the NPC firing path but not on player `useAbility`. Re-check in [P1.4](project-status-validation-plan.md#p14--combat--abilities).
- **Mercury gaps** — no piggyback ACKs; no reconnection grace; v2 encryption never exercised against a live client. Re-check in [P0.2](project-status-validation-plan.md#p02--mercury-protocol) / [P0.5](project-status-validation-plan.md#p05--session-management).
- **Effect content gap** — the long tail of 3,217 effect rows lacks script coverage. Sampled in [P1.5](project-status-validation-plan.md#p15--effects--buffs).
- **Crafting half-ported** — Phase 1 state layer only; every player-facing verb logs `UNIMPLEMENTED`. Confirmed from the client in [P1.14](project-status-validation-plan.md#p114--crafting).
- **Seeded cinematic data never reaches the client** — `sequences_nvp` seeds 2,042 rows but all six `onSequence` emit sites hardcode a NameValuePairs count of 0. Watch item in [P1.21](project-status-validation-plan.md#p121--content-engine).
- **Mission XP** — `mission.reward_xp` is 0 in all seed rows. Re-check in [P1.11](project-status-validation-plan.md#p111--xp--leveling).
- **Single-zone coverage** — only Castle Cellblock was ever routinely smoked; 23 other populated spaces unchecked. Addressed by [Phase 4](project-status-validation-plan.md#phase-4--multi-zone-sweep).

## Critical Path

The pre-reset critical path and roadmap are suspended until the campaign produces evidence-backed statuses. The current critical path **is the campaign itself**:

1. **Phase 0** — infrastructure smoke (login through world entry, observability capture working)
2. **Phase 1** — single-client core-loop verification in Castle Cellblock
3. **Phase 2** — two-client verification (AoI, trading, chat, presence, anti-cheat)
4. **Phase 3** — expected-missing confirmation sweep
5. **Phase 4** — multi-zone sweep across the other populated spaces
6. **Phase 5** — server-side / harness systems not observable from the client

Once the tables above are repopulated, the roadmap gets re-derived from what the campaign actually found. The 2026-07-25 roadmap is in git history for comparison.

## Related Documents

- [Project Status Validation Plan](project-status-validation-plan.md) — **the campaign playbook**: environment setup, per-system client test procedures, verdict rules, evidence requirements
- [Gap Analysis](gap-analysis.md) — per-feature inventory (source of truth for the 443-feature list; statuses there are the last code audit, pending the same re-verification)
- [Gameplay Dashboard](gameplay/README.md) — per-system gameplay breakdowns
- [Content Engine](content/content-engine.md) — the data-driven runtime
- [Known Issues](known-issues.md) — catalogue of known bugs
- [Multiplayer / LAN Setup](multiplayer.md) — two-client environment for Phase 2
- [../README.md](../README.md) — high-level project status
