# Rust Gameserver Dev Memory

One line per topic file; the detail lives in the file. Keep hooks short (this index must stay under ~17 KB).

## Build environment

- [build-environment.md](build-environment.md) — rust-lld override obsolete; worktrees need the `external/` junction.
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` the old `%TEMP%` lane exited 0 on a failed cargo.
- [mutation-restore-mtime-trap.md](mutation-restore-mtime-trap.md) — restoring from a backup copy leaves an old mtime; cargo keeps the mutated build. Touch restored files.
- [dependency-dedupe-blockers.md](dependency-dedupe-blockers.md) — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- [services-split-extraction-traps.md](services-split-extraction-traps.md) — extracting a crate from services: allowlist edges, unreachable_pub, privacy errors in phases.

## Working environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — other sessions on the repo: work in `.claude/worktrees/<slug>/`, junction `external/`.
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha may not be an ancestor.
- [rebase-keep-both-regex-drops-braces.md](rebase-keep-both-regex-drops-braces.md) — scripted "keep both" conflict fixes can drop a `}` mid-hunk; inspect + `cargo check` before `--continue`.
- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unverified` commit may not compile; its tests encode the starting design; port hunks by hand.
- [crate-split-extraction-traps.md](crate-split-extraction-traps.md) — moving code out of services: `pub(crate)` turns dead, `unreachable_pub` hits pub fields.
- [pre-split-branch-port-traps.md](pre-split-branch-port-traps.md) — porting a June branch onto the split: pull cell-visible types into wire (no sqlx there).

## Tooling quirks

