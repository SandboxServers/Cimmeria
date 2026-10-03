---
title: "Gameplay Systems Gap Analysis"
type: explanation
audience: engineers
last_updated: 2026-10-03
---

# Gameplay Systems Gap Analysis

> **Last updated**: 2026-10-03 (token-usage close-out, TP-12: this file became the index and summary, and the per-system sections moved, unchanged, into one file per area under [gap-analysis/](gap-analysis/); see [Systems by area](#systems-by-area). §36 gains a note on the token profiler, which is development tooling and has no matrix row. No row changed.) Before that 2026-09-28 (ammo close-out, AM-12: §9, §12 and §14 gain the special-ammo rows; see "Since 2026-09-25"). Before that 2026-09-28 (#804: §10's clear-on-death row and path forward re-verified; no status changed). Before that 2026-09-27 (social-systems close-out: §21, §24 and §27 and the matrix recount; then the organizations close-out: §21, §23 and §30 and a second recount; then the crafting close-out: §19, with the loot and trade rows that crafting changed in §14 and §22; see [Since 2026-09-25](#since-2026-09-25)). The last full re-verification pass was 2026-09-25, against `main` at `acbcc22e`, about 160 PRs after the 2026-07-25 edition.
> **Purpose**: Map every gameplay system's Rust implementation against what's needed for a complete server
> **Status**: Source of truth for project completion tracking
> **Measured against**: `main`. Work living only on an unmerged feature branch is called out explicitly in the affected section and is **not** counted as implemented.
> **Workspace scale** (2026-09-27, `python tools/extract_tests.py`): **7,337 test functions (6,919 gated in CI)** across **1,192 files** in 49 workspace members, **1,133 live-DB tests** (`require_db_or_skip!`, spread over 13 crates since the services split), **3 PL/pgSQL end-to-end smokes**, with a **first-class content engine** the original Python codebase did not have. CI excludes `cimmeria-app`, `cimmeria-content-editor`, `cimmeria-scene-editor`, `sgw-launcher`, `cimmeria-client-telemetry`, `cimmeria-client-patches` and `cimmeria-lab`, which is the whole of the 7,337 → 6,919 difference. The 2026-09-25 edition's grep counted 5,333 attributes (4,975 in CI); the extractor counts test functions, so the two are close but not identical methods.
>
> **Evidence bar for CW in this edition**: a written record of an in-client test (playtest report, UAT worknote, PR body or comment, issue comment, or a recorded confirmation). Rows that very likely work in-client but have no such record stay NT or IM.
>
> **Arithmetic note**: the 2026-07-25 edition's `TOTALS` row did not match its own per-system tables (its rows sum to 444 / CW 164 / NT 18 / IM 134 / KM 124 / NU 4; its totals line printed 443 / 159 / 18 / 134 / 128 / 4). The 2026-05-27 edition had the same problem. This edition recomputes every matrix row and total directly from the feature tables.

---

## How to read this doc

- **Code paths** cite the active Rust workspace under [`crates/`](../crates/). When a feature exists in the deprecated [`deprecated/python/`](../deprecated/python/) or [`deprecated/cpp/`](../deprecated/cpp/) trees but **not yet** in Rust, the row is marked `KM` (port pending). The Python and C++ trees are reference-only.
- **Confidence** reflects how sure we are about the status — HIGH means the code has been read and judged; MEDIUM means line counts and recent-PR evidence support the status but a deep read hasn't happened; LOW means inference from neighbouring code or .def files.
- **Recent PRs** are listed where they're load-bearing for the status.

## Status Taxonomy

| Status | Symbol | Meaning |
|--------|--------|---------|
| **Confirmed Working** | `CW` | Tested end-to-end with the game client (Castle Cellblock smoke + Lomiada captures) and verified correct |
| **Needs Test** | `NT` | Code exists, looks reasonable, but hasn't been verified with a live client |
| **Implemented** | `IM` | Code written but may be incomplete or have known issues |
| **Known / Missing** | `KM` | We know this needs to exist (from `.def` files, docs, or game design) but no code exists in `crates/` |
| **Needed / Unknown** | `NU` | Server-only system we infer must exist but have no direct evidence for |

---

## Systems by area

Each system's evidence, code paths and feature table live in one file per area. The § numbers match the matrix below.

| § | System | Status | File |
|---|---|---|---|
| 1 | [Authentication and Login](gap-analysis/infrastructure.md#1-authentication-and-login-----cw) | CW | [infrastructure.md](gap-analysis/infrastructure.md) |
| 2 | [Mercury Protocol](gap-analysis/infrastructure.md#2-mercury-protocol-----cw) | CW | [infrastructure.md](gap-analysis/infrastructure.md) |
| 3 | [Game Data Pipeline (Cooked Data + Resources)](gap-analysis/infrastructure.md#3-game-data-pipeline-cooked-data--resources-----cw) | CW | [infrastructure.md](gap-analysis/infrastructure.md) |
| 4 | [Database Persistence](gap-analysis/infrastructure.md#4-database-persistence-----cw) | CW | [infrastructure.md](gap-analysis/infrastructure.md) |
| 5 | [Character Creation](gap-analysis/core-gameplay.md#5-character-creation-----cw-core-create-and-enter-flow-was-nt) | CW | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 6 | [World Entry and Spaces](gap-analysis/core-gameplay.md#6-world-entry-and-spaces-----cw) | CW | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 7 | [Movement and Navigation](gap-analysis/core-gameplay.md#7-movement-and-navigation-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 8 | [Entity Lifecycle (AoI)](gap-analysis/core-gameplay.md#8-entity-lifecycle-aoi-----im-open-entity-introduction-defect-see-project-status-known-issues) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 9 | [Combat and Abilities](gap-analysis/core-gameplay.md#9-combat-and-abilities-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 10 | [Effects and Buffs](gap-analysis/core-gameplay.md#10-effects-and-buffs-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 11 | [Stats](gap-analysis/core-gameplay.md#11-stats-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 12 | [Inventory and Items](gap-analysis/core-gameplay.md#12-inventory-and-items-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 13 | [Missions](gap-analysis/core-gameplay.md#13-missions-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 14 | [Loot](gap-analysis/core-gameplay.md#14-loot-----im) | IM | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 15 | [Stores / Vendors](gap-analysis/core-gameplay.md#15-stores--vendors-----nt) | NT | [core-gameplay.md](gap-analysis/core-gameplay.md) |
| 16 | [NPC AI and Behavior](gap-analysis/npc-systems.md#16-npc-ai-and-behavior-----im) | IM | [npc-systems.md](gap-analysis/npc-systems.md) |
| 17 | [Spawn System](gap-analysis/npc-systems.md#17-spawn-system-----im) | IM | [npc-systems.md](gap-analysis/npc-systems.md) |
| 18 | [XP and Leveling](gap-analysis/secondary-gameplay.md#18-xp-and-leveling-----im) | IM | [secondary-gameplay.md](gap-analysis/secondary-gameplay.md) |
| 19 | [Crafting](gap-analysis/secondary-gameplay.md#19-crafting-----nt-the-crafting-campaign-is-merged-every-verb-respec-stations-and-tools-blueprint-items-and-guides-awaiting-the-owners-cr-14-uat) | NT | [secondary-gameplay.md](gap-analysis/secondary-gameplay.md) |
| 20 | [Stargate Travel](gap-analysis/secondary-gameplay.md#20-stargate-travel-----im) | IM | [secondary-gameplay.md](gap-analysis/secondary-gameplay.md) |
| 21 | [Chat](gap-analysis/secondary-gameplay.md#21-chat-----nt) | NT | [secondary-gameplay.md](gap-analysis/secondary-gameplay.md) |
| 22 | [Trading](gap-analysis/secondary-gameplay.md#22-trading-----im-ported-2026-06-was-km) | IM | [secondary-gameplay.md](gap-analysis/secondary-gameplay.md) |
| 23 | [Organizations / Guilds](gap-analysis/stub-only-systems.md#23-organizations--guilds-----nt-squads-teams-and-commands-work-server-side-awaiting-the-owners-two-client-uat) | NT | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 24 | [Mail](gap-analysis/stub-only-systems.md#24-mail-----nt-send-attachments-with-escrow-take-cod-return-notification-and-30-day-expiry-all-implemented-awaiting-the-owners-in-client-uat) | NT | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 25 | [Black Market (Auction House)](gap-analysis/stub-only-systems.md#25-black-market-auction-house-----km) | KM | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 26 | [Contact Lists](gap-analysis/stub-only-systems.md#26-contact-lists-----cw-shipped-2026-06-20-was-km) | CW | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 27 | [Dueling](gap-analysis/stub-only-systems.md#27-dueling-----im-1v1-duels-implemented-end-to-end-awaiting-the-owners-in-client-uat) | IM | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 28 | [Pets](gap-analysis/stub-only-systems.md#28-pets-----nt-was-km) | NT | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 29 | [Minigames](gap-analysis/stub-only-systems.md#29-minigames-----im-was-km) | IM | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 30 | [Groups / Parties](gap-analysis/stub-only-systems.md#30-groups--parties-----nt-the-squad-is-the-group-see-23) | NT | [stub-only-systems.md](gap-analysis/stub-only-systems.md) |
| 31 | [Content Engine](gap-analysis/new-systems.md#31-content-engine-----cw) | CW | [new-systems.md](gap-analysis/new-systems.md) |
| 32 | [Mercury Bundle / ChannelBundle](gap-analysis/new-systems.md#32-mercury-bundle--channelbundle-----cw) | CW | [new-systems.md](gap-analysis/new-systems.md) |
| 33 | [Observability Pipeline](gap-analysis/new-systems.md#33-observability-pipeline-----cw) | CW | [new-systems.md](gap-analysis/new-systems.md) |
| 34 | [Wireclient + Network Chaos Testing](gap-analysis/new-systems.md#34-wireclient--network-chaos-testing-----im) | IM | [new-systems.md](gap-analysis/new-systems.md) |
| 35 | [Discord Notifications](gap-analysis/new-systems.md#35-discord-notifications-----cw) | CW | [new-systems.md](gap-analysis/new-systems.md) |
| 36 | [Tauri Admin App + Tools](gap-analysis/new-systems.md#36-tauri-admin-app--tools-----im) | IM | [new-systems.md](gap-analysis/new-systems.md) |
| 37 | [Ring Transport](gap-analysis/new-systems.md#37-ring-transport-----im) | IM | [new-systems.md](gap-analysis/new-systems.md) |
| -- | [Session Management](gap-analysis/server-infrastructure.md#session-management-----im) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Rate Limiting](gap-analysis/server-infrastructure.md#rate-limiting-----km) | KM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Anti-Cheat Validation](gap-analysis/server-infrastructure.md#anti-cheat-validation-----im-was-km) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Economy Sinks / Faucets](gap-analysis/server-infrastructure.md#economy-sinks--faucets-----im) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [World State Persistence](gap-analysis/server-infrastructure.md#world-state-persistence-----im) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Event / Scheduler System](gap-analysis/server-infrastructure.md#event--scheduler-system-----im) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Admin / GM Tools](gap-analysis/server-infrastructure.md#admin--gm-tools-----im-gm-command-surface-is-cw-admin-panel-and-dot-command-parity-still-im) | IM | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |
| -- | [Metrics / Telemetry](gap-analysis/server-infrastructure.md#metrics--telemetry-----cw) | CW | [server-infrastructure.md](gap-analysis/server-infrastructure.md) |

---

## Summary Completion Matrix

Recomputed 2026-09-27 directly from the feature rows, by script: every matrix row equals the count of its section's feature rows, which are in the area files listed under [Systems by area](#systems-by-area). Change a feature row and its matrix row in the same PR.

| # | System | Total | CW | NT | IM | KM | NU |
|---|--------|-------|----|----|----|----|-----|
| 1 | Authentication and Login | 13 | 8 | 1 | 2 | 2 | 0 |
| 2 | Mercury Protocol | 15 | 10 | 0 | 3 | 2 | 0 |
| 3 | Game Data Pipeline | 9 | 6 | 2 | 0 | 1 | 0 |
| 4 | Database Persistence | 8 | 6 | 0 | 0 | 2 | 0 |
| 5 | Character Creation | 11 | 4 | 4 | 1 | 2 | 0 |
| 6 | World Entry and Spaces | 12 | 7 | 4 | 1 | 0 | 0 |
| 7 | Movement and Navigation | 11 | 1 | 3 | 7 | 0 | 0 |
| 8 | Entity Lifecycle | 10 | 6 | 2 | 1 | 1 | 0 |
| 9 | Combat and Abilities | 26 | 6 | 1 | 15 | 4 | 0 |
| 10 | Effects and Buffs | 13 | 3 | 0 | 5 | 5 | 0 |
| 11 | Stats | 8 | 5 | 0 | 0 | 2 | 1 |
| 12 | Inventory and Items | 15 | 8 | 5 | 1 | 1 | 0 |
| 13 | Missions | 12 | 7 | 0 | 3 | 2 | 0 |
| 14 | Loot | 10 | 4 | 2 | 0 | 4 | 0 |
| 15 | Stores / Vendors | 8 | 1 | 6 | 1 | 0 | 0 |
| 16 | NPC AI and Behavior | 26 | 8 | 6 | 9 | 3 | 0 |
| 17 | Spawn System | 23 | 7 | 0 | 1 | 14 | 1 |
| 18 | XP and Leveling | 12 | 7 | 3 | 1 | 1 | 0 |
| 19 | Crafting | 9 | 0 | 9 | 0 | 0 | 0 |
| 20 | Stargate Travel | 10 | 2 | 4 | 3 | 1 | 0 |
| 21 | Chat | 11 | 0 | 8 | 0 | 3 | 0 |
| 22 | Trading | 8 | 0 | 0 | 8 | 0 | 0 |
| 23 | Organizations / Guilds | 21 | 0 | 18 | 1 | 2 | 0 |
| 24 | Mail | 17 | 0 | 15 | 0 | 2 | 0 |
| 25 | Black Market | 10 | 0 | 0 | 0 | 9 | 1 |
| 26 | Contact Lists | 10 | 10 | 0 | 0 | 0 | 0 |
| 27 | Dueling | 6 | 0 | 0 | 5 | 1 | 0 |
| 28 | Pets | 7 | 0 | 6 | 1 | 0 | 0 |
| 29 | Minigames | 9 | 5 | 0 | 1 | 3 | 0 |
| 30 | Groups / Parties | 7 | 0 | 3 | 1 | 3 | 0 |
| 31 | Content Engine | 11 | 6 | 2 | 1 | 2 | 0 |
| 32 | Mercury Bundle / ChannelBundle | 5 | 5 | 0 | 0 | 0 | 0 |
| 33 | Observability Pipeline | 13 | 10 | 3 | 0 | 0 | 0 |
| 34 | Wireclient + Network Chaos Testing | 7 | 3 | 0 | 3 | 1 | 0 |
| 35 | Discord Notifications | 6 | 6 | 0 | 0 | 0 | 0 |
| 36 | Tauri Admin App + Tools | 13 | 2 | 2 | 6 | 3 | 0 |
| 37 | Ring Transport | 9 | 3 | 4 | 2 | 0 | 0 |
| -- | Session Management | 7 | 0 | 0 | 4 | 3 | 0 |
| -- | Rate Limiting | 6 | 1 | 2 | 0 | 3 | 0 |
| -- | Anti-Cheat Validation | 8 | 1 | 0 | 6 | 1 | 0 |
| -- | Economy Sinks / Faucets | 7 | 0 | 4 | 0 | 3 | 0 |
| -- | World State Persistence | 6 | 1 | 1 | 1 | 3 | 0 |
| -- | Event / Scheduler System | 4 | 0 | 0 | 1 | 3 | 0 |
| -- | Admin / GM Tools | 13 | 4 | 2 | 5 | 2 | 0 |
| -- | Metrics / Telemetry | 9 | 4 | 3 | 2 | 0 | 0 |
| | **TOTALS** | **<!-- gen:gap-count total -->491<!-- /gen:gap-count -->** | **<!-- gen:gap-count CW -->167<!-- /gen:gap-count -->** | **<!-- gen:gap-count NT -->125<!-- /gen:gap-count -->** | **<!-- gen:gap-count IM -->102<!-- /gen:gap-count -->** | **<!-- gen:gap-count KM -->94<!-- /gen:gap-count -->** | **<!-- gen:gap-count NU -->3<!-- /gen:gap-count -->** |

### Summary Percentages

The TOTALS line above and every number in this section are generated from the matrix rows by `tools/docs-gen/regen.py`, which reruns on `main` after every merge; edit the rows, never these numbers. The totals line sums to <!-- gen:gap-count total -->491<!-- /gen:gap-count --> features.

| Status | Count | Percentage |
|--------|-------|-----------|
| Confirmed Working (CW) | <!-- gen:gap-count CW -->167<!-- /gen:gap-count --> | <!-- gen:gap-pct CW -->34.0%<!-- /gen:gap-pct --> |
| Needs Test (NT) | <!-- gen:gap-count NT -->125<!-- /gen:gap-count --> | <!-- gen:gap-pct NT -->25.5%<!-- /gen:gap-pct --> |
| Implemented (IM) | <!-- gen:gap-count IM -->102<!-- /gen:gap-count --> | <!-- gen:gap-pct IM -->20.8%<!-- /gen:gap-pct --> |
| Known/Missing (KM) | <!-- gen:gap-count KM -->94<!-- /gen:gap-count --> | <!-- gen:gap-pct KM -->19.1%<!-- /gen:gap-pct --> |
| Needed/Unknown (NU) | <!-- gen:gap-count NU -->3<!-- /gen:gap-count --> | <!-- gen:gap-pct NU -->0.6%<!-- /gen:gap-pct --> |

**Code exists (CW + NT + IM)**: <!-- gen:gap-count CW+NT+IM -->394<!-- /gen:gap-count --> features (<!-- gen:gap-pct CW+NT+IM -->80.2%<!-- /gen:gap-pct -->)
**Missing (KM + NU)**: <!-- gen:gap-count KM+NU -->97<!-- /gen:gap-count --> features (<!-- gen:gap-pct KM+NU -->19.8%<!-- /gen:gap-pct -->)

**Tested end-to-end (CW)**: <!-- gen:gap-count CW -->167<!-- /gen:gap-count --> features (<!-- gen:gap-pct CW -->34.0%<!-- /gen:gap-pct -->).

### Since 2026-09-25

| | CW | NT | IM | KM | NU | Total |
|---|---:|---:|---:|---:|---:|---:|
| 2026-09-25 | 169 | 58 | 98 | 142 | 4 | 471 |
| 2026-09-27 (social close-out) | 167 | 93 | 108 | 111 | 3 | 482 |
| 2026-09-27 (organizations close-out) | 167 | 110 | 109 | 97 | 3 | 486 |
| 2026-09-27 (bank close-out) | 167 | 112 | 109 | 95 | 3 | 486 |
| 2026-09-27 (crafting close-out) | 167 | 121 | 101 | 94 | 3 | 486 |
| 2026-09-28 (ammo close-out) | 167 | 125 | 102 | 94 | 3 | 491 |
| **Delta** (2026-09-25 to now) | **-2** | **+67** | **+4** | **-48** | **-1** | **+20** |

- **Social systems (mail, chat, 1v1 duels).** The social-systems campaign ([ledger](analysis/social-systems/README.md), PRs #873 to #937) moved Chat to 7 NT / 1 IM / 3 KM (tells, Ignore, flood limit, GM broadcast, GM mute), Mail to 15 NT / 2 KM with four new rows (server-generated mail, GM mail tools, quarantined-mail recovery, vault and organization aliases), and Dueling to 5 IM / 1 KM. Rate Limiting's chat row also covers the mail and duel buckets. Every one of these rows waits on the owner's [SS-UAT](analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release).
- **Other campaigns.** Crafting (CR-07 to CR-09), pets, organizations (ORG-03, ORG-04) and the ability-tree campaign changed feature rows in their sections. Four of those sections had been edited without their matrix row: World Entry (10 → 12 rows), XP and Leveling (11 → 12 rows, two CW rows now NT until the ability-tree UAT), Organizations (15 → 17 rows, 3 NT) and Anti-Cheat (7 → 8 rows). The close-out recount brought those matrix rows back in line with their tables.
- **Organizations (Squads, Teams, Commands).** The organizations campaign ([ledger](analysis/organizations/README.md), PRs #861 to #954) moved §23 from 3 NT / 14 KM to 16 NT / 1 IM / 4 KM, with four new rows (kick, login restore and presence, disband, GM commands); the four KM rows left are the Bank campaign's cash and vault, and the two strike-team responses, which are refused because no strike-team feature exists. §30 now counts the squad as the group (3 NT / 1 IM / 3 KM), and §21's pre-defined channels row is NT now that team, command and officer lines are delivered (ORG-09). Every one of these rows waits on the owner's [ORG-UAT](analysis/organizations/work-packets.md#org-uat-owner-two-client-uat-colo).
- **Bank and vault.** The Bank and Vault campaign ([ledger](analysis/bank-vault/README.md), PRs #860 to #966) moved §23's two Bank rows, the treasury and the organization vault, from KM to NT (§23: 18 NT / 1 IM / 2 KM). The personal bank changed no row count: §12's bag row already counted the bank. Every one of these rows waits on the owner's bank UAT ([checklist](analysis/bank-vault/handoffs/session-resume.md#uat-checklist)).
- **Crafting.** The crafting campaign ([ledger](analysis/crafting/README.md), PRs #851 to #979) moved every §19 row to NT: the eight IM rows (craft, research, reverse engineering, alloy, learning, expertise, paradigms, blueprints) and respec (KM). It also changed rows without moving their status: §14's loot take-all and table content (a refused pickup stays on the corpse; the debug crate drops guides and a Blueprint item, CR-16) and §22's item swap (trade from the crafting bag, CR-17). Every §19 row waits on the owner's [CR-14 UAT](analysis/crafting/handoffs/session-resume.md#cr-14-owner-uat-checklist).
- **Token usage (no row change).** The token-usage campaign ([ledger](analysis/token-usage/README.md), PRs #1122 to #1140, close-out TP-12) built the token profiler in `tools/token-profile/`, which measures what AI-assisted work costs and posts a stats comment on every merged PR. It is development tooling, not a server feature, so it is noted under §36 without a matrix row. The same close-out split this file into the index and the per-area files.
- **Ammo.** The ammo campaign ([ledger](analysis/ammo/README.md), PRs #1040 to #1069, close-out AM-12) added five rows, all new features rather than moves: §9 `Special ammo modifiers and on-hit effects` (IM: penetration is inert while `MITIGATION` is 0, and EMP has no interrupt) and `Support-dart ally shots` (NT); §12 `Special ammo reserve` and `Ammo-type validation` (NT); §14 `Special ammo drops` (NT). No existing row changed status. The feature ships on (`ammo.finite_special`, D-AM11) and awaits the owner's UAT ([unified UAT guide § Special ammo](guides/unified-uat.md#special-ammo)).

### What moved since 2026-07-25

The 2026-07-25 edition's matrix again disagreed with its own tables. Its rows sum to 444 / CW 164 / NT 18 / IM 134 / KM 124 / NU 4, while its totals line printed 443 / 159 / 18 / 134 / 128 / 4. The mismatches were in Authentication, Combat, Loot, Missions, NPC AI, Spawn and Mail. The comparison below is row against row.

| | CW | NT | IM | KM | NU | Total |
|---|---:|---:|---:|---:|---:|---:|
| 2026-07-25 rows | 164 | 18 | 134 | 124 | 4 | 444 |
| 2026-09-25 | 169 | 58 | 98 | 142 | 4 | 471 |
| **Delta** | **+5** | **+40** | **-36** | **+18** | **+0** | **+27** |

The headline percentages barely moved, for two opposite reasons. About 160 PRs landed, most of them tested and merged but not yet run against a live client, so NT roughly tripled. At the same time this pass read the code behind every row and demoted claims that did not hold up. The evidence bar for CW was a written record of an in-client test (playtest report, UAT worknote, PR or issue note).

- **Gained ground.** Character creation `0 CW → 4 CW` (two characters created and played in the 2026-09-18 colo playtest). Minigames `0 → 5 CW` (12 in-client Livewire sessions in the same playtest). Ring transport `7 IM → 3 CW / 4 NT / 2 IM` (four in-client Cellblock ring trips; stall timeouts and the mission 688 client-patch ceremony are new rows). NPC AI `+4 rows` and `+1 CW` (assist aggro, UAT-1 on 2026-09-25); proximity aggro, collision-geometry line of sight and grounding are new NT rows. Loot generation and damage application to CW (colo playtest). Stargate DHD, cancel, address discovery and multi-player sync to NT (#662, #663, #682). Observability and tools `+9 rows` for the `.bug` bookmark, NPC AI telemetry and dashboard, the log index, the UPK patcher (Phase 0 passed in-client 2026-09-19) and the live research lab.
- **Corrected down.** Spawn system `9 CW / 10 IM → 7 CW / 1 IM / 14 KM`: SpawnRegion/SpawnSet activation, population, set cooldowns, weighted tables and level ranges were attributed to `spawner/regions.rs`, which loads client-hinted trigger regions instead (#62). Mail `9 IM → 2 NT / 2 IM / 8 KM`: send, attachments, cash and COD are stubs that log "unimplemented". Economy `5 CW → 0 CW / 4 NT`: mission cash rewards are not paid at all (#310), and vendor rows lost CW when #609 found that earlier vendor testing had routed the vendor window to the mission handler; vendors have not been re-tested since. Effects `-1 CW, +3 KM`: no clear-on-damage, clear-on-revive or clear-on-bandolier-swap flags exist, and permanent vs non-permanent stat tracking does not exist. Database persistence `-1 CW`: there are no compile-time checked `sqlx::query!` macros; all queries are checked at runtime. Combat position/facing checks `IM → KM`. Player position persistence `CW → NT` (#756 found logout never saved position; the fix has no in-client relog test). Tauri tools: JWT auth for remote and the WebSocket entity stream are TODO stubs (`IM → KM`).
- **New rows (+27).** Mostly the systems above, plus Kismet sequence overrides (#755), dialog override patch mode (#767), NPC barks, same-world respawn resync (#756), per-world navmesh containment and coverage, player-to-player introduction (#737), login IP binding (#738), dev-session token quota (#740) and disk-to-SigNoz log parity (#792).

**Rows awaiting a tester.** Each reviewer listed rows that very likely work in-client but have no written test record. They are the fastest way to move NT to CW. The largest groups are the NPC AI changes merged after UAT-1, gate dial/open/cross with a second observer, the dialog-UI buttons and barks, two-client chat and player visibility, relog position, and vendors.

---

## What changed since the previous (deprecated-codebase) gap analysis

| Metric | Audit (Python+C++) | 2026-05-27 rows | 2026-07-25 rows | This pass (2026-09-25) | Why the change |
|---|---:|---:|---:|---:|---|
| Total features tracked | 369 | 428 | 444 | 471 | New rows for NPC AI (aggro, assist, line of sight, grounding), ring transport, observability and tools, navmesh containment, Kismet and dialog overrides |
| Confirmed Working | 31 (8.4%) | 151 (35.3%) | 164 (36.9%) | 169 (35.9%) | Character creation, minigames, ring trips, damage, loot and assist aggro moved in on the 2026-09-18 playtest and UAT-1; vendors, spawn population, economy and effect clear-flags moved out after a code re-read |
| Code exists (CW+NT+IM) | 175 (47.4%) | 260 (60.7%) | 316 (71.2%) | 325 (69.0%) | Needs Test tripled (18 → 58): most of the ~160 PRs since July are merged and tested but not yet run in a client |
| Missing | 194 (52.6%) | 168 (39.3%) | 128 (28.8%) | 146 (31.0%) | Rows that had been credited to the wrong code (spawn population, mail send and attachments, mission cash) were corrected to KM |

Every column is the sum of that edition's own rows, not the headline it printed. The 2026-05-27 totals line said 437 / CW 139 / KM 184 / NU 5, and the 2026-07-25 totals line said 443 / CW 159 / KM 128; neither matched its tables.

The shape of "done" as of this pass: Mercury, observability and the content engine are firmly done. Two zones (Castle Cellblock and Castle) have been played end to end in a client, and a third (Harset) has been rebuilt but not yet played. The NPC AI restoration campaign (NA00-NA33) replaced the aggro, leash, cover and line-of-sight logic, and every world now has a navmesh (#794). The biggest gap has shifted from *missing code* to *missing client tests*: 58 rows are merged and waiting for a tester. The long-tail social systems (organizations, dueling, pets, groups) are still stubs, mail can only read, and the black market is still queued behind an unmerged branch. (2026-09-27: mail, chat and 1v1 duels, pets and squads have since landed server-side; see [Since 2026-09-25](#since-2026-09-25).)

---

## Critical Path for Playability

Re-ranked 2026-09-25.

1. **Client-test the September landings** — 93 rows are NT (2026-09-27): the NPC AI changes merged after UAT-1, gate dial/open/cross with a second observer, the dialog-UI buttons and barks, two-client chat and player visibility, relog position, vendors (untested since #609), and the social-systems rows (mail, chat, duels; [SS-UAT](analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release)). This is the cheapest way to move the headline number
2. **Effect-script content coverage** — the framework works but the long tail of the 3,216 effect rows still needs scripts, and the clear-on-damage / clear-on-revive / clear-on-bandolier-swap flags are not implemented. `cell/effects/scripts.rs` is 1,648 lines
3. **AoI invisible-entity defect** — a witness can be correctly introduced to an entity and still not render it (Castle Cellblock GuardBody corpse). The 2026-09-19 repro put the drop inside the client, after a fully ACKed delivery, and fixed the `OTEL_FILTER` gap that had kept `aoi.create_emit` out of SigNoz. The first-login cinematic hold ships as the experiment on the one remaining lead; this needs an in-game repro to confirm or kill it, not more code
4. **Mission rewards** — the `GrantXP` action exists (#618), but `mission.reward_xp` is 0 in every seed row and the reward formula needs a maintainer decision. Mission cash and item rewards are not dispatched at all (#310)
5. **Crafting UAT** — the crafting campaign is merged: every verb, respec, stations and tools, Blueprint items and guides (§19); nine NT rows wait on the owner's [CR-14 run](analysis/crafting/handoffs/session-resume.md#cr-14-owner-uat-checklist)
6. **Multi-zone end-to-end** — Castle Cellblock and Castle have been played in a client (2026-09-18 colo playtest); Harset is rebuilt but unplayed; the other spaces have navmeshes but no content campaign
7. **Two-client verification** — trading, player-to-player introduction (#737), chat between players, mail and duels have never been exercised with two real clients. The social-systems [SS-UAT](analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) is the script for the last three

Quality-of-life items (organizations, black market, remaining minigame ports, groups) are still gated on the above but each can be picked up independently. GM tooling and contact lists have left this list, and so have mail, chat and 1v1 duels (the social-systems campaign, merged 2026-09-27, awaiting the owner's UAT) and pets (restored server-side 2026-09-27, awaiting the owner's in-game UAT).

---

## Cross-Reference Tables

### Documentation Exists but Rust Doesn't (port pending)

Corrected 2026-07-25 — trading and contact lists have left this table. Dueling left it on 2026-09-27 (social-systems SS-D1 to SS-D3).

| System | Gameplay Doc | Wire Format Doc | Rust Code Status |
|--------|-------------|----------------|-------------------|
| Crafting | crafting-system.md | crafting-wire-formats.md | Ported by the crafting campaign (#427, then #851 to #979); awaiting the owner's CR-14 UAT |
| Organizations | organization-system.md | organization-wire-formats.md | 200 lines stubs — unchanged |
| Black Market | black-market.md | black-market-wire-formats.md | 94 lines stubs on `main`; full Phase 1 waiting on `feat/571-black-market-phase1` |
| Pets | pet-system.md | pet-wire-formats.md | Restored server-side by the pets campaign (PT-E1 to PT-11, #570); owner in-game UAT pending. Not done: persistence (not planned, D-PT01), turrets (no client model), a Lo'taur that heals |
| Groups | group-system.md | group-wire-formats.md | Not ported |

### Rust Code Exists but Doc Lags

These have substantial Rust implementations the per-system docs haven't fully caught up on. P3-equivalent doc-refresh pending.

| System | Code Location | Doc Status |
|--------|--------------|-----------|
| Content Engine | crates/cell-content/src/cell/content/ + crates/content-engine/ | docs/content/content-engine.md is the canonical reference but is currently labelled audience: "engineers" — could use a "what's done vs. planned" callout |
| Mercury Bundle | crates/mercury/src/channel_bundle.rs | docs/architecture/mercury-bundle.md is the ADR |
| Observability | crates/server/, crates/mercury/instrumentation.rs | docs/architecture/observability.md + operations/signoz-*.md |
| Wireclient | crates/wireclient/ | docs/architecture/wireclient.md — **verify this doc's Tier 3 claims**; the crate has no UDP socket (see §34) |
| Discord Notifications | crates/discord/ | docs/architecture/discord-notifications.md |
| Trading | crates/cell-methods/src/cell/cell_methods/player/trade/ + base/world_entry/methods/trade/ | **Added 2026-07-25.** docs/gameplay/trade-system.md still describes Python `Trade.py` as the implementation |
| Ring Transport | crates/cell-content/src/cell/ring_transport/ | **Added 2026-07-25.** About 5,856 lines (3,191 production); docs/gameplay/ring-transport-system.md does not yet cover the mission 688 client-patch route |
| Cover system | crates/cell-cover/src/cell/cover/ | **Added 2026-07-25.** docs/game-systems.md still says "CoverSet entity is a stub" — corrected in that file on 2026-07-25 |
| NPC AI movement states | crates/cell-combat/src/cell/service/npc_ai/ | **Added 2026-07-25.** docs/gameplay/npc-ai.md predates PR #428 |
| GM command surface | crates/cell-console/src/cell/console/gm/ + cell/console/ | **Added 2026-07-25.** About 6,070 + 12,730 lines (89 dot-commands as of 2026-09-25); no consolidated GM command reference |
| Minigame server | crates/minigame/src/minigame/ | **Added 2026-07-25.** docs/gameplay/minigame-system.md still describes an external SmartFox process |
| Movement validation | crates/entity/src/movement_validation/ | **Added 2026-07-25.** Four-layer anti-cheat with no ADR |

### Server-Only Blind Spots (Ranked by Gameplay Impact)

Re-ranked 2026-09-25. #5 (speed-hack detection) is implemented but deliberately warn-only; #7 (mission rewards) moved up after #310 confirmed missions pay neither cash nor items.

| Rank | System | Impact | Status | Notes |
|------|--------|--------|--------|-------|
| 1 | Crafting verbs | HIGH — nothing crafted in a client yet | NT | Every verb, respec, stations, tools and crafting items are server-side complete (crafting campaign, §19); the owner's CR-14 UAT is the next step |
| 2 | AoI entity-introduction drop | HIGH — entities silently invisible | IM | Known-open. Address-gate hypothesis disproved 2026-06-20; Mercury delivery retired 2026-09-19 (every create ACKed first try), so the drop is client-side. `aoi.create_emit` now actually exports to SigNoz. The first-login cinematic hold (#747) is the experiment on the n=1 cinematic lead, and as of 2026-09-25 nobody has recorded an in-game look since it shipped |
| 3 | Organizations / guilds | MEDIUM — no persistent social layer | KM | 200 lines of stubs, no schema |
| 4 | Rate Limiting | MEDIUM — exploitable | KM | Chat, mail sends and duel challenges are limited since the social-systems campaign (SS-00, SS-M1, SS-D1, 2026-09-27); trade requests and login are still unthrottled. Trading shipped without a request cooldown, so this got *worse* |
| 5 | Speed-hack enforcement | MEDIUM — detection lands, action doesn't | IM | Layer is live but warn-only by design pending tolerance calibration from SigNoz |
| 6 | Damage sanity checking | MEDIUM — no max-damage cap | KM | The one anti-cheat layer with no implementation at all |
| 7 | Mission rewards | MEDIUM — missions pay nothing | KM | `GrantXP` action exists (#618) but no seed rows use it and `reward_xp` is 0 everywhere; cash and item rewards are never dispatched (#310) |
| 8 | Multi-zone verification | LOW — 2 zones played in client | NT | Castle Cellblock and Castle played 2026-09-18; Harset unplayed; other spaces have navmeshes only |
| 9 | `sequences_nvp` unread | LOW — cinematic sound-bank / params never reach the client | KM | `db/resources/Events/Seed/sequences_nvp.sql` seeds **2,042 rows** (SoundBankName and friends). No Rust code reads the table, and all six `onSequence` emit sites hardcode a NameValuePairs count of 0: abilities/damage_apply/mod.rs:351, abilities/use_ability/handle.rs:540 and :569, cell/console/net.rs:104, content/executor/mod.rs:122, ring_transport/wire_helpers.rs:42 |

---

## Related Documents

- [project-status.md](project-status.md) — human-readable summary of this analysis
- [gap-analysis/](gap-analysis/) — the per-area files with each system's feature table (index: [Systems by area](#systems-by-area))
- [../README.md](../README.md) — high-level project status
- [gameplay/](gameplay/) — per-system gameplay docs
- [content/](content/) — content audit + content engine
- [protocol/](protocol/) — wire formats
- [architecture/](architecture/) — server architecture and ADRs
- [reverse-engineering/](reverse-engineering/) — RE findings + Ghidra work
- [../CONTRIBUTING.md](../CONTRIBUTING.md) — how to pick a feature and ship it
