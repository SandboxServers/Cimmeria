---
title: "Gap Analysis: Server Infrastructure, Cross-Cutting"
type: explanation
audience: engineers
last_updated: 2026-10-03
companion_docs:
  - ../gap-analysis.md
  - ../project-status.md
---

# Gap Analysis: Server Infrastructure, Cross-Cutting

> Part of the [Gap Analysis](../gap-analysis.md), split out of it on 2026-10-03 with no change to any row. The status taxonomy, the evidence bar and the Summary Completion Matrix are in the main file; each matrix row counts the feature rows of its section here, so change both together.

## Server Infrastructure (Cross-Cutting)

### Session Management --- IM

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Documentation**: [architecture/server-infrastructure-proposals.md](../architecture/server-infrastructure-proposals.md) §1 (session-resume design), [protocol/login-handshake.md](../protocol/login-handshake.md) (cross-IP session binding). [architecture/server-systems.md](../architecture/server-systems.md) is the superseded survey this section replaced.
- **Rust code**: `crates/base/src/base/connect_loop/` (per-client lifecycle), `crates/base/src/base/login/mod.rs` (Phase 3 + duplicate eviction), `crates/base-session/src/base/tick_sync.rs`, `crates/auth/src/auth/`
- **Recent PRs**: #711 (inactivity constants split, closes #293), #738 (login sessions bound to issuing IP, warn-only, closes #442), #698 (stop logging full SIDs / tickets / SOAP body), #756 (position persisted on logout)
- **Open issues**: #460 (security audit CAT-A, auth / session / character lifecycle)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Inactivity timeout | IM | -- | mercury/lib.rs:135, base/tick_sync.rs:84 | Two layers since #711: `MERCURY_PEER_DEAD_MS = 300_000` (Mercury peer-dead bookkeeping) and a 60 s tick-sync client reap. `UE3_INACTIVITY_TIMEOUT_MS = 15_000` documents the client-side tolerance only. Faster reaping is an open owner decision. Re-verified 2026-09-25 |
| Duplicate login check | IM | -- | base/login/mod.rs:89-134 | **Corrected 2026-09-25.** Runs at Phase 3 login, not character select: an existing session for the same account gets `LOGGED_OFF` and its entities are destroyed with reason `duplicate_login` |
| Developer mode bypass | IM | -- | auth/handlers.rs:129, 172 | **Corrected 2026-09-25 (was CW).** `developer_mode` skips the protocol-digest check and accepts credentials without a DB. It does **not** bypass duplicate-login eviction: base/login/mod.rs:89-134 never reads it, although the config doc comment promises "multi-login" (common/src/config.rs:97). No written test record backed the CW |
| Reconnection grace period | KM | -- | -- | Instant disconnect = session lost |
| Session token persistence | KM | -- | -- | No resume after network blip |
| Continuous auth validation | KM | -- | -- | Only at login |
| Login session IP binding | IM | -- | auth/mod.rs, auth/handlers.rs, base/login/mod.rs | **New 2026-09-25.** #738: `SessionRecord` / `PendingLogin` record the issuing IP; Phase 2 SID and Phase 3 ticket consumption log `session_ip_mismatch` / `ticket_ip_mismatch` WARNs. **Warn-only by design** until NAT false positives are measured, so nothing is refused yet |

### Rate Limiting --- KM

