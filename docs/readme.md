# Cimmeria Documentation

**Cimmeria** is a server emulator for *Stargate Worlds* (SGW), the cancelled MMO built on BigWorld Technology and Unreal Engine 3 by Cheyenne Mountain Entertainment (CME). The active server is a Rust workspace under [`crates/`](../crates/); these docs cover its architecture, the wire protocol, game-system internals, the reverse-engineering work that informs both, and the operational runbooks for running it.

The emulator is **playable today**: players can log in, enter the world, interact with NPCs, run quests, and engage in combat. See the [project README](../README.md) for the high-level status snapshot.

---

## New Developer Start Here

If you've just cloned the repo, walk through these in order:

1. **[../README.md](../README.md)** — project overview and current status.
2. **[guides/getting-started.md](guides/getting-started.md)** — first-time setup tutorial: prerequisites, `setup.ps1`, verifying the server is up, connecting the client.
3. **[building.md](building.md)** — how to build and run the Rust server, including the CI checks.
4. **[../CLAUDE.md](../CLAUDE.md)** — repo invariants, build memory rules (WSL), pre-PR checklist.
5. **[../TESTING.md](../TESTING.md)** — test types, picker for which to use when, gotchas mined from PR reviews.
6. **[connection-flow.md](connection-flow.md)** — end-to-end login + world entry. Once you've followed it through, the architecture starts to make sense.
7. **[how-sgw-works.md](how-sgw-works.md)** — BigWorld, CME, and how the pieces fit together.
8. **[troubleshooting.md](troubleshooting.md)** — common first-day problems. Bookmark for when something breaks.

Want to start contributing? Read **[../CONTRIBUTING.md](../CONTRIBUTING.md)** — it covers contribution scope, where to find an approachable first issue, and the difference between content-chain work (no RE required) and protocol work (RE required).

---

## Quick Stats

| Metric | Count |
|--------|-------|
| Event_NetOut messages (client to server) | 253 |
| Event_NetIn messages (server to client) | 167 |
| Total cataloged network messages | 420 |
| Event-to-.def mapping coverage | ~98% |
| Entity types | 18 |
| Interfaces | 18 |
| Named functions in sgw.exe | 101,909 (60.6%) |
| Remaining unnamed functions | ~66,330 |
| Python game logic scripts | 164 |
| Database rows (game data) | 112,626 |
| Abilities / Items / Missions / Effects | 1,887 / 6,060 / 1,041 / 3,217 |
| Documentation files | 285 (`find docs -name '*.md' \| wc -l`) |
| Rust tests (`#[test]` / `#[tokio::test]`) | 3,012 across 473 files (2,767 gated in CI) |
| Live-DB regression guards | 247 |
| End-to-end PL/pgSQL smoke scripts | 3 |


## Document Map

### Top-Level Documents

| Document | Description |
|----------|-------------|
| [How SGW Works](how-sgw-works.md) | Technology overview -- BigWorld, CME, and how the pieces fit together |
| [Client Tools](client-tools.md) | Launcher, editor mode, debug tools available in the client |
| [Building the Server](building.md) | How to build and run the Cimmeria server emulator |
| [Container Distribution](operations/container.md) | `docker run` the published `ghcr.io/sandboxservers/cimmeria-server` image — env reference, volume layout, reset workflow |
| [Colo Auto-Update Deployment](operations/colo-deploy.md) | Self-maintaining single-host deploy: Watchtower auto-pulls `latest-prerelease`, fresh DB on every swap, single self-contained compose at [`docker/compose.yml`](../docker/compose.yml) |
| [Dev-Session Telemetry](operations/telemetry.md) | `CIMMERIA_TELEMETRY_HMAC_SECRET` provisioning + rotation, kill switch, storage layout. Design lives in [architecture/dev-session-telemetry.md](architecture/dev-session-telemetry.md). |
| [SigNoz Deployment](operations/signoz-deployment.md) | Server logs + Mercury packet shipping via OTLP to a self-hosted SigNoz (ClickHouse-backed). Local + colo bring-up, retention, troubleshooting. Design lives in [architecture/observability.md](architecture/observability.md). |
| [SigNoz Remote Access](operations/signoz-remote-access.md) | Cloudflare Tunnel + Cloudflare Access for secure UI access and Cimmeria-MCP service-token auth — no inbound firewall ports. |
| [Testing Guide](../TESTING.md) | Test types, picker for which to use when, gotchas mined from PR reviews |
| [Test Inventory](testing/inventory/README.md) | Catalogue of every test in the workspace, one file per crate, with kind / system / first-commit date / what-it-tests |
| [Test Audit 2026-05-31](testing/audit-2026-05-31.md) | Point-in-time codebase-wide audit: real bugs surfaced by the test suite, tests to delete, tests to tighten, coverage gaps, strategic recommendations |
| [Game Systems](game-systems.md) | Survey of every game feature: combat, abilities, stargates, missions, crafting |
| [Game Data](game-data.md) | What game content exists (items, abilities, missions) and what is missing |
| [Slash Commands](commands.md) | Player-friendly guide to all 266 in-game `/commands` (the real typed names captured live), with what each does, whether it works on our server yet, access level, parameters, and examples |
| [Connection Flow](connection-flow.md) | End-to-end login and world entry sequence |
| [Network Messages](network-messages.md) | High-level catalog of client-server messages |
| [Project Status](project-status.md) | What works, what is left, and the roadmap |
| [Gap Analysis](gap-analysis.md) | Comprehensive system-by-system gap analysis with per-feature status tracking |
| [Known Issues](known-issues.md) | Catalogue of known bugs (client/shared and server-side) with severity, status, and root cause |
| [Multiplayer / LAN Setup](multiplayer.md) | `BASE_EXTERNAL` env var, LAN configuration, multi-machine play |
| [Troubleshooting](troubleshooting.md) | Common first-day problems: build OOM, Postgres won't start, `DATABASE_URL` not set, client can't connect, `external/` missing |

---

### `spec/` -- The Cimmeria Bible (canonical, evidence-backed reference)

The bible is the canonical, evidence-backed reference for what the SGW server does. Each chapter follows a 5-section evidence chain (RE findings → client → deprecated server → expected Rust → actual Rust). When the bible contradicts another doc, the bible wins.

Currently Phase 0 (scaffolding) — the writing apparatus exists; gameplay/infrastructure chapter content is Phase 0.5/1 work, authored per-chapter from the V5 evidence pool under `reverse-engineering/findings/`. See umbrella issue [#264](https://github.com/SandboxServers/Cimmeria/issues/264).

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](spec/README.md) | **HUB** -- Master index, system-first navigation, status snapshot of every chapter | Phase 0 |
| [conventions.md](spec/conventions.md) | Citation grammar, frontmatter schema, the no-line-numbers rule for sections 4 and 5 | Phase 0 |
| [how-to-read.md](spec/how-to-read.md) | Reader's guide: status tags, confidence tags, challenge protocol | Phase 0 |
| [how-to-write.md](spec/how-to-write.md) | Author's guide: the 5-section walkthrough, promotion gate (draft → verified → stale → disputed → deprecated) | Phase 0 |
| [glossary.md](spec/glossary.md) | Bible vocabulary (58 terms covering engine / protocol / state / inventory / combat) | Phase 0 |

