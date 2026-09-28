# Rust Gameserver Dev Memory

One line per topic file; the detail lives in the file. Keep hooks short (this index must stay under ~17 KB).

## Build environment

- [build-environment.md](build-environment.md) — rust-lld override obsolete; worktrees need the `external/` junction; a hung test looks like a hung build (check `UserModeTime`).
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` the old `%TEMP%` lane exited 0 on a failed cargo (the committed `tools/build-lane/lane.sh` passes the status through).
- [mutation-restore-mtime-trap.md](mutation-restore-mtime-trap.md) — restoring from a backup copy leaves an old mtime; cargo keeps the mutated build. Touch restored files.
- [dependency-dedupe-blockers.md](dependency-dedupe-blockers.md) — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- [services-split-extraction-traps.md](services-split-extraction-traps.md) — extracting a crate from services: allowlist edges, unreachable_pub, privacy errors in phases, test/guard blind spots.

## Working environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — other sessions on the repo: work in `.claude/worktrees/<slug>/`, junction `external/`.
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha may not be an ancestor.
- [rebase-keep-both-regex-drops-braces.md](rebase-keep-both-regex-drops-braces.md) — scripted "keep both" conflict fixes can drop a `}` mid-hunk; inspect + `cargo check` before `--continue`.
- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unverified` commit may not compile; its tests encode the starting design; port hunks by hand.
- [crate-split-extraction-traps.md](crate-split-extraction-traps.md) — moving code out of services: `pub(crate)` turns dead, `unreachable_pub` hits pub fields, layering-guard globs, the live-DB crate list.
- [pre-split-branch-port-traps.md](pre-split-branch-port-traps.md) — porting a June branch onto the split: pull cell-visible types into wire (no sqlx there), OTEL row per new wire module, taken seed ids.

## Tooling quirks