- **Confidence**: HIGH (code searched 2026-09-25: no chat, action, trade or login throttle anywhere in `crates/services`; chat limit added 2026-09-27 by SS-00)
- **Recent PRs**: #740 (dev-session token mint and refresh quotas, closes #441)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Ability cooldown enforcement | CW | -- | cell/abilities/ | Per-ability timers |
| Chat flood protection | NT | -- | base-session/src/base/rate_limit/, base/src/base/dispatch/chat.rs | **New 2026-09-27 (SS-00).** Base-side token bucket (burst 5, 1/s) before the cell forward, not the cell-tick design in `server-infrastructure-proposals.md` §2, and enforced from day one (D-SS14). The same `rate_limit` module holds the mail-send (burst 3, 1/10 s) and duel-challenge (burst 2, 1/15 s) buckets, wired by SS-M1 and SS-D1 (see §24 and §27). No in-client test on record |
| Action throttling | KM | -- | -- | No per-action rate tracking |
| Trade request spam | KM | Trade | -- | No request cooldown |
| Login attempt limiting | KM | -- | -- | No brute-force protection on the auth service |
| Dev-session token mint quota | NT | -- | admin-api/src/routes/dev_session/quota.rs, handlers.rs | **New 2026-09-25.** #740: per-IP and per-`install_id` fixed-window mint quotas and a per-IP refresh quota (429 + `Retry-After`), bounded refresh chain, `telemetry.write` scope check. Handler-level tests; not exercised by a launcher session on record |

### Anti-Cheat Validation --- IM (was KM)

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Recent PRs**: #437 / #478 (four-layer movement validation), #643 (jump apex no longer trips navmesh containment), #644 (snap-back rubber-band loop replaced by Rejected / Recovered / CorrectionSuppressed with a 5-correction budget; player validation honours `movementSpeedMod`), #639 (`onPhysics` fly/ghost GM bypass), #700 / #726 (diagnosed and throttled navmesh rejects), #741 (dead actors rejected before interaction dispatch), #791 (item use refused while `BSF_DEAD`)