See also: [.templates/spec-chapter.md](../.templates/spec-chapter.md) (the 5-section skeleton authors copy from), [`reverse-engineering/findings/`](reverse-engineering/findings/) (the section-1 evidence pool).

---

### `content/` -- Content Data Audit + Content Engine

Content-level audit of all game data plus the cradle-to-grave reference for the data-driven content engine that drives missions, dialogs, region triggers, and consumables at runtime.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](content/README.md) | **HUB** -- Playability matrix, content summary, zone progression, reconstruction priority | Complete |
| [content-inventory.md](content/content-inventory.md) | Statistical inventory of all content types with completeness metrics | Complete |
| [zone-audit.md](content/zone-audit.md) | Per-zone completeness scorecard (2 PLAYABLE, 2 PARTIAL, 5 transport-only, 14 SHELL, 1 DATA-ONLY) | Complete |
| [mission-chains.md](content/mission-chains.md) | All 1,040 missions: scripted chains, unscripted analysis, inferred reconstruction | Complete |
| [association-map.md](content/association-map.md) | The crazy wall: cross-references, broken FKs, orphaned content, reconstruction web | Complete |
| [archetype-content-map.md](content/archetype-content-map.md) | Per-archetype content availability (2 implemented, 6 placeholder) | Complete |
| [reconstruction-map.md](content/reconstruction-map.md) | What can be rebuilt vs holes vs never-built, priority recommendations | Complete |
| [external-data-analysis.md](content/external-data-analysis.md) | Analysis of 11 external dev team spreadsheets and text files | Complete |
| [interaction-flags.md](content/interaction-flags.md) | `EInteractionNotificationType` bitmask reference for `set_interaction_type` actions | Complete |
| [equip-from-inventory-pattern.md](content/equip-from-inventory-pattern.md) | **EXPLANATION** — chain shape for granting weapons via a manual equip step instead of force-equipping into the bandolier (mission 622 / 641 worked examples) | Complete |
| [content-engine.md](content/content-engine.md) | **REFERENCE** — the runtime: architecture, vocabulary, schema, lifecycle, observability, performance | Complete |
| [extending-the-engine.md](content/extending-the-engine.md) | **HOW-TO** — add a new trigger / condition / action variant | Complete |
| [proposed-extensions.md](content/proposed-extensions.md) | **ROADMAP** — justified engine extensions tied to recent direction or shipped content | Complete |
| [serverEd-comparison.md](content/serverEd-comparison.md) | Gap analysis vs. the legacy SGW visual-graph editor | Complete |

See also: [gap-analysis.md](gap-analysis.md), [game-data.md](game-data.md)

---

### `protocol/` -- Wire Formats and Messaging

Network protocol internals: packet structures, Mercury reliable messaging, entity property synchronization, and specific message flows. Section index: [protocol/README.md](protocol/README.md).

| Document | Description | Status |
|----------|-------------|--------|
| [message-catalog.md](protocol/message-catalog.md) | **HUB** -- Complete catalog of all 420 network messages with IDs, directions, and payload structures | Complete |
| [mercury-wire-format.md](protocol/mercury-wire-format.md) | Mercury packet header, reliable sequencing, AES-256 encryption, ACK/NACK handling | Complete (legacy summary). Canonical authoritative source is the bible chapter at [`drafts/spec/mercury-wire-format.md`](drafts/spec/mercury-wire-format.md). |
| [entity-property-sync.md](protocol/entity-property-sync.md) | How entity properties are serialized, delta-compressed, and synchronized client/server | Complete |
| [login-handshake.md](protocol/login-handshake.md) | HTTP auth, SOAP schemas, session key exchange, baseAppLogin binary format, error recovery | Complete |
| [position-updates.md](protocol/position-updates.md) | 32 avatarUpdate variants, packed formats, SVID aliasing, client prediction/reconciliation | Complete (legacy summary). Canonical authoritative source is the bible chapter at [`drafts/spec/position-updates.md`](drafts/spec/position-updates.md). |

See also: [technical/mercury-protocol.md](technical/mercury-protocol.md), [technical/mercury-audit.md](technical/mercury-audit.md), [technical/login-auth-flow.md](technical/login-auth-flow.md), [technical/post-auth-sequence.md](technical/post-auth-sequence.md), [technical/network-messages.md](technical/network-messages.md)

---

### `gameplay/` -- Game System Documentation

Per-system breakdowns of game mechanics, derived from RE analysis, entity definitions, and Python scripts. 27 per-system documents covering combat, weapon/ammo, abilities, effects, stats, inventory, crafting, missions, travel, cinematics, ring transport, minigames, social systems, NPC AI, spawning, loot, progression, death/respawn, and character creation.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](gameplay/README.md) | **HUB** -- System dashboard with status and cross-references for every gameplay system | Complete |
| [combat-system.md](gameplay/combat-system.md) | Cover mechanics, threat/aggro, damage resolution, death/respawn | Complete |
| [weapon-ammo-reload.md](gameplay/weapon-ammo-reload.md) | Per-bandolier-slot ammo, fire-gate validation, `requestReload` warmup, magazine refill, persistence batching | Complete |
| [death-respawn-system.md](gameplay/death-respawn-system.md) | Death state, corpse lifecycle, respawn placement and the respawn-point selection flow | Complete |
| [ability-system.md](gameplay/ability-system.md) | Ability activation, cooldowns, targeting, channeling, combos | Complete |
| [effect-system.md](gameplay/effect-system.md) | Buffs, debuffs, DoTs, HoTs, effect stacking and priority | Complete |
| [stat-system.md](gameplay/stat-system.md) | Base stats, derived stats, level scaling, equipment modifiers | Complete |
| [inventory-system.md](gameplay/inventory-system.md) | Item slots, stacking, equipment, bag management | Complete |
| [crafting-system.md](gameplay/crafting-system.md) | Blueprints, material requirements, crafting stations, 499 recipes | Complete |
| [mission-system.md](gameplay/mission-system.md) | Quest objectives, step advancement, rewards, mission scripts | Complete |
| [gate-travel.md](gameplay/gate-travel.md) | Stargate dialing, 29 defined gates, zone transitions | Complete |
| [minigame-system.md](gameplay/minigame-system.md) | 10 minigames (3 implemented, 7 remaining), lockpicking, hacking, etc. | Complete |
| [organization-system.md](gameplay/organization-system.md) | Guilds/organizations, ranks, permissions, roster management | Complete |
| [group-system.md](gameplay/group-system.md) | Party formation, loot rules, group chat | Complete |
| [contact-list.md](gameplay/contact-list.md) | Contact list management, named lists, flags, event notifications | Complete |
| [chat-system.md](gameplay/chat-system.md) | Chat channels, whispers, emotes, chat commands | Complete |
| [mail-system.md](gameplay/mail-system.md) | In-game mail, attachments, delivery | Complete |
| [trade-system.md](gameplay/trade-system.md) | Player-to-player trade, trade windows, confirmation | Complete |
| [black-market.md](gameplay/black-market.md) | Auction house, listings, bidding, buyout | Complete |
| [pet-system.md](gameplay/pet-system.md) | Pet summoning, commands, abilities | Complete |
| [duel-system.md](gameplay/duel-system.md) | PvP duel requests, rules, resolution | Complete |
| [npc-ai.md](gameplay/npc-ai.md) | NPC AI state machine, threat system, aggro, ability selection | Complete |
| [spawn-system.md](gameplay/spawn-system.md) | Spawn regions, spawn sets, population management, respawn timers | Complete |
| [loot-system.md](gameplay/loot-system.md) | Loot generation algorithm, loot tables, eligibility rules | Complete |
| [progression-system.md](gameplay/progression-system.md) | XP curves, leveling, stat growth, training points, applied science | Complete |
| [cinematic-system.md](gameplay/cinematic-system.md) | Kismet/Matinee sequences: stargate animations, ring transport, ability VFX, console activations, camera control | Complete |
| [ring-transport-system.md](gameplay/ring-transport-system.md) | Ring transporter mechanics: 8-state FSM, activation flow, multi-player sync, Kismet integration | Complete |
| [character-creation.md](gameplay/character-creation.md) | Character creation flow, archetypes, visual choices, starting loadout | Complete |