- [offline-client-event-trace-and-udp-port-trap.md](offline-client-event-trace-and-udp-port-trap.md) — no Ghidra: client Lua + PE bytes + RTTI name the CME event a handler raises.
- [mail-escrow-lock-order-and-proof-traps.md](mail-escrow-lock-order-and-proof-traps.md) — inventory lock order is advisory → item row → sgw_player.
- [python-write-mangles-utf8-and-crlf.md](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF.
- [i686-test-exe-uac-installer-detection.md](i686-test-exe-uac-installer-detection.md) — a 32-bit test exe named `*patch*` fails with os error 740 under UAC.
- [rustfmt-trailing-line-comment-quirk.md](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column; add a blank line.
- [rustfmt-reorders-mod-declarations.md](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module.md](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last; clippy 1.98+ wants `as_chunks::<2>()` over `chunks_exact(2)`.
- [tooling-filter-and-path-traps.md](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets; `gh -F body=@file` needs a Windows path.
- [sqlx-dynamic-sql-string.md](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`; share SELECTs with `macro_rules!` + `concat!`.
- [sqlx-chain-id-is-i32-vacuous-guards.md](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs.md](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`; check with `git check-ignore -v`.
- [worktree-shell-and-external-binary-tests.md](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`, heredoc appends and chained commits (use scratch scripts).

## Wire format

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — client- vs cell-method dispatch tables are different files; GM tail is `109 + K`.
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — `cell/client_methods/` is authoritative; `mercury::method_idx` is a drifted partial copy.
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns bytes consumed: `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — a bind pushes only `InteractionType`; method 104 is never emitted.
- [cooked-pak-and-dialog-override-traps.md](cooked-pak-and-dialog-override-traps.md) — `data/cache/*.pak` IS in git; fail-closed patcher vs seed linter diverge silently.
- [cooked-override-client-cache-persistence.md](cooked-override-client-cache-persistence.md) — the client caches pushed overrides on disk; removed ids are never evicted.
- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — four method-28 serializers.
- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — two `witness_entity_method` fns; idbase 61 player / 62 NPC matters for index >= 61.
- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — `direction` is `[pitch, yaw, roll]` radians for all entities; `[i8; 3]` param zeroes facing.
- [game-clock-and-timer-expiry-tests.md](game-clock-and-timer-expiry-tests.md) — client clock is ticks / hertz; expiries = `game_time_secs() + d`; settle the clock past its epoch in tests.

## Injected client DLLs

- [injected-dll-unwind-and-lua-error-rules.md](injected-dll-unwind-and-lua-error-rules.md) — `thiscall-unwind` detours for C++-EH prologues.
- [client-patch-send-natives-traps.md](client-patch-send-natives-traps.md) — startEntityMessage sends even offline; microseh masks the ABI; no cpcall around C-function args; iced-x86 for SGW.exe.
- [telemetry-anchor-audit-and-hookgate.md](telemetry-anchor-audit-and-hookgate.md) — telemetry anchors never ran and 5 were wrong (IAT hint/name RVAs, COL-shifted vtable, `ret 8` exec thunks); offline check recipe; hookgate install lock.
- [injector-bitness-and-start32-helper.md](injector-bitness-and-start32-helper.md) — x64 launcher injects via the i686 sgw-start32 helper; the WOW64 resolver fails on suspended targets.

## UE3 packages and navmesh

- [ue3-absent-property-defaults.md](ue3-absent-property-defaults.md) — an absent tagged property is the SGW class default (`Terrain.DrawScale3D` = 100).
- [ue3-staticmesh-extraction.md](ue3-staticmesh-extraction.md) — `bCollideActors=false` still carries kDOP; prefab archetype chains; match dotted Outer paths.
- [ue3-bsp-model-decode.md](ue3-bsp-model-decode.md) — empty Model = 108 bytes; BSP winding is opposite StaticMesh; Castle brush Models decode to stubs.
- [ue3-prefab-rig-anatomy.md](ue3-prefab-rig-anatomy.md) — component props start at byte 8; prefab meshes on imported archetypes; `.umap` is LZO.
- [navbuilder-obj-interop.md](navbuilder-obj-interop.md) — NavBuilder exits 0 on axis order, CRLF, winding and stray `*.obj` failures. Read before the OBJ writer.
- [navbuilder-obj-traps.md](navbuilder-obj-traps.md) — UE3->BW is `v x z y`; CRLF mandatory; stray non-`<hex8>o.obj` reads garbage bounds.
- [castle-staticmesh-coverage.md](castle-staticmesh-coverage.md) — Castle interior floors are BSP, not StaticMesh.
- [navmesh-recast-and-castle-topology.md](navmesh-recast-and-castle-topology.md) — Recast's unchecked 24-bit span index is the real `cs` floor.
- [navmesh-probe-and-bsp-traps.md](navmesh-probe-and-bsp-traps.md) — `NavGraph::locate` false negatives on stacked meshes; validate BSP filters on a second map.
- [harset-nav-does-not-cover-upper-quarters.md](harset-nav-does-not-cover-upper-quarters.md) — harset.nav covers only the plaza.
- [navmesh-containment-modes.md](navmesh-containment-modes.md) — per-world `navmesh_mode`; `TEST_SPACES_XML` pins space ids; `get_nearest_point` echoes on a miss.
- [npc-ground-clamp-and-detour-traps.md](npc-ground-clamp-and-detour-traps.md) — `moveAlongSurface` output is unprojected; build.rs never rebuilt `detour_wrapper.cpp`.
- [obj-slab-and-nav-inspect-probe-traps.md](obj-slab-and-nav-inspect-probe-traps.md) — obj_slab chunk pre-filter; the top up-facing surface may be the roof.
- [navmesh-onmesh-assertions-are-weak.md](navmesh-onmesh-assertions-are-weak.md) — `is_point_valid` and `find_path` pass on the wrong component.
- [map-data-placement-toolkit.md](map-data-placement-toolkit.md) — deriving spawn coordinates from a cooked map; heading = atan2(dx, dz).
- [telemetry-last-valid-is-mostly-synthetic.md](telemetry-last-valid-is-mostly-synthetic.md) — 77% of Harset `last_valid_*` rejects are (0,0,0).
- [occluder-sizing-and-los-truth.md](occluder-sizing-and-los-truth.md) — NA27 occluder paging; build-determinism and grazing-ray traps.

## Seeds and content chains

- [entity-template-seed-authoring.md](entity-template-seed-authoring.md) — one ability per set; faction 10 is immutable; no components + no mesh = invisible.
- [cover-seed-ids-and-orient-convention.md](cover-seed-ids-and-orient-convention.md) — cover set ids `world*100000+n`; cover `orient` is not entity yaw.
- [seed-name-id-and-asset-naming.md](seed-name-id-and-asset-naming.md) — new `texts.sql` moniker ids never render; monikers name UE3 asset families.
- [content-engine-condition-gotchas.md](content-engine-condition-gotchas.md) — a rejected condition row UNGATES its chain; world ids live in `resources.worlds`.
- [cell-startup-caches-vs-base-roundtrip.md](cell-startup-caches-vs-base-roundtrip.md) — the cell has a DB pool and ~20 caches; no base round-trips mid-chain.
- [content-chain-authoring-traps.md](content-chain-authoring-traps.md) — `display_dialog` needs an interact; nothing respawns; zero-baseline rule for interaction flags.
- [content-chain-dispatch-traps.md](content-chain-dispatch-traps.md) — `dialog_choice` has no archetype; button-less dialogs still fire it; negatives pass vacuously.
- [content-chain-condition-context-gaps.md](content-chain-condition-context-gaps.md) — `archetype neq` fails open on dialog chains; `delay_ms > 0` queues.
- [player-loaded-edge-trigger-race.md](player-loaded-edge-trigger-race.md) — a gated `player_loaded` chain never fires for a player already inside; add a state-change trigger.
- [edge-trigger-replay-and-abandon.md](edge-trigger-replay-and-abandon.md) — H52 `enter_region` replay and H54 `mission_abandoned` wiring.
- [chain-replay-trigger-param-vacuity.md](chain-replay-trigger-param-vacuity.md) — a `TriggerEvent` missing its key param matches nothing.
- [dialog-set-bind-routing-and-edges.md](dialog-set-bind-routing-and-edges.md) — `target_id` is a dialog_set_MAP id; a bind fans to every entity of the template.
- [dialog-button-strip-and-seed-agreement.md](dialog-button-strip-and-seed-agreement.md) — linter floors block the packet that changes them; roster pins for patch tests.
- [container-capacity-and-grant-targets.md](container-capacity-and-grant-targets.md) — raising a `bag_max_slots` arm opens loot/content grants into it (`container_sets[1]`, 752 items prefer 17).
- [inventory-lock-keys-and-failure-injection.md](inventory-lock-keys-and-failure-injection.md) — inventory writers use different lock keys (grants merge stacks under `(player, container)`), so a read-then-send must row-lock.
- [org-cash-balance-and-overflow-traps.md](org-cash-balance-and-overflow-traps.md) — KEY SHARE does not freeze `naquadah` (use RETURNING).
- [move-path-lock-layers-and-vault-verdict.md](move-path-lock-layers-and-vault-verdict.md) — moveItem has three lock layers (strip all in a concurrency revert proof).
- [debug-hub-npc-authoring-traps.md](debug-hub-npc-authoring-traps.md) — Vendor interaction was never set (now derived at spawn).
- [pet-template-seed-traps.md](pet-template-seed-traps.md) — NoPetLeveling freezes a pet at template level; summons need an event set; pet kits are silent no-ops (NA43 allowlist).
- [trainer-seed-and-gm-grant-traps.md](trainer-seed-and-gm-grant-traps.md) — trainer_abilities.sql is generated; capstones need .giveability; grants persist via base; pets 350-359 vs NPCs 360-369.

## Base sessions

- [mail-expiry-and-notify-seams.md](mail-expiry-and-notify-seams.md) — every mail writer sets `expires_at`; `NOT quarantined` on every player path; system-mail callers call `notify`; ordered-gate race tests.
- [connected-map-view-over-parallel-index.md](connected-map-view-over-parallel-index.md) — online lookups: a view over `connected` + `listed_online`, not a parallel map.

- [trade-results-client-ignores-space-cash.md](trade-results-client-ignores-space-cash.md) — client trade window acts only on results 1/2; space/cash codes 3-6 show nothing, so send a feedback line.
- [tell-channel-and-ignore-copies.md](tell-channel-and-ignore-copies.md) — client /tell is byte 10 (CHAN_TELL since SS-C4); the Ignore list has 3 copies synced by one resync; 0xBD decode for method idx >= 61.
- [chat-channel-client-display.md](chat-channel-client-display.md) — server 8 opens a modal prompt, 9 is plain feedback, 7 shows nothing (nil ChannelMap); built-in onChatJoined creates user channels.

## Cell systems

- [grant-placement-and-loot-handback-traps.md](grant-placement-and-loot-handback-traps.md) — grants re-placed by container_sets on the base.
- [grant-paths-pick-different-containers.md](grant-paths-pick-different-containers.md) — gmGiveItem grants to bag 1; loot and content grant_item use the first `container_sets` entry (17 for crafting items).
- [mail-placement-rule-and-fixture-types.md](mail-placement-rule-and-fixture-types.md) — send, take and pay-COD share `take::carried_bag`; lowest item id 10 is `{2}`, so never pick fixtures by lowest id.
- [per-session-player-state-lifecycle.md](per-session-player-state-lifecycle.md) — a CellEntity field dies on every space change/logout by construction.
- [ability-event-sets-are-server-only.md](ability-event-sets-are-server-only.md) — ability event sets never reach the client (seed-only wiring); most mob kits deal 0 damage.
- [client-action-bar-is-client-side.md](client-action-bar-is-client-side.md) — hotbar bindings are a client Lua saved var; server "hotbar" = `onKnownAbilitiesUpdate`.
- [npc-range-gate-and-weapon-range-columns.md](npc-range-gate-and-weapon-range-columns.md) — four item range columns; range gated in two places; bogus melee `max_range`.
- [npc-ai-fight-test-fixtures.md](npc-ai-fight-test-fixtures.md) — `make_ai_fixture` has no navmesh; assert the INFO log, not `nav_path`.
- [npc-detector-telemetry-traps.md](npc-detector-telemetry-traps.md) — AI-path statics race across tests (use task_local); release detector state on destroy.
- [npc-class-filter-and-dead-target-traps.md](npc-class-filter-and-dead-target-traps.md) — `all_npc_entity_ids` is mob-only; HEALTH alone is not dead.
- [ai-state-private-and-revert-proof-mtime.md](ai-state-private-and-revert-proof-mtime.md) — write `ai_state` via `npc_ai::set_ai_state`; revert proofs must `touch` restored files.
- [no-movement-type-wire-and-nav-path-writers.md](no-movement-type-wire-and-nav-path-writers.md) — no movement-type wire exists; nav_path writes go through `movement_stop`.
- [mission-persist-hydrate-roundtrip.md](mission-persist-hydrate-roundtrip.md) — one serializer, one hydrator; roster rebuilt from `mission_objectives`.
- [stargate-address-book-three-legs.md](stargate-address-book-three-legs.md) — three copies (DB, cell, client); a grant needs client method 66.
- [cell-mirrors-of-base-owned-counters.md](cell-mirrors-of-base-owned-counters.md) — base-owned counters must be messaged to the cell.
- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat in `StatList` may have no reader; the dirty-publish pattern.
- [ring-transport-fsm.md](ring-transport-fsm.md) — `disconnect_entity` vs `destroy_entity`; `BSF_*` bits are ref-counted.
- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the back half; fake default-instance mechanisms.
- [session-scoped-cell-state-hooks.md](session-scoped-cell-state-hooks.md) — per-session cell state: key by player_id, tear down on DisconnectEntity only (DestroyEntity = gate travel), replay on InitPlayerState.
- [revert-proof-commit-first.md](revert-proof-commit-first.md) — commit before a revert-proof run; git checkout -- <dir> also wipes uncommitted work; heredoc apostrophe trap.
- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `destroy_entity` sends no LeftAoI; use `despawn_npc` for visible removals.
- [effect-scripts-run-after-the-death-check.md](effect-scripts-run-after-the-death-check.md) — `abilities::death::resolve_death` is the only kill path.
- [kill-credit-seams-and-loot-ownership.md](kill-credit-seams-and-loot-ownership.md) — XP decided in `grant_kill_xp`, mission credit via `credited_player` (4 callers); corpses have no loot owner.
- [throttle-key-hides-transitions.md](throttle-key-hides-transitions.md) — key throttles by `(entity_id, kind)`; `destroy_space` is a second teardown path.
- [npc-caster-player-ordered-gates.md](npc-caster-player-ordered-gates.md) — an NPC casting on a player's order skips #444, fire_los and the warmup re-check; add them.
- [native-consumable-round-trip.md](native-consumable-round-trip.md) — item heals/stims: consume-first cell->base->cell round trip (not outbox); stat-buff ledger across 4 crates; flush only expired stats.
- [ability-launch-fire-split.md](ability-launch-fire-split.md) — AT-10: handle_use_ability is launch-only; damage may fire a tick later via fire.rs; ground-AoE tests need class_id 0x04.
- [npc-ai-tick-snapshot-and-hash-order.md](npc-ai-tick-snapshot-and-hash-order.md) — the AI tick's state snapshot goes stale inside a tick; NPCs visit in HashMap order, so multi-NPC tests flake under nextest.
- [crafting-verb-traps.md](crafting-verb-traps.md) — crafting verbs: component sets are subsets (match designs exactly); don't hold a craft to its named instances; `&Completion` across await is not Send.
- [pet-owner-lifecycle-hooks.md](pet-owner-lifecycle-hooks.md) — PT-02: every GateTravel/TeleportPlayer site calls a pets hook (scan-guarded); owner gets no LeftAoI on travel.
- [crafting-induction-engine-seams.md](crafting-induction-engine-seams.md) — crafting engine: global registry + drop hooks.
- [cimmeria-side-flag-bits-collide-with-client-enums.md](cimmeria-side-flag-bits-collide-with-client-enums.md) — check `enumerations.xml` before inventing a flag bit; AF_CHANNEL_ALLOWS_MOVEMENT was SpeedPet.
- [crafting-verb-packet-traps.md](crafting-verb-packet-traps.md) — a new crafting verb breaks stub-pinning dispatch tests in base-world-entry; `handle_<verb>_in` test shape; alloy page's 10-slot cap.
- [org-vault-storage-and-lock-order.md](org-vault-storage-and-lock-order.md) — org vault items are their own table; lock order advisory, KEY SHARE player, lock_org, rows; never resend the routed org.
- [owner-pet-effects-and-passives.md](owner-pet-effects-and-passives.md) — self casts apply no effects; pulse_count=1 buffs never register; passives need 3 seams; [0,0] stat bounds.
- [duel-end-paths-and-travel-scan.md](duel-end-paths-and-travel-scan.md) — SS-D3: travel sites need `duel::on_travel` (scan test); clamp HEALTH first, end the duel last in a damage resolution.
- [black-market-escrow-and-authority.md](black-market-escrow-and-authority.md) — listed items live in container 18 (exclude it from client reads); BM lock order; 62-64 gated on a cell session.
- [bm-settlement-mail-traps.md](bm-settlement-mail-traps.md) — BM-02b: status gate before any mail (writer mints every call); quarantine = status 4; last_auction_of and unused-bind mutation traps.

## Observability

- [observability-test-and-throttle-traps.md](observability-test-and-throttle-traps.md) — counters unobservable in tests; LogThrottle guards; `create_entity` leaves `is_player=false`.
- [log-filter-parity-traps.md](log-filter-parity-traps.md) — `EnvFilter::new` drops a bad directive silently; custom targets leave their file.
- [tracing-span-fields-not-on-log-records.md](tracing-span-fields-not-on-log-records.md) — span fields are not flattened onto OTLP log records.

## Testing patterns

- [cargo-test-vs-nextest-flakiness.md](cargo-test-vs-nextest-flakiness.md) — `cargo test -p cimmeria-services` has order-dependent failures; validate with nextest.
- [db-test-revert-verification.md](db-test-revert-verification.md) — split DB code into a pure helper + shell; revert seed guards in place.
- [bincode-persisted-cache-format.md](bincode-persisted-cache-format.md) — bincode 2 needs `config::legacy()`; the wrong config decodes silently.
- [live-db-scratch-cluster.md](live-db-scratch-cluster.md) — `db.bat init` loads nothing; scratch Postgres recipe on :5544.
- [chain-replay-executor-guards.md](chain-replay-executor-guards.md) — run `execute_actions`, not just `resolve_event`; `0x7000_5000` reserved.
- [local-postgres-port.md](local-postgres-port.md) — probe port and DB name first; on a wrong one live-DB tests skip green.
- [test-file-split-without-touching-mod-rs.md](test-file-split-without-touching-mod-rs.md) — `tests.rs` -> `tests/mod.rs` needs no parent edit.
- [revert-test-restore-crlf-trap.md](revert-test-restore-crlf-trap.md) — restore with `git checkout HEAD -- <file>` between revert tests.
- [revert-verification-checkout-wipes-uncommitted.md](revert-verification-checkout-wipes-uncommitted.md) — scope restores to one file; checkpoint per packet.
- [revert-verification-loses-uncommitted-fmt.md](revert-verification-loses-uncommitted-fmt.md) — run `cargo fmt` before a WIP checkpoint.
- [revert-proof-mutation-must-be-confirmed.md](revert-proof-mutation-must-be-confirmed.md) — a failed scripted mutation reports every guard "ok"; confirm it applied, never split on `=>`.
- [vacuous-guard-and-sentinel-collision-review.md](vacuous-guard-and-sentinel-collision-review.md) — review checklist: vacuous guards, fixtures that fail two rules, `0x7000_xxxx` collisions.
- [interact-range-and-logcapture-traps.md](interact-range-and-logcapture-traps.md) — `get_entity` spans all spaces, so proximity gates need a space check; LogCapture tests flake under threaded cargo test.
- [test-session-packets-are-encrypted.md](test-session-packets-are-encrypted.md) — TestTransport packets are encrypted (zero key) and feedback lines need player_entity_id; decrypt before grepping text.
- [wireclient-passive-session-dies.md](wireclient-passive-session-dies.md) — a listen-only `GameSession` is reaped at 60 s; send an unreliable AUTHENTICATE heartbeat, as `sparbot::run` does.
- [live-db-lock-race-tests.md](live-db-lock-race-tests.md) — a lock-race test must see the waiter blocked first.

## Campaign judgment

- [legacy-command-parity-scoping-judgment.md](legacy-command-parity-scoping-judgment.md) — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — the egui-only dependabot PR is a no-op; the eframe PR carries the breakage.
- [training-points-cache-absolute-write.md](training-points-cache-absolute-write.md) — `handle_grant_xp` writes training_points absolutely from the session cache; every other TP writer must refresh it.

## Testing patterns

- [forced-db-race-share-lock.md](forced-db-race-share-lock.md) — deterministic type-5 live-DB race: hold `LOCK TABLE ... IN SHARE MODE`, release when 2 sessions are held by the gate (pg_blocking_pids).
- [training-points-cache-absolute-write.md](training-points-cache-absolute-write.md) — `handle_grant_xp` writes training_points absolutely from the session cache.