Four layers of server-authoritative movement validation landed in PRs #437 and #478; see §7 for the detail. The 2026-09-18 colo playtest exercised them with the real client: 756 `movement.speed_warning` rows and a jump/snap-back defect, fixed by #643 and #644 ([appendix-session-timeline.md](../analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) line 124). The remaining gap is damage-side sanity checking.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Position bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB from the loaded navmesh with a generous fallback for navmesh-less spaces; catches NaN / infinity / Z-floor-clip |
| Ability target validation | IM | -- | cell/abilities/use_ability/handle.rs:182-260 | Target exists + alive; faction gate added by #444. Adjacent dead-actor gates on interaction (#741) and item use (#791) |
| Inventory ownership check | CW | -- | base/world_entry/methods/inventory | Live-DB regression guards |
| Speed hack detection | IM | -- | entity/movement_validation/mod.rs:18-22, 173 | Server-monotonic-clock `dt`, `top_speed × SPEED_WARN_TOLERANCE (1.5)`, now scaled by `movementSpeedMod` (#644). **Warn-only by design** until SigNoz telemetry calibrates the threshold |
| Teleport detection | IM | -- | entity/movement_validation/mod.rs:24-29 | Hard reject on the dual distance-AND-implied-speed gate. Recovery path rewritten by #644 after the playtest rubber-band loop; not re-tested in client since |
| Damage sanity check | KM | -- | -- | No max-damage cap |
| Action-at-distance exploit | IM | -- | cell/abilities/use_ability/handle.rs:238-262 | `useAbility` rejects targets beyond the ability's `max_range` (30.0 default) with `OutsideWeaponRange`. Player-side LOS is still *not* checked on this path; the #797 occluder LoS serves NPC AI only |
| Ability-trainer authority | IM | -- | ability_tree/gates/trainer.rs; cell/interactions/trainer_authority.rs; cell/cell_methods/player/vendor/train_feedback.rs | **New 2026-09-26 (AT-04).** `trainAbility` used to train from anywhere. It now needs a pinned, live trainer whose list offers the node to the archetype, within `interact_target_in_range`. Rejections send `onErrorCode` (6/9/167/43) and then re-send `onTrainerOpen`. `interact_target_in_range` also rejects a target in another space, which closes the same hole for `interact`. Unit and wire tests only; not client-validated, and whether the client renders `onErrorCode` is unresolved (AT-E1 Q2) |

### Economy Sinks / Faucets --- IM

Re-read 2026-09-25. Every vendor-priced sink and faucet depends on a store window that could not open in-client until PR #609 (2026-07-26), and nothing has been client-tested since, so those rows drop to NT. Mission cash rewards do not exist at all (#310).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Vendor buy/sell prices | NT | Vendors | base/world_entry/methods/vendor/ | **Demoted 2026-09-25 (CW → NT).** Static from DB; server-side live-DB + PL/pgSQL coverage only. PR #609 voids pre-2026-07-26 client observations of the store. Re-verified 2026-09-25 |
| Mission cash rewards | KM | Missions | -- | **Demoted 2026-09-25 (CW → KM).** No code grants cash on mission completion: the content executor has no cash action (only `GrantXP`, cell/content/executor/mod.rs:512), `chosenRewards` (CM 87) is a stub, nothing sends `onMissionRewardsDisplay`, and every seeded mission has `reward_naq = 0`. Open issue #310, re-verified by a comment on 2026-09-25: "Rewards still never dispatch". Re-verified 2026-09-25 |
| Loot cash drops | NT | -- | cell/interactions/loot/mod.rs | **Demoted 2026-09-25 (CW → NT)** to match §14 "Cash drops", which is the same code path. Naquadah rolls are recorded in the 2026-09-18 playtest; its arrival in the wallet is not. Re-verified 2026-09-25 |
| Repair costs | NT | Vendors | vendor/paid_repair/ | **Demoted 2026-09-25 (CW → NT).** Cost formula is covered by live-DB tests only; §15 already rates repair NT; #609 voids earlier client tests. Re-verified 2026-09-25 |
| Recharge costs | NT | Vendors | vendor/paid_recharge/ | **Demoted 2026-09-25 (CW → NT).** Same reasoning as Repair costs. Re-verified 2026-09-25 |
| AH listing fees | KM | Black Market | -- | -- |
| Cash flow tracking | KM | -- | -- | No currency ledger, counter or metric in `crates/`; design only ([server-infrastructure-proposals.md](../architecture/server-infrastructure-proposals.md) §5) |

### World State Persistence --- IM

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Recent PRs**: #756 (position persisted on logout), #663 (stargate open/cross events, transient only)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Player position persistence | NT | -- | base/world_entry/cell_dispatch/position.rs | **Demoted 2026-09-25 (was CW).** #756 records that before it, logout never persisted position: only gate travel and GM teleport wrote `sgw_player.pos_*`, so returning characters spawned at the last gate arrival. #756 adds `CellToBaseMsg::PersistPosition` on disconnect with live-DB tests. No in-client relog test on record. The same PR notes an open "returning character hangs on world load" report |
| Cell event outbox | CW | -- | base/outbox/ | Durable Base→Cell |
| Space scripts | IM | -- | content-engine | Reset on restart |
| Gate state persistence | KM | DB | -- | Open/closed not saved; the #663 stargate events and 4 s dial timer are in-memory only |
| Door state persistence | KM | DB | -- | Not saved |
| World state table | KM | DB | -- | No sgw_world_state |

### Event / Scheduler System --- IM

- **Confidence**: HIGH (code searched 2026-09-25: no cron or global scheduler in `crates/`)

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Per-entity timers | IM | -- | content-engine | Per-chain timers wire through |
| Global event scheduler | KM | -- | -- | No cron-like system |
| Daily resets | KM | Scheduler | -- | -- |
| Holiday events | KM | Scheduler | -- | -- |

### Admin / GM Tools --- IM (GM command surface is CW; admin panel and dot-command parity still IM)

- **Confidence**: HIGH (code re-read 2026-09-25)
- **Documentation**: [analysis/legacy-command-parity/](../analysis/legacy-command-parity/) (README, audit, work-packets), [tools/admin-api.md](../tools/admin-api.md)
- **Rust code**: native GM methods `cell/console/gm/` (6,070 lines); GM dot-console `cell/console/` (12,730 lines, 89 registered dot commands, up from 71 at the 2026-09-16 audit baseline); `crates/admin-api/`
- **Recent PRs**: #609 (GM gate extended to the minigame debug quartet, CM 20-23), #635-#640 and #642 (legacy dot-command parity packets P01-P05, P08, P18, P26, P44-P47), #644 (case-insensitive world lookup, `.gotospace`, account identity on console logs), #749 (`.summon` always brings the player to the caller), #787 (`.aggro` toggle), #676 (`.bug` playtest bookmark; rejected console commands now logged), #724 (admin API binds loopback by default), #740 (dev-session quotas)
- **Open issues**: #439 (admin API has no authentication; the loopback bind is step 1 only), #473 (security audit CAT-N, 40 GM findings)

The GM command surface shipped in June via the client's **native `/` console**: the `SGWGmPlayer` class flip (PR #473, merged in #518 on 2026-06-17) makes a GM enter the world as entity class `0x03`, which unlocks the client's built-in GM command tail. Owner-confirmed working 2026-06-20. The **legacy dot-command parity campaign** has integrated 12 of 49 P packets (P01-P05, P08, P18, P26, P44-P47); P49 is implemented and awaiting UAT; P06, P07 and P48 are Ready; the other P packets wait on dependencies and all 14 G design groups (G01-G14) are BlockedDesign ([work-packets.md](../analysis/legacy-command-parity/work-packets.md)). None of the six milestone UATs (M1-M6) has run. A GM mute (`.mute` / `.unmute`) landed with the social-systems campaign (SS-C3); ban is still missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Admin API (REST) | IM | -- | crates/admin-api/ | Loopback bind by default (#724); still **no authentication** (#439 open), so it must not be published |
| Tauri admin panel | IM | -- | tools/ | Per-page features partial |
| Native GM console (SGWGmPlayer) | CW | -- | cell/console/gm/ | 6,070 lines across give / stats / missions / travel / spawn / query / world / feedback + tests. PRs #473 / #516 / #518 / #521 / #524. Owner-confirmed 2026-06-20 |
| Access level system | CW | -- | cell/dispatch/gm_gate.rs | `enforce_gm_gate` refuses the whole gated method range; #609 added the minigame debug methods 20-23 to the allow-list. Owner-confirmed 2026-06-20 |
| Python console | KM | -- | -- | C++ console not ported (intentional security) |
| Console commands | IM | -- | crates/commands/ | Generic command framework (registry / parser / permissions). **Not** the active dot roster: the live path is cell/console/chat/ → cell/console/ (legacy-command-parity README, "Architecture Guardrails") |
| Dev/authoring `.`-console | IM | -- | cell/console/ | 12,730 lines, 89 registered dot commands. 12 of 49 parity packets integrated, milestone UATs pending. In-client record: the 2026-09-18 playtest ran `.speed`, `.gotoxyz`, `.location` and `.searchmission` successfully (12 of 20 accepted; appendix-session-timeline.md lines 76, 83, 123, 148). Re-verified 2026-09-25 |
| Player info lookup | IM | -- | admin-api/routes/players.rs, cell/console/query.rs | Plus `gmShowPlayer` / `gmUsers` / `testLOS`, and `.info` / `.players` (P02, P04; `.players` now CellApp-wide) |
| Ban/mute system | KM | -- | -- | No `GM_BAN` / `GM_MUTE` index and no ban anywhere in `crates/`; the admin API has a `/players/{id}/kick` route only. A GM chat mute exists as `.mute` / `.unmute` (SS-C3, see §21 "Mute system"), held in memory and not saved across a restart |
| Teleport command | CW | -- | cell/console/gm/travel.rs, cell/console/travel/ | Native `gmGotoXYZ` / `gmGoto` / `gmSummon` / `gmGotoLocation` / `gmDHD`, plus the dot commands `.gotoxyz` / `.goto` / `.summon` / `.gotolocation` / `.gotospace` (P26, P44-P46, #644, #749). `.gotoxyz` confirmed in the 2026-09-18 colo playtest (appendix-session-timeline.md line 148, "Works as designed"). Re-verified 2026-09-25 |
| Item grant | CW | -- | cell/console/gm/give.rs | `gmGiveItem`, alongside give-xp / give-cash / remove-item / give-expertise / give-ASP; base-side confirmation. Owner-confirmed 2026-06-20. Dot `.giveitem` (P06) not yet built |
| Action logging | NT | -- | cell/console/dispatch.rs:47-116, cell/playtest_friction.rs | **Promoted 2026-09-25 (was IM).** Accepted commands log with `account_id` / `player_id` / `access_level` (#644) and relay to the Discord GM channel; rejections (unknown command, argc, bad target) now log with a `reason` (#676), closing playtest gap G7. The accepted-command audit reconstructed the 2026-09-18 playtest; rejection logging not yet seen in a session |
| Announcement broadcast | NT | -- | cell/console/gm/shout.rs | `/gmshout` and `.announce` (SS-C2); see §21 "GM broadcast" |

### Metrics / Telemetry --- CW

- **Confidence**: HIGH (code and PRs re-read 2026-09-25)
- **Documentation**: [architecture/observability.md](../architecture/observability.md), [operations/telemetry.md](../operations/telemetry.md), [operations/npc-ai-telemetry-runbook.md](../operations/npc-ai-telemetry-runbook.md), [analysis/playtests/2026-09-18-colo-castle/telemetry-design.md](../analysis/playtests/2026-09-18-colo-castle/telemetry-design.md)
- **Rust code**: `crates/server/src/otel.rs`, `crates/server/src/logging/` (mod, filters, parity_tests, target_scan_tests), `crates/mercury/src/instrumentation.rs`, `crates/cell-world/src/cell/playtest_friction.rs`, `crates/cell-console/src/cell/console/bookmark.rs`, `crates/cell-world/src/cell/service/npc_ai/detectors/`
- **Recent PRs**: #676 / #678 / #679 / #680 (`.bug` bookmark, stuck-player detectors, outbound-intent logging, per-player journal), #700 / #726 (navmesh observability), #776 / #781 (NPC AI state transitions and detectors), #782 (NPC AI SigNoz dashboard + 9 saved views), **#792 (NA25: disk-to-SigNoz log parity, `cimmeria-trace` index, sampled firehoses)**, #620 (client-telemetry DLL observes dropped inbound methods; not runtime-verified)
- **In-client record**: [npc-ai-restoration/worknotes/uat-1.md](../analysis/npc-ai-restoration/worknotes/uat-1.md) (owner session 2026-09-25 on the colo) was reconstructed from colo SigNoz filtered on `service.version` plus the owner's `.bug` notes. It found that `.bug` bookmarks listed no witnesses (fixed in #791) and that the colo still reports `deploy_env = dev` until the compose file is re-applied.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Category logging | CW | -- | tracing crate, server/logging/ | -- |
| OTLP export | CW | -- | server/otel.rs, server/logging/mod.rs | One `log_provider` helper since #792 |
| Mercury packet metrics | CW | -- | mercury/instrumentation.rs | -- |
| Player count tracking | IM | -- | admin-api/ | Exposed via REST |
| Performance metrics | IM | -- | tracing + SigNoz | Visible in dashboards |
| Custom dashboards | NT | -- | SigNoz, docs/operations/signoz/ | **Promoted 2026-09-25 (was IM).** #782 created the 11-panel "Cimmeria — NPC AI health" dashboard and 9 saved views, with re-importable JSON exports. The PR says most panels populate only after a colo session on an NA00/NA02 build; no record of the dashboard being read after UAT-1 |
| Cloudflare-Access remote ops | CW | -- | operations/signoz-remote-access.md | -- |
| Playtest `.bug` bookmark + stuck-player detectors | NT | -- | cell/console/bookmark.rs, cell/playtest_friction.rs | **New 2026-09-25.** #676 / #678: `.bug <note>` freezes a `playtest.bookmark` snapshot of the scene; once-per-episode `playtest.friction` detectors. The owner used `.bug` in UAT-1 (uat-1.md), which found the witness list empty; #791 fixed that, not re-tested |
| Disk-to-SigNoz log parity | NT | -- | server/logging/ | **New 2026-09-25.** #792 (NA25): a TRACE-only `cimmeria-trace` index, prime-N sampled firehoses (`sampled_1_in` / `suppressed`), 14 hidden DEBUG targets exported, `launcher.key_dump` pinned off. Parity and target-scan guards; not yet observed on the colo |