See also: [game-systems.md](game-systems.md), [technical/game-systems.md](technical/game-systems.md), [technical/game-data-analysis.md](technical/game-data-analysis.md)

---

### `engine/` -- BigWorld and CME Internals

How the underlying BigWorld engine and CME game framework operate inside sgw.exe. Section index: [engine/README.md](engine/README.md).

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](engine/entity-type-catalog.md) | **HUB** -- All 18 entity types with properties, methods, and interface bindings | Complete |
| [bigworld-architecture.md](engine/bigworld-architecture.md) | BigWorld 1.9.1 client internals: entity manager, space manager, connection layer | Complete |
| [cme-framework.md](engine/cme-framework.md) | CME framework: PropertyNode, EventSignal (750 types), Atrea scripts, SpaceViewport | Complete |
| [cooked-data-pipeline.md](engine/cooked-data-pipeline.md) | PAK/ZIP format, XML cooking, gSOAP deserialization, Mercury resource delivery | Complete |
| [watcher-system.md](engine/watcher-system.md) | BigWorld watcher/debug variable system (not used by SGW) | Complete |
| [space-management.md](engine/space-management.md) | WorldGrid AoI, 24 spaces, BSP trees, ghost entities, load balancing | Complete |
| [entity-lod-system.md](engine/entity-lod-system.md) | BigWorld entity property LOD and volatile info (not used by SGW) | Complete |
| [distributed-checkpointing.md](engine/distributed-checkpointing.md) | BigWorld backup/recovery system, Cimmeria gap analysis | Complete |
| [entity-def-guide.md](engine/entity-def-guide.md) | Reference for the entity definition (`.def`) file format: property/method declarations, interfaces, type aliases | Complete |
| [character-visual-components.md](engine/character-visual-components.md) | Character visual components: how avatar appearance (model, skin, equipment) is composited on the client | Complete |
| [client-visual-system.md](engine/client-visual-system.md) | Client visual system: rendering, scene graph, and how entities are drawn in the BigWorld/UE3 client | Complete |
| [cooked-data-pak-format.md](engine/cooked-data-pak-format.md) | Cooked-data PAK file format: on-disk layout, entry table, compression, and how the client reads resource packs | Complete |
| [ue3-package-format.md](engine/ue3-package-format.md) | SGW UE3 package binary format (ver 486 licensee fork): section ordering and the `total_header_size` trap, LZO chunking, variable-length export trailers, actor/component serial prefixes, ULevel `Actors` layout, ver-486 property tag stream, HUD↔world coordinate swizzle | Complete |

See also: [technical/bigworld-version-analysis.md](technical/bigworld-version-analysis.md), [technical/sgw-binary-overview.md](technical/sgw-binary-overview.md)

---

### `architecture/` -- Cimmeria Server Architecture

How the Cimmeria emulator itself is structured. 32 documents.