- [offline-client-event-trace-and-udp-port-trap](offline-client-event-trace-and-udp-port-trap.md) — no Ghidra: client Lua + PE bytes + RTTI name the CME event a handler raises.
- [mail-escrow-lock-order-and-proof-traps](mail-escrow-lock-order-and-proof-traps.md) — inventory lock order is advisory → item row → sgw_player.
- [python-write-mangles-utf8-and-crlf](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF.
- [i686-test-exe-uac-installer-detection](i686-test-exe-uac-installer-detection.md) — a 32-bit test exe named `*patch*` fails with os error 740 under UAC.
- [rustfmt-trailing-line-comment-quirk](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column.
- [rustfmt-reorders-mod-declarations](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last.
- [tooling-filter-and-path-traps](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets.
- [sqlx-dynamic-sql-string](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`.
- [sqlx-chain-id-is-i32-vacuous-guards](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`.
- [worktree-shell-and-external-binary-tests](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`, heredoc appends, chained commits.

## Wire format

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — client- vs cell-method dispatch tables are different files; GM tail is `109 + K`.
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — `cell/client_methods/` is authoritative; `mercury::method_idx` is a drifted partial copy; `def_conformance` guards both (#801).
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns bytes consumed: `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — a bind pushes only `InteractionType`; method 104 is never emitted.
- [cooked-pak-and-dialog-override-traps.md](cooked-pak-and-dialog-override-traps.md) — `data/cache/*.pak` IS in git; fail-closed patcher vs seed linter diverge silently.
- [cooked-override-client-cache-persistence.md](cooked-override-client-cache-persistence.md) — the client caches pushed overrides on disk; removed ids are never evicted.
- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — four method-28 serializers.
- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — two `witness_entity_method` fns; idbase 61 player / 62 NPC matters for index >= 61.
- [request-ammo-change-instance-id.md](request-ammo-change-instance-id.md) — method 42 `ItemId` is the weapon instance id; match `instance_id`, whitelist by the slot's design id (#534).
- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — `direction` is `[pitch, yaw, roll]` radians for all entities; `[i8; 3]` param zeroes facing.
- [login-handshake-acks-and-pre-channel-sends.md](login-handshake-acks-and-pre-channel-sends.md) — seqs 1/2 in the TX window (#842); clients ack them in 3 of 5 captures; testing a pre-channel drop.
- [game-clock-and-timer-expiry-tests.md](game-clock-and-timer-expiry-tests.md) — client clock is ticks / hertz; expiries = `game_time_secs() + d`.
- [cooked-data-full-resync.md](cooked-data-full-resync.md) — #840: paced full resync (RequiredUpdates=0), misses jump the stream, Play held for 6 no-miss-path categories.
- [cooked-item-additions-shape.md](cooked-item-additions-shape.md) — real shipped COOKED_ITEM shape (not alphabetical); new ids via ITEM_ADDITIONS; AmmoType_Icons = EAmmoType labels.

## Injected client DLLs

- [injected-dll-unwind-and-lua-error-rules.md](injected-dll-unwind-and-lua-error-rules.md) — `thiscall-unwind` detours for C++-EH prologues.
- [client-patch-send-natives-traps.md](client-patch-send-natives-traps.md) — startEntityMessage sends even offline; microseh masks the ABI; no cpcall around C-function args.
- [telemetry-anchor-audit-and-hookgate.md](telemetry-anchor-audit-and-hookgate.md) — telemetry anchors never ran and 5 were wrong (IAT hint/name RVAs, COL-shifted vtable.
- [injector-bitness-and-start32-helper.md](injector-bitness-and-start32-helper.md) — x64 launcher injects via the i686 sgw-start32 helper; the WOW64 resolver fails on suspended targets.

## UE3 packages and navmesh

- [ue3-and-navmesh-index](ue3-and-navmesh-index.md) — sub-index: UE3 package decode, BSP/StaticMesh, NavBuilder OBJ, Recast/Detour, map placement, occluders.

## Seeds and content chains

- [seeds-and-content-chains-index](seeds-and-content-chains-index.md) — sub-index: template/cover/name seeds, chain conditions and edge triggers, dialog binds, inventory locks, pet and trainer seeds.

## Base sessions

- [mail-expiry-and-notify-seams.md](mail-expiry-and-notify-seams.md) — every mail writer sets `expires_at`; `NOT quarantined` on every player path.
- [connected-map-view-over-parallel-index.md](connected-map-view-over-parallel-index.md) — online lookups: a view over `connected` + `listed_online`, not a parallel map.
- [trade-results-client-ignores-space-cash.md](trade-results-client-ignores-space-cash.md) — client trade window acts only on results 1/2; space/cash codes 3-6 show nothing, so send a feedback line.
- [tell-channel-and-ignore-copies.md](tell-channel-and-ignore-copies.md) — client /tell is byte 10 (CHAN_TELL since SS-C4); the Ignore list has 3 copies synced by one resync.
- [chat-channel-client-display.md](chat-channel-client-display.md) — server 8 opens a modal prompt, 9 is plain feedback, 7 shows nothing (nil ChannelMap).

## Cell systems

- [per-shot-damage-seam-is-damage-apply.md](per-shot-damage-seam-is-damage-apply.md) — per-shot modifiers hook damage_apply, not effect scripts; MITIGATION cap 0 makes armour inert.

- [grantitem-overcap-and-process-wide-gm-switches.md](grantitem-overcap-and-process-wide-gm-switches.md) — GrantItem over-cap row (#1045), use return_rounds; player_id-keyed GM switches in cimmeria-entity.
- [grant-placement-and-loot-handback-traps.md](grant-placement-and-loot-handback-traps.md) — grants re-placed by container_sets on the base.
- [grant-paths-pick-different-containers.md](grant-paths-pick-different-containers.md) — gmGiveItem grants to bag 1; loot and content grant_item use the first `container_sets` entry.
- [mail-placement-rule-and-fixture-types.md](mail-placement-rule-and-fixture-types.md) — send, take and pay-COD share `take::carried_bag`.
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
- [session-scoped-cell-state-hooks.md](session-scoped-cell-state-hooks.md) — per-session cell state: key by player_id, tear down on DisconnectEntity only.
- [revert-proof-commit-first.md](revert-proof-commit-first.md) — commit before a revert-proof run; git checkout -- <dir> also wipes uncommitted work.
- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `destroy_entity` sends no LeftAoI; use `despawn_npc` for visible removals.
- [effect-scripts-run-after-the-death-check.md](effect-scripts-run-after-the-death-check.md) — `abilities::death::resolve_death` is the only kill path.
- [kill-credit-seams-and-loot-ownership.md](kill-credit-seams-and-loot-ownership.md) — XP decided in `grant_kill_xp`, mission credit via `credited_player` (4 callers).
- [throttle-key-hides-transitions.md](throttle-key-hides-transitions.md) — key throttles by `(entity_id, kind)`; `destroy_space` is a second teardown path.
- [npc-caster-player-ordered-gates.md](npc-caster-player-ordered-gates.md) — an NPC casting on a player's order skips #444, fire_los and the warmup re-check; add them.
- [native-consumable-round-trip.md](native-consumable-round-trip.md) — item heals/stims: consume-first cell->base->cell round trip (not outbox); stat-buff ledger across 4 crates; flush only expired stats.
- [ammo-reserve-round-trip.md](ammo-reserve-round-trip.md) — AM-02: base loop is sequential, so flush then trust the weapon row; load rounds at draw commit, not in the tick.
- [ability-launch-fire-split.md](ability-launch-fire-split.md) — AT-10: handle_use_ability is launch-only; damage may fire a tick later via fire.rs; ground-AoE tests need class_id 0x04.
- [npc-ai-tick-snapshot-and-hash-order.md](npc-ai-tick-snapshot-and-hash-order.md) — the AI tick's state snapshot goes stale inside a tick; NPCs visit in HashMap order, so multi-NPC tests flake under nextest.
- [crafting-verb-traps.md](crafting-verb-traps.md) — crafting verbs: component sets are subsets (match designs exactly); don't hold a craft to its named instances; `&Completion` across await is not Send.
- [pet-owner-lifecycle-hooks.md](pet-owner-lifecycle-hooks.md) — PT-02: every GateTravel/TeleportPlayer site calls a pets hook (scan-guarded); owner gets no LeftAoI on travel.
- [live-loot-containers-and-tag-state.md](live-loot-containers-and-tag-state.md) — open_loot rolls per player_id on a live chest; once flags in sgw_player.looted_containers; entity_tag_state reads live_tags.
- [crafting-induction-engine-seams.md](crafting-induction-engine-seams.md) — crafting engine: global registry + drop hooks.
- [cimmeria-side-flag-bits-collide-with-client-enums.md](cimmeria-side-flag-bits-collide-with-client-enums.md) — check `enumerations.xml` before inventing a flag bit; AF_CHANNEL_ALLOWS_MOVEMENT was SpeedPet.
- [crafting-verb-packet-traps.md](crafting-verb-packet-traps.md) — a new crafting verb breaks stub-pinning dispatch tests in base-world-entry.
- [org-vault-storage-and-lock-order.md](org-vault-storage-and-lock-order.md) — org vault items are their own table; lock order advisory, KEY SHARE player, lock_org, rows.
- [owner-pet-effects-and-passives.md](owner-pet-effects-and-passives.md) — self casts apply no effects; pulse_count=1 buffs never register; passives need 3 seams.
- [duel-end-paths-and-travel-scan.md](duel-end-paths-and-travel-scan.md) — SS-D3: travel sites need `duel::on_travel` (scan test).
- [black-market-escrow-and-authority.md](black-market-escrow-and-authority.md) — listed items live in container 18 (exclude it from client reads); BM lock order.
- [bm-settlement-mail-traps.md](bm-settlement-mail-traps.md) — BM-02b: status gate before any mail (writer mints every call); quarantine = status 4.
- [deployable-pulse-and-seed-traps.md](deployable-pulse-and-seed-traps.md) — `apply_damage_to_target` registers every pulsing effect of its def; DeploymentBar flag is not a spawn marker; templates 200-409 taken.
- [ammo-on-hit-effect-needs-a-script.md](ammo-on-hit-effect-needs-a-script.md) — ammo on-hit effects need a script_name or the hit pulse never fires; no Radioactive dart toggle exists.

- [stored-target-lifetime-and-gm-view-check](stored-target-lifetime-and-gm-view-check.md) — #844 clears current_target_id; GM targets must be in view.
- [cell-systems-index](cell-systems-index.md) — sub-index: grants and loot, per-session state, abilities and effects, NPC AI, missions, pets, crafting, black market, duels, respawn and re-create.

## Observability

- [observability-test-and-throttle-traps](observability-test-and-throttle-traps.md) — counters unobservable in tests.
- [log-filter-parity-traps](log-filter-parity-traps.md) — `EnvFilter::new` drops a bad directive silently.
- [tracing-span-fields-not-on-log-records](tracing-span-fields-not-on-log-records.md) — span fields are not flattened onto OTLP log records.

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
- [interact-range-and-logcapture-traps.md](interact-range-and-logcapture-traps.md) — `get_entity` spans all spaces, so proximity gates need a space check; LogCapture cargo-test flake fixed in #891.
- [test-session-packets-are-encrypted.md](test-session-packets-are-encrypted.md) — TestTransport packets are encrypted (zero key) and feedback lines need player_entity_id; decrypt before grepping text.
- [wireclient-passive-session-dies.md](wireclient-passive-session-dies.md) — a listen-only `GameSession` is reaped at 60 s; send an unreliable AUTHENTICATE heartbeat, as `sparbot::run` does.
- [live-db-lock-race-tests.md](live-db-lock-race-tests.md) — a lock-race test must see the waiter blocked first.
- [forced-db-race-share-lock.md](forced-db-race-share-lock.md) — deterministic type-5 live-DB race with no code hook: hold `LOCK TABLE ... IN SHARE MODE`, release once `pg_stat_activity` shows N lock waiters

## Campaign judgment

- [legacy-command-parity-scoping-judgment](legacy-command-parity-scoping-judgment.md) — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — the egui-only dependabot PR is a no-op; the eframe PR carries the breakage.
- [training-points-cache-absolute-write.md](training-points-cache-absolute-write.md) — `handle_grant_xp` writes training_points absolutely from the session cache.
- [loot-seed-pins-and-grant-stack-cap.md](loot-seed-pins-and-grant-stack-cap.md) — new loot rows break exact pins on tables 3/7/8/9 (filter by NOT EXISTS); GrantItem does not cap at max stack.