| Document | Description | Status |
|----------|-------------|--------|
| [service-architecture.md](architecture/service-architecture.md) | Auth, Base, Cell service topology, inter-service protocol, developer mode, console commands | Complete |
| [server-infrastructure-proposals.md](architecture/server-infrastructure-proposals.md) | The five unbuilt server-only systems, with a concrete design for each: session resume across a network blip, per-player rate limiting, world-state persistence, a global event scheduler, and economy instrumentation. Sequenced by test-session pain, not cost | Proposed |
| [server-systems.md](architecture/server-systems.md) | **Superseded pointer page.** Routing table showing where each of the original eight server-system sections went, plus the four stale claims most likely to be re-quoted from it | Superseded |
| [python-console.md](architecture/python-console.md) | **Historical.** The deprecated server's Python REPL: its security model (still a useful checklist for the admin API) and the 116-command operator-capability inventory. Wire format and reference client removed — recoverable from `deprecated/cpp/` | Historical |
| [scaling-analysis.md](architecture/scaling-analysis.md) | Scaling strategy: BigWorld vs Cimmeria comparison, 5-tier scaling ladder, Tier-0 recommendation. **Conclusions current; all measured figures are C++-era and unverified against Rust** | Complete (figures stale) |
| [tech-stack-replacement.md](architecture/tech-stack-replacement.md) | **2026-03 decision record.** The audit that chose a rewrite: 5 options, codebase inventory, protocol-reimplementation feasibility. Recommended C#/.NET; the project built Rust. The content-authoring-bottleneck finding still drives the content engine | Historical |
| [data-driven-content-engine.md](architecture/data-driven-content-engine.md) | Data-driven content engine: replace per-script Python with DB-driven trigger/condition/action chains, full schema, worked examples, runtime implementation, migration path | Complete |
| [mission-pak-overrides.md](architecture/mission-pak-overrides.md) | How Cimmeria injects new mission steps without reshipping `CookedDataMissions.pak`: `MissionOverride` patcher, `InvalidKeys` handshake, content-derived metadata bump, XML-index gotcha, operator runbook | Complete |
| [tauri-rewrite.md](architecture/tauri-rewrite.md) | Tauri desktop app rewrite analysis: replacing Qt ServerEd with a modern Rust+TypeScript stack | Complete |
| [migration-roadmap.md](architecture/migration-roadmap.md) | **Historical.** C++-only dependency upgrade plan (MSVC ✅, PostgreSQL ✅ 17.9; the rest unexecuted and unneeded by Rust). Its "CRITICAL OpenSSL" row is **not** a Cimmeria finding — see [project-status.md](project-status.md) for the real roadmap | Historical |
| [state-flag-conventions.md](architecture/state-flag-conventions.md) | Reference for state-flag write conventions: refcounted vs raw, who can clear, auth flow | Complete |
| [abilities-and-effects-system.md](architecture/abilities-and-effects-system.md) | ADR for the abilities + effects design decisions shipped in PR #420: EffectScript trait shape, stacking semantics, channel cancellation triggers, absorption pool drain ordering, TCM dispatch routing, AF_CHANNEL_ALLOWS_MOVEMENT default | Complete |
| [state-field-bits.md](architecture/state-field-bits.md) | Verified `bStateField` bit layout (bits 0-7 only), client dispatch table, BSF_Holster retirement notice with Ghidra anchors, relog persistence of `BSF_AutoCycling` | Complete |
| [gm-cell-method-gating.md](architecture/gm-cell-method-gating.md) | ADR for the server-authoritative GM gate (#475 / CAT-N-03): `access_level` plumbing into `CellEntity`, the dispatch-layer `gm_gate`, how to add the next `gm*` method | Complete |
| [movement-validation.md](architecture/movement-validation.md) | ADR for server-authoritative position validation (#478 / CAT-B-01,-06,-09): the 4-layer seam (bounds / speed warn-only / teleport / navmesh), dual teleport gate, server-clock dt, authorized-teleport reseed, warn-only spaceId cross-check, tolerances + calibration path | Complete |
| [gm-cell-method-adapt-plan.md](architecture/gm-cell-method-adapt-plan.md) | Roadmap for the developer-useful ADAPT `gm*` cell methods (#473/#518): the feedback-channel blocker that unlocks the query surface, name→id resolution, the `loadX` hot-reload family, recommended sequencing | Complete |
| [dev-console-channel.md](architecture/dev-console-channel.md) | ADR for the GM `.`-console (#523): chat-intercept channel for the ~66 dev/authoring commands with no native slash binding, registry dispatch, record→confirm seed-SQL authoring (live write + per-session log + Discord hook), FanMMORPG patrol authoring + schema, per-command status | Complete |
| [integration-test-infra.md](architecture/integration-test-infra.md) | Live-DB test infrastructure: why no testcontainers, why no `sqlx::test`, local setup, isolation patterns | Complete |
| [transport-trait.md](architecture/transport-trait.md) | ADR: `Transport` trait for the Mercury send side — `UdpTransport`/`TestTransport`, the send/recv asymmetric split, why `Nub` I/O (#57) was retired, and the fan-out byte test seam | Complete |
| [mercury-bundle.md](architecture/mercury-bundle.md) | `ChannelBundle` accumulator: cross-entity bundling rule, transaction-state hazard, AoI burst migration (#356), follow-up migration playbook | Complete |
| [negative-logging-convention.md](architecture/negative-logging-convention.md) | Negative-logging convention (issue #304): three patterns, field-naming rules, level discipline, defensible silent sends, `LogCapture` regression-guard helper | Complete |
| [instrumentation-discipline.md](architecture/instrumentation-discipline.md) | Instrumentation discipline (issue #482): success-side rules — dispatch-entrypoint info spans, debug-event discriminators, hot-loop span discipline, metric-label cardinality | Complete |
| [encryption-modernization.md](architecture/encryption-modernization.md) | ADR (Proposed) for issue #434: auth TLS (rustls loopback proxy around libcurl), argon2id passwords, Mercury v2 wire crypto (HKDF/random-IV/HMAC-SHA256, version-gated). RE targets in [findings/auth-and-crypto-modernization-targets.md](reverse-engineering/findings/auth-and-crypto-modernization-targets.md) | Proposed |
| [observability.md](architecture/observability.md) | ADR for server-side observability: OTLP exporter, Mercury packet instrumentation, SigNoz overlay, target catalog, `decision_outcome` enum | Complete |
| [dev-session-telemetry.md](architecture/dev-session-telemetry.md) | Dev-session telemetry pipeline: the `/auth/dev-session` HMAC token, launcher `telemetry/` capture, storage layout | Complete |
| [client-telemetry.md](architecture/client-telemetry.md) | Client-side telemetry architecture: from-scratch instrumentation hookpoints in the launcher, capture surface, transport | Complete |
| [discord-notifications.md](architecture/discord-notifications.md) | Discord notification design + ops: `EventKind` catalogue, channel routing, embed formatting, default toggles | Complete |
| [atrea-editor-bridge.md](architecture/atrea-editor-bridge.md) | ADR for the Atrea Editor bridge — an MCP server exposing the in-game UnrealEd surface | Complete |
| [mercury-loopback-harness.md](architecture/mercury-loopback-harness.md) | ADR for the Tier-2 Mercury loopback session harness: channel state, retransmit, fragmentation, keepalive, ack, RTO test seam | Complete |
| [network-chaos-testing.md](architecture/network-chaos-testing.md) | ADR for the network-chaos apparatus: lossy-socket wrappers, pcap-replay infra, chaos scenarios over the L2 trait | Complete |
| [wireclient.md](architecture/wireclient.md) | ADR for `cimmeria-wireclient`: headless wire-level test client, `session_trace` JSONL schema, pcap exporter | Complete |
| [black-market.md](architecture/black-market.md) | ADR for the Black Market / auction house (#571, PR #586 — **unmerged**): cell methods 61–66 in / client methods 90–95 out, the four-state auction lifecycle, DELETE-based item escrow + SQL-guarded cash escrow, the 30 s expiry sweep, the reserved system seller for boot-seed listings, and the shelved client-method binding that forces a runtime patch (#587). Open: guessed `next_min_bid`, unbounded search (CAT-I-05), undecodable `sellerName` | Implemented, unmerged |

See also: [building.md](building.md), [connection-flow.md](connection-flow.md), [../TESTING.md](../TESTING.md)

---

### `client/` -- Game Client Analysis

Analysis of game client binaries, launcher tools, and client asset inventories. 7 documents.

| Document | Description | Status |
|----------|-------------|--------|
| [sgw-launcher.md](client/sgw-launcher.md) | Launcher design: seed + patch manifest install, Atera-detection launch, single-PUT log upload, hostname patch | Complete |
| [launcher-guide.md](client/launcher-guide.md) | User-facing guide: how the launcher works for players, how operators prepare and publish patches | Complete |
| [launcher-distribution-setup.md](client/launcher-distribution-setup.md) | Operational runbook: GitHub Releases publish flow for content, Ed25519 manifest signing setup, Azure Blob SAS for log uploads | Complete |
| [audio-voice-inventory.md](client/audio-voice-inventory.md) | Complete FMOD audio inventory: 280 .fev + 566 .fsb files, zone ambience, music, weapons, abilities, UI, dialog VO gap analysis | Complete |
| [facefx-lip-sync.md](client/facefx-lip-sync.md) | FaceFX lip sync system: .fxa animation files, phoneme mapping, engine integration | Complete |
| [ui-layout-inventory.md](client/ui-layout-inventory.md) | UI layout inventory: all Scaleform .swf files, Lua bindings, screen types, HUD elements | Complete |
| [crash-dumps.md](client/crash-dumps.md) | SGW crash-dump pipeline: minidump capture, symbolication, what the dumps reveal about client state | Complete |

See also: [client-tools.md](client-tools.md), [launcher-exe.md](reverse-engineering/binaries/launcher-exe.md), [technical/atrealoader-exe.md](technical/atrealoader-exe.md)

---

### `tools/` -- Development Tools

Design documents for development and administration tools.

| Document | Description | Status |
|----------|-------------|--------|
| [admin-panel.md](tools/admin-panel.md) | Web admin dashboard: architecture, Flask+React stack, py_client protocol bridge, DB queries, API routes | Complete |
| [admin-api.md](tools/admin-api.md) | REST + WebSocket admin API: `crates/admin-api/` Axum implementation, routes, auth, Tauri IPC bridge | Complete |

---

### `analysis/` -- Investigation Logs

Working notes and cross-reference indexes from ongoing RE sessions.

| Document | Description | Status |
|----------|-------------|--------|
| [event-net-mapping.md](analysis/event-net-mapping.md) | 420 Event_NetIn/NetOut mapped to .def methods, Ghidra addresses, handler chains (~98% coverage) | Complete |
| [bigworld-reference-index.md](analysis/bigworld-reference-index.md) | Cross-reference: BigWorld 2.0.1 source symbols to sgw.exe addresses | Complete |

---

### `audits/` -- Conformance Audits

Point-in-time conformance records. Each audit pins the spec version and the
`binary_sha256` it was run against; findings are not rewritten afterwards.

| Document | Description | Status |
|----------|-------------|--------|
| [mercury-rust-conformance-2026-05-15.md](audits/mercury-rust-conformance-2026-05-15.md) | Rust Mercury implementation vs. the [mercury-wire-format](drafts/spec/mercury-wire-format.md) bible chapter | Under review |
| [entity-property-sync-section2-audit-2026-05-16.md](audits/entity-property-sync-section2-audit-2026-05-16.md) | Section 2 of the [entity-property-sync](drafts/spec/entity-property-sync.md) chapter audited against the binary | Complete |
| [telemetry-audit-2026-06-01.md](audits/telemetry-audit-2026-06-01.md) | Telemetry + logging sweep of code landed 2026-05-31 → 2026-06-01. Companions: [architecture/observability.md](architecture/observability.md), [architecture/negative-logging-convention.md](architecture/negative-logging-convention.md) | Complete |

---

### `security-audit/` -- Server-Authority and Anti-Cheat Audits

Time-stamped security-audit records, one directory per audit. Findings are a
point-in-time snapshot and are **not** updated post-audit — remediation is
tracked through the linked GitHub issues. Index: [security-audit/README.md](security-audit/README.md).

| Document | Description | Status |
|----------|-------------|--------|
| [2026-05-31-server-authority/UMBRELLA.md](security-audit/2026-05-31-server-authority/UMBRELLA.md) | **HUB** -- Tracking record for the exhaustive server-authority / anti-cheat / anti-replay sweep across every player-facing wire surface | Complete |
| [2026-05-31-server-authority/BRIEF.md](security-audit/2026-05-31-server-authority/BRIEF.md) | Shared agent brief: evidence rules every per-category auditor worked under | Complete |
| [2026-05-31-server-authority/surface.md](security-audit/2026-05-31-server-authority/surface.md) | The client → server outbound message surface (~250 `Event_NetOut_*` classes) extracted from SGW.exe | Complete |
| [2026-05-31-server-authority/findings/](security-audit/2026-05-31-server-authority/findings/) | Per-category findings, CAT-A through CAT-O: auth, movement, combat/abilities, inventory, vendor, crafting, mail, trade, black market, mission/dialog, minigame, chat/contact, org/squad/duel, GM commands, world/space/gate | Complete |

---

### `guides/` -- How-To Guides

Practical guidance for contributors working on the RE effort or the emulator.

| Document | Description | Status |
|----------|-------------|--------|
| [getting-started.md](guides/getting-started.md) | **Start here for new contributors.** First-time tutorial: prerequisites → `setup.ps1` → verifying the server is up → connecting the client → running tests → where to go next | Complete |
| [add-a-message-handler.md](guides/add-a-message-handler.md) | **How-to.** The most common first server-side feature: route a new client message, decode the payload, do the work, reply. Tied to dispatcher seams, decode helpers, and the test types you need | Complete |
| [extend-the-content-engine.md](guides/extend-the-content-engine.md) | **How-to (quickstart).** 1-page entry point to adding a new trigger / condition / action. Defers to [content/extending-the-engine.md](content/extending-the-engine.md) for the detailed walkthrough | Complete |
| [write-a-database-migration.md](guides/write-a-database-migration.md) | **How-to.** Schema vs. migration vs. seed; the `db/scripts/` idempotent pattern; live-DB test discipline; verifying idempotency before pushing | Complete |
| [re-toolchain-setup.md](guides/re-toolchain-setup.md) | **Start here for RE.** End-to-end setup for Ghidra, x64dbg, MCP bridges, and `.mcp.json`. Includes the `pwsh setup.ps1 -WithReToolchain` automated path. | Complete |
| [reverse-engineering-with-claude.md](guides/reverse-engineering-with-claude.md) | Workflow doc: when to invoke `game-archaeology-specialist`, Six-Phase mapping to Claude Code sessions, evidence handoff to `documentation-writer`, what NOT to delegate | Complete |
| [reading-decompiled-code.md](guides/reading-decompiled-code.md) | Tips for reading Ghidra decompiler output, common patterns, pitfalls | Complete |
| [sgw-live-debugging.md](guides/sgw-live-debugging.md) | Live debugging SGW.exe with x32dbg + log breakpoints — manual fallback when MCP-driven flows fail; pybag incompatibility documented | Complete |

Two former-guides files moved to their correct homes in #344: [evidence-standards.md](reverse-engineering/evidence-standards.md) is now under `reverse-engineering/` (it's the standards reference for the RE process), and [entity-def-guide.md](engine/entity-def-guide.md) is now under `engine/` (it's reference doc on entity definition files).

---

### `reverse-engineering/` -- Ghidra Work

Annotation scripts, function naming progress, per-system RE findings, and toolchain installation references.

See [reverse-engineering/README.md](reverse-engineering/README.md) for the top-level orientation. New RE contributors should start at [guides/re-toolchain-setup.md](guides/re-toolchain-setup.md) before exploring this directory.

| Document | Description | Status |
|----------|-------------|--------|
| [README.md](reverse-engineering/README.md) | **HUB** -- Top-level orientation for the RE tree, directory map, bible relationship | Complete |
| [PLAN.md](reverse-engineering/PLAN.md) | RE plan: phases, targets, methodology | Complete |
| [STATUS.md](reverse-engineering/STATUS.md) | RE status: 101,909/168,239 functions named (60.6%), all 5 phases complete | Complete |
| [function-naming-progress.md](reverse-engineering/function-naming-progress.md) | Naming conventions, coverage metrics, per-script results | Complete |
| [address-map.md](reverse-engineering/address-map.md) | Key address table: vtables, global objects, critical functions in sgw.exe | Complete |
| [editor-source-mapping.md](reverse-engineering/editor-source-mapping.md) | Editor Ghidra map: SGW.exe function ↔ reference-source mapping for the in-game editor surface | Complete |
| [toolchain/install-ghidra-mcp.md](reverse-engineering/toolchain/install-ghidra-mcp.md) | GhidraMCP plugin install reference — manual + bootstrap-driven paths, port-fallback gotcha | Complete |

#### `binaries/` -- Binary Analysis

| Document | Description | Status |
|----------|-------------|--------|
| [sgw-exe.md](reverse-engineering/binaries/sgw-exe.md) | sgw.exe binary overview from Ghidra: sections, RTTI, function counts, layout | Complete |
| [launcher-exe.md](reverse-engineering/binaries/launcher-exe.md) | CME Launcher.exe RE analysis: patch client internals, SOAP protocol | Complete |

#### `annotation-scripts/` -- Ghidra Jython Scripts (Phase 1 — all run)

| Script | Purpose | Status |
|--------|---------|--------|
| `01_rtti_annotator.py` | Annotate functions from RTTI type-info / typeinfo structures | Complete |
| `02_ue3_exec_annotator.py` | Annotate UE3 `exec`/native-function dispatch points | Complete |
| `03_bigworld_source_annotator.py` | Map BigWorld reference-source symbols onto sgw.exe functions | Complete |
| `04_event_signal_annotator.py` | Annotate CME EventSignal emit/subscribe handlers | Complete |
| `05_mercury_annotator.py` | Annotate Mercury networking functions (Nub, Channel, Connection) | Complete |
| `06_cme_framework_annotator.py` | Annotate CME framework functions (PropertyNode, SpaceViewport, etc.) | Complete |
| `07_vtable_annotator.py` | Name vtable methods from class layouts | Complete |
| `07b_targeted_vtable_annotator.py` | Targeted follow-up pass for specific class vtables | Complete |
| `08_lua_binding_annotator.py` | Annotate Lua/Scaleform binding glue functions | Complete |
| `09_string_discovery.py` | Discover and label functions via referenced string constants | Complete |
| `10_xref_propagation.py` | Propagate names across the call graph via cross-references | Complete |

#### `findings/` -- Per-System Wire Format Findings (Phases 2–4)

| Document | Messages | Description | Confidence |
|----------|----------|-------------|------------|
| [combat-wire-formats.md](reverse-engineering/findings/combat-wire-formats.md) | 29 | Universal RPC dispatcher, abilities, effects, stats, timers | HIGH |
| [inventory-wire-formats.md](reverse-engineering/findings/inventory-wire-formats.md) | 22 | Bags, items, equipment, repair, salvage | HIGH |
| [entity-types-wire-formats.md](reverse-engineering/findings/entity-types-wire-formats.md) | 14 | Entity creation, cache stamps, version info | HIGH |
| [entity-property-sync.md](reverse-engineering/findings/entity-property-sync.md) | — | Property ID assignment, delta encoding, create/update formats | HIGH |
| [mission-wire-formats.md](reverse-engineering/findings/mission-wire-formats.md) | 11 | Mission accept, advance, abandon, rewards | HIGH |
| [organization-wire-formats.md](reverse-engineering/findings/organization-wire-formats.md) | 18 | Guild create, invite, ranks, permissions, roster | HIGH |
| [crafting-wire-formats.md](reverse-engineering/findings/crafting-wire-formats.md) | 8 | Blueprints, crafting stations, research | HIGH |
| [gate-travel-wire-formats.md](reverse-engineering/findings/gate-travel-wire-formats.md) | 6 | DHD, gate activation, zone transitions | HIGH |
| [group-wire-formats.md](reverse-engineering/findings/group-wire-formats.md) | 4 | Group authority, member coordination, mob groups | HIGH |
| [minigame-wire-formats.md](reverse-engineering/findings/minigame-wire-formats.md) | 9 | Lockpick, hacking, fishing minigames | HIGH |
| [chat-wire-formats.md](reverse-engineering/findings/chat-wire-formats.md) | 6 | Channels, whispers, emotes | HIGH |
| [mail-wire-formats.md](reverse-engineering/findings/mail-wire-formats.md) | 5 | Send, receive, attachments, delete | HIGH |
| [black-market-wire-formats.md](reverse-engineering/findings/black-market-wire-formats.md) | 6 | Auction listings, bids, buyout | HIGH |
| [contact-list-wire-formats.md](reverse-engineering/findings/contact-list-wire-formats.md) | 4 | Named lists, flags, event notifications | HIGH |
| [trade-wire-formats.md](reverse-engineering/findings/trade-wire-formats.md) | 5 | Trade initiate, offer, accept/cancel | HIGH |
| [duel-wire-formats.md](reverse-engineering/findings/duel-wire-formats.md) | 3 | Duel request, accept, resolution | HIGH |
| [pet-wire-formats.md](reverse-engineering/findings/pet-wire-formats.md) | 2 | Pet summon, commands | HIGH |
| [system-protocol-wire-formats.md](reverse-engineering/findings/system-protocol-wire-formats.md) | — | Ghidra decompilation evidence for the system/connection protocol message handlers | HIGH |
| [space-viewport-wire-formats.md](reverse-engineering/findings/space-viewport-wire-formats.md) | — | Space, viewport, and entity-lifecycle wire formats (spaceViewportInfo, enter/leave AoI) | HIGH |
| [position-movement-wire-formats.md](reverse-engineering/findings/position-movement-wire-formats.md) | — | Position and movement wire formats: avatarUpdate variants, forced position, packed coords | HIGH |
| [entity-creation-wire-formats.md](reverse-engineering/findings/entity-creation-wire-formats.md) | — | Entity creation wire formats: createBasePlayer / createCellPlayer, cache stamps | HIGH |
| [cme-event-signal.md](reverse-engineering/findings/cme-event-signal.md) | — | CME EventSignal emit pipeline + `TypedEmitInfo`/`CallbackImpl` class anatomy (V5 campaign session 1) | HIGH |
| [mercury-nub-anatomy.md](reverse-engineering/findings/mercury-nub-anatomy.md) | — | Mercury `Nub` / `BaseNub` / `ChannelInternal` / `Connection` class layouts (22 functions, 4 struct anatomies); two-channel-map design; network thread loop; `Nub::send` 4-phase pipeline; rdtsc inactivity vs our `MAX_RETRIES`; two latent wire gaps | HIGH |
| [mercury-protocol-internals.md](reverse-engineering/findings/mercury-protocol-internals.md) | — | Mercury protocol internals from the client binary: reliable sequencing, ack/nack, fragmentation | HIGH |
| [struct-field-layouts.md](reverse-engineering/findings/struct-field-layouts.md) | — | FIXED_DICT struct field layouts recovered from the client binary | HIGH |
| [combat-damage-analysis.md](reverse-engineering/findings/combat-damage-analysis.md) | — | Combat damage system from the client binary: resolution chain, multipliers, result codes | HIGH |
| [ability-resolution-pipeline.md](reverse-engineering/findings/ability-resolution-pipeline.md) | — | Ability resolution pipeline: activation → targeting → effect dispatch | HIGH |
| [effect-execution-model.md](reverse-engineering/findings/effect-execution-model.md) | — | Effect execution model from the client binary: apply/remove, pulsing, stacking | HIGH |
| [cover-system.md](reverse-engineering/findings/cover-system.md) | — | Cover system from the client binary: reservation state, surfacing channels (no direct claim message) | HIGH |
| [stat-scaling-formulas.md](reverse-engineering/findings/stat-scaling-formulas.md) | — | Stat scaling and XP progression formulas | HIGH |
| [faction-alignment-system.md](reverse-engineering/findings/faction-alignment-system.md) | — | Faction / alignment system: single-byte ClientMethod broadcasts of alignment state | HIGH |
| [state-flag-broadcast.md](reverse-engineering/findings/state-flag-broadcast.md) | — | State-flag (`BSF_*`) broadcast: how `bStateField` bits propagate to witnesses | HIGH |
| [inventory-state-machine.md](reverse-engineering/findings/inventory-state-machine.md) | — | Client-side inventory state machine: the `Inventory` class, 14 CME signals, item-tree model | HIGH |
| [weapon-ammo-pipeline.md](reverse-engineering/findings/weapon-ammo-pipeline.md) | — | Weapon / ammo pipeline: fire-gate, ammo decrement, reload warmup | HIGH |
| [crafting-state-machine.md](reverse-engineering/findings/crafting-state-machine.md) | — | Crafting state machine: discipline / research / reverse-engineer flow | HIGH |
| [loot-generation.md](reverse-engineering/findings/loot-generation.md) | — | Loot generation pipeline: roll model, eligibility, the ephemeral loot window | HIGH |
| [mission-state-machine.md](reverse-engineering/findings/mission-state-machine.md) | — | Client-side mission state machine: `MissionSet` singleton, five incoming wire messages | HIGH |
| [character-creation-pipeline.md](reverse-engineering/findings/character-creation-pipeline.md) | — | Character creation pipeline: createCharacter RPC path, validation, starting loadout | HIGH |
| [world-entry-pipeline.md](reverse-engineering/findings/world-entry-pipeline.md) | — | World entry pipeline: login → entity creation → AoI bootstrap sequence | HIGH |
| [npc-ai-state-machine.md](reverse-engineering/findings/npc-ai-state-machine.md) | — | NPC AI state machine from the client binary: states, threat, ability selection | HIGH |
| [npc-movement-pathfinding.md](reverse-engineering/findings/npc-movement-pathfinding.md) | — | NPC movement and pathfinding: navmesh use, patrol, leash behavior | HIGH |
| [spawn-system-mechanics.md](reverse-engineering/findings/spawn-system-mechanics.md) | — | Spawn system mechanics from the client binary: spawn regions/sets, population, respawn | HIGH |
| [respawn-lifecycle.md](reverse-engineering/findings/respawn-lifecycle.md) | — | Respawn lifecycle: death → respawn-point selection → placement | HIGH |
| [animation-system.md](reverse-engineering/findings/animation-system.md) | — | Animation system: sequence lookup, combat/weapon animation triggers | HIGH |
| [minigame-architecture.md](reverse-engineering/findings/minigame-architecture.md) | — | Minigame architecture from the client binary: SmartFoxServer session, per-game flow | HIGH |
| [stargate-dhd-state-machine.md](reverse-engineering/findings/stargate-dhd-state-machine.md) | — | Stargate DHD state machine; finding that `onDHDReply` is a comms channel, not a travel event | HIGH |
| [dialog-portrait-lookup.md](reverse-engineering/findings/dialog-portrait-lookup.md) | — | Dialog portrait and speaker-name lookup path | HIGH |
| [client-instrumentation-hookpoints.md](reverse-engineering/findings/client-instrumentation-hookpoints.md) | — | Client instrumentation hookpoints for from-scratch telemetry | HIGH |
| [client-wire-emit-suppression.md](reverse-engineering/findings/client-wire-emit-suppression.md) | — | Client-side wire-emit suppression cases (heal-focus, P90 swap) | HIGH |
| [right-click-routing-on-corpse.md](reverse-engineering/findings/right-click-routing-on-corpse.md) | — | Right-click routing on corpses: why some corpses fail to open the loot window | HIGH |
| [cooked-data-pipeline.md](reverse-engineering/findings/cooked-data-pipeline.md) | — | Cooked-data pipeline binary findings: PAK read path, resource delivery | HIGH |
| [atrea-editor.md](reverse-engineering/findings/atrea-editor.md) | — | Atrea Editor (in-game UnrealEd): architecture and exposed surface | HIGH |
| [architectural-anomalies.md](reverse-engineering/findings/architectural-anomalies.md) | — | Architectural anomalies in the CME EventSignal subsystem | MEDIUM |
| [annotation-script-shift-bugs.md](reverse-engineering/findings/annotation-script-shift-bugs.md) | — | Cyclic-shift bugs in the Ghidra annotation scripts and their impact on naming | MEDIUM |

#### `decompiled/` -- Bulk Decompiled Source Dump

Bulk Ghidra decompiler output, grouped by subsystem (`.c` files), with an index. Reference material — read the index first, not the raw dumps.

| Document | Description |
|----------|-------------|
| [00_INDEX.md](reverse-engineering/decompiled/00_INDEX.md) | **INDEX** -- Map of the 14 decompiled `.c` group files to their subsystems (game classes, BigWorld network, CEGUI, Crypto++, Scaleform, events, entities, data, systems, libraries, debug/config) |

#### `v5-campaign/` -- V5 Function-Naming Campaign

Coordination and checkpoint artifacts from the V5 mass function-naming campaign (status docs plus per-worker checkpoint JSON).

| Document | Description |
|----------|-------------|
| [CAMPAIGN_STATUS.md](reverse-engineering/v5-campaign/CAMPAIGN_STATUS.md) | **STATUS** -- Overall V5 campaign progress and per-worker rollup |
| [SESSION_2_PLAN.md](reverse-engineering/v5-campaign/SESSION_2_PLAN.md) | Session 2 plan: target areas and worker assignments |
| [WORKER_BRIEF.md](reverse-engineering/v5-campaign/WORKER_BRIEF.md) | Per-worker brief: scope, conventions, handoff format |

---

### `technical/` -- Legacy Technical Documents (historical)

Early-project RE analysis from before the reorganised `docs/` tree and the Rust rewrite. **All pages in this directory are now historical** — every file links forward to the current canonical replacement. Read them for context and original first-pass framing; do not extend them. See [`technical/README.md`](technical/README.md) for the directory orientation and the page-by-page replacement table.

| Document | Description |
|----------|-------------|
| [sgw-binary-overview.md](technical/sgw-binary-overview.md) | Binary structure, sections, RTTI analysis, function counts |
| [network-messages.md](technical/network-messages.md) | Complete 420-message catalog with IDs, names, directions |
| [mercury-protocol.md](technical/mercury-protocol.md) | Mercury reliable UDP protocol analysis |
| [mercury-audit.md](technical/mercury-audit.md) | Audit of Mercury implementation against BigWorld reference |
| [login-auth-flow.md](technical/login-auth-flow.md) | Full login and authentication flow analysis |
| [post-auth-sequence.md](technical/post-auth-sequence.md) | What happens after auth: entity creation, world entry |
| [bigworld-version-analysis.md](technical/bigworld-version-analysis.md) | BigWorld version identification (1.9.1 client) |
| [game-systems.md](technical/game-systems.md) | Game systems analysis from the binary |
| [game-data-analysis.md](technical/game-data-analysis.md) | Analysis of game data content and coverage |
| [slash-commands.md](technical/slash-commands.md) | Client slash command handler analysis |
| [server-feasibility.md](technical/server-feasibility.md) | Server emulation feasibility assessment |
| [source-reconstruction-feasibility.md](technical/source-reconstruction-feasibility.md) | Source code reconstruction feasibility |
| [building.md](technical/building.md) | Build process technical details |
| [launcher-exe.md](reverse-engineering/binaries/launcher-exe.md) | Launcher binary analysis (canonical copy lives under `reverse-engineering/binaries/`) |
| [atrealoader-exe.md](technical/atrealoader-exe.md) | AtreaLoader binary analysis |
| [atrealoader-config.md](technical/atrealoader-config.md) | AtreaLoader configuration format |
| [atrearl-loader.md](technical/atrearl-loader.md) | AtreaRL.dll — the runtime patcher injected into SGW.exe (hooks, sniffer, two-gate activation) |


## Key Data Sources

The most important files and directories for RE work, located relative to the project root.

| Source | Path | What It Contains |
|--------|------|------------------|
| Entity definitions | `entities/defs/*.def` | XML property/method definitions for all 18 entity types |
| Entity registry | `entities/entities.xml` | Master entity type list with type IDs |
| Interface definitions | `entities/defs/interfaces/*.def` | Shared property/method sets used across entity types |
| Custom type aliases | `entities/custom_alias.xml` | Type mappings for entity property serialization |
| Python game scripts | `python/` | 164 files: entity behavior, missions, combat, interactions |
| Database schema | `db/database.sql`, `db/sgw/`, `db/resources/` | PostgreSQL schema (split into per-domain directories) |
| Cooked game data | `data/cache/*.pak` | Client resource packs (items, abilities, effects, etc.) |
| Effect/mission scripts | `data/scripts/` | Source .script XML files (visual node graphs) |
| Space definitions | `entities/cell_spaces.xml` | Zone/cell partitioning configuration |
| Server configs | `config/*.config` | Service configuration (ports, DB, tuning parameters) |
| BigWorld reference | *(external)* | BigWorld 1.9.1 + 2.0.1 source for protocol/architecture reference |
| sgw.exe Ghidra project | *(external)* | The primary RE target, loaded in Ghidra |


## Phase Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| **Phase 1** | World Entry -- Login, auth, shard selection, world entry, movement | COMPLETE |
| **Phase 2** | Expanding Content -- More zones, stargate travel, chat, crafting | In Progress |
| **Phase 3** | Social Systems -- Organizations, mail, auction house, minigames | Planned |
| **Phase 4** | Polish -- AI patrols, loot tables, XP curves, stat scaling | Planned |
| **Phase 5** | Modernization -- Dependency upgrades, CMake build, CI/CD, Docker | Planned |

See [Project Status](project-status.md) for detailed breakdown of each phase.


## Evidence Standards

All RE documentation uses a three-tier confidence system to distinguish verified facts from educated guesses.

| Level | Label | Meaning |
|-------|-------|---------|
| **HIGH** | Confirmed | Directly verified in Ghidra disassembly, confirmed by BigWorld reference source, or tested in live emulator |
| **MEDIUM** | Probable | Strong indirect evidence: consistent string references, RTTI matches, behavioral correlation with reference source |
| **LOW** | Speculative | Inferred from naming patterns, partial decompilation, or analogy with similar systems; needs further verification |

When documenting findings, always state the confidence level and cite the evidence basis (address, function name, reference source file, or test observation). See [Evidence Standards](reverse-engineering/evidence-standards.md) for full details.


## Contributing

For the full contribution guide — scope, code style, PR conventions, where to find a first issue, how content-chain work differs from protocol work — see **[../CONTRIBUTING.md](../CONTRIBUTING.md)** at the repo root.

When adding or updating documentation specifically:

1. Place documents in the correct subdirectory per the map above.
2. Use relative links for all cross-references within `docs/`.
3. Tag every claim with a confidence level (HIGH / MEDIUM / LOW).
4. Include Ghidra addresses or source references where applicable.
5. Update this README when adding new documents.
6. Update the doc-update map in [../CLAUDE.md](../CLAUDE.md) so reviewers can verify the right files were touched.

Reporting a security issue? See **[../SECURITY.md](../SECURITY.md)** for the private reporting path. Project conduct expectations are in **[../CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md)**.
