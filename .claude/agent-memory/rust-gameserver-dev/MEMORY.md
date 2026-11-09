# Rust Gameserver Dev Memory

One line per topic file; a sub-index's files are also named on its "Files" line. Stays under 200 lines and 25,000 bytes (memory_lint index_over); a new file listed in a sub-index also goes on that sub-index's Files line, or the lint flags it unindexed.

## Build environment

- build-environment.md — rust-lld override obsolete; worktrees need the `external/` junction; PowerShell `bash` may be WSL.
- stale-branch-clippy-toolchain-drift.md — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- lane-sh-masks-cargo-exit-code.md — `lane.sh` / `live-db-test.sh` the old `%TEMP%` lane exited 0 on a failed cargo.
- mutation-restore-mtime-trap.md — restoring from a backup copy leaves an old mtime; cargo keeps the mutated build. Touch restored files.
- dependency-dedupe-blockers.md — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- services-split-extraction-traps.md — extracting a crate from services: allowlist edges, unreachable_pub, privacy errors in phases.
- launcher-core-path-include-extraction.md — #[path] to crate: cfg(test) items need a `test-fixtures` feature; telemetry moved whole; ignored tests hide from nextest list.
- desktop-engine-tests-need-cargo-test.md — nextest cannot list the desktop engine (harness=false PASS lines); use `cargo test` via the lane.

## Working environment

- concurrent-claude-sessions.md — other sessions on the repo: work in `.claude/worktrees/<slug>/`, junction `external/`.
- shared-scratchpad-name-collisions.md — sibling workers share the scratchpad; prefix script names with the packet id.
- stacked-branch-rebase-traps.md — a handed-down base sha may not be an ancestor.
- shepherd-merge-traps.md — clean merges of main can duplicate a doc section or index line; const-slice ptr::eq fails on i686.
- rebase-keep-both-regex-drops-braces.md — scripted "keep both" conflict fixes can drop a `}` mid-hunk; inspect + `cargo check` before `--continue`.
- resuming-a-dead-workers-wip.md — a `wip(...) unverified` commit may not compile; its tests encode the starting design; port hunks by hand.
- crate-split-extraction-traps.md — moving code out of services: `pub(crate)` turns dead, `unreachable_pub` hits pub fields.
- pre-split-branch-port-traps.md — porting a June branch onto the split: pull cell-visible types into wire (no sqlx there).

## Tooling quirks

- tooling-quirks-index.md — sub-index: rustfmt/clippy/sqlx traps, worktree shell limits, client file case, patchset/ring rigs, Python UTF-8.
  - Files: bash-heredoc-backslash-and-metric-tests.md, client-file-case-and-stock-listing.md, clippy-items-after-test-module.md, gitignore-swallows-new-dirs.md, i686-test-exe-uac-installer-detection.md, mail-escrow-lock-order-and-proof-traps.md, offline-client-event-trace-and-udp-port-trap.md, patchset-supersede-and-restore-to-stock.md, powershell-ctrl-c-test-pipeline-stopped.md, powershell-dotnet-relative-path-trap.md, powershell-git-commit-stdin-trap.md, powershell-like-backtick-and-injected-call-tests.md, powershell-strict-mode-empty-index.md, python-write-mangles-utf8-and-crlf.md, ring-rig-clone-to-new-map.md, rustfmt-reorders-mod-declarations.md, rustfmt-skips-sigil-tracing-macros.md, rustfmt-trailing-line-comment-quirk.md, sqlx-chain-id-is-i32-vacuous-guards.md, sqlx-dynamic-sql-string.md, tooling-filter-and-path-traps.md, worktree-shell-and-external-binary-tests.md

## Wire format

- gm-kill-target-sentinel.md — #1727: stock `/gmkilltarget <id>` sends method 190 INT64 -1; select the NPC first; explicit wire IDs still take precedence.
- mercury-aoi-intro.md — first-login AoI NPC record is two messages (11-byte create, 26-byte avatar); one-message packets avoid later request-cursor collision, while queued TX entries retain ACK tracking beyond 32.
- error-strings-served-from-pak-not-seed.md — client error text = served ErrorStrings.pak (216), not the seed; patch Text or add the entry in attribute_patches.
- client-error-codes-and-ammo-chatter.md — create-failed codes are served ErrorStrings.pak ids, not error_texts; check the PAK text; "out of ammo" is the client reacting to AmmoSlotN = 0.
- wire-names-generated-tables.md — `cimmeria_wire::names` is generated from defs (regen cmd), doc-checked; naming rules for unknown entity types.
- gm-tail-dispatch-doc-filename-trap.md — client- vs cell-method dispatch tables are different files; GM tail is `109 + K`.
- method-idx-duplicate-table-drift.md — `cell/client_methods/` is authoritative; `mercury::method_idx` is a drifted partial copy; `def_conformance` guards both (#801).
- read-wstring-offset-semantic.md — `read_wstring` returns bytes consumed: `offset += n`, never `offset = n`.
- dialog-set-bind-carries-no-dialog-id.md — a bind pushes only `InteractionType`; method 104 is never emitted.
- cooked-pak-and-dialog-override-traps.md — `data/cache/*.pak` IS in git; fail-closed patcher vs seed linter diverge silently.
- cooked-override-client-cache-persistence.md — the client caches pushed overrides on disk; removed ids are never evicted.
- gm-feedback-cell-base.md — four method-28 serializers.
- witness-entity-method-dual-fn.md — two `witness_entity_method` fns; idbase 61 player / 62 NPC matters for index >= 61.
- request-ammo-change-instance-id.md — method 42 `ItemId` is the weapon instance id; match `instance_id`, whitelist by the slot's design id (#534).
- cell-entity-direction-semantics.md — `direction` is `[pitch, yaw, roll]` radians for all entities; `[i8; 3]` param zeroes facing.
- login-handshake-acks-and-pre-channel-sends.md — seqs 1/2 in the TX window (#842); clients ack them in 3 of 5 captures; testing a pre-channel drop.
- game-clock-and-timer-expiry-tests.md — client clock is ticks / hertz; expiries = `game_time_secs() + d`.
- cooked-data-full-resync.md — #840: paced full resync (RequiredUpdates=0), misses jump the stream, Play held for 6 no-miss-path categories.
- cooked-item-additions-shape.md — real shipped COOKED_ITEM shape (not alphabetical); new ids via ITEM_ADDITIONS; AmmoType_Icons = EAmmoType labels.
- client-gm-slash-xml-and-dhd-address-resolution.md — /gm* need InternalSlashCommands.xml; property 7 = command mask; updateStargateAddress resolves async (open DHD shows "Unknown").
- ability-telemetry-coverage-gate.md — AB-C7: script set + four scanned code tables + client declaration; AB-C6 timing tables are process-global.
- praxis-wire-script-traps.md — Praxis pistol is a chain-1005 grant; ability arrays hash-ordered; trail drowns in 0x10; movie/arrival race.
- entity-tint-packed-colours.md — onEntityTint args are packed 0xRRGGBB__ colours; seed bigints = signed i32, send low 32 bits; opt-in send_tint.

## Lab supervisor

- lab-small-prop-hover-offsets.md — entity hover search tries actor origin last for small props; avatar proximity stays on humanoid heights.
- client-interact-firer-by-id.md — `FUN_00def4b0` cdecl(entity_id) fires interact; lab `client_interact` calls it (N3), live wire unverified.

- lab-template-id-mismatch.md — `unitMobId` is not the server template id; join server entity snapshots to the client map by entity id for template and life-state selectors

- lab-console-tee-deferred-reply.md — lab tee sees only sync lines; base round-trip results park the reply on a table (CH-13 spawn batches, batch id on GmSpawnNpc).
- lab-chaos-net-shim-generation-and-teardown.md — net shim keys on per-set generation; teardown discards held/delayed sends; bounded line; paused-clock tests.
- client-lua-cegui-global-subscribe.md — client Lua `window:subscribeGlobal(ns, ev, 'Fn')`; no unsubscribe, so once per Lua state; args (this, eventWindow, ...).
- lab-cli-child-test-daemon-isolation.md — command tests in a child pwsh reach the real daemon (User-scope token): use -StatusFile + closed-port labd.pid.
- lab-native-cegui-input-and-levels.md — cursor/drag via native CEGUI injectors; `cegui_fake.rs` test seam; new native-level words need a `from_reported` row.
- stacked-pr-ship-title-and-ab-lab-tools.md — ship.py mistitles stacked PRs; AB-T5 snapshot builder, lab dummy = AI-skip extension, cooldown clear is type 2.
- lab-uat-runner-in-process-tool-calls.md — call rmcp tools by name in-process via the RequestContext extractor; chat marks before the action; capability table.
- lab-daemon-lease-gate.md — lease gate is hand-written call_tool; new tools need a policy line; runner touches its own lease; per-test lease books.
- ability-uat-staging-limits.md — warmup and cleanse rows need .dummy caster (#1188); 1462/4306/2827 are effect ids; one graded press per row.
- uat-runner-grade-neutral-capture.md — reasons name step indices; opt-in capture records go to the row at the end, re-indexed (GD-06).
- uat-run-dir-sibling-json-trap.md — `RunDir::rows` parses every json beside the row JSON (skip `.snapshot.json`); attest re-grade must re-apply post-grade rules.
- lab-mcp-raw-args-and-live-db-crate.md — rmcp arg-parse errors skip audit (raw Value + delegated schema); first live-DB test in a crate: dev-dep, shim last, test-live-db lists.
- lab-calibrate-sweep-test-seams.md — runner fakes are private: write a geometry fake; paused time makes each stand cost 250 ms.
- lab-daemon-cwd-is-state-dir.md — installed daemon cwd = %LOCALAPPDATA%\cimmeria-lab; checkout walk-ups are inert, search env then state dir first.
- lab-chaos-net-shim.md — CH-06: shared `lossy_transport::filter`, ChaosTransport in base, recv drop-only, has_net guard, fake-wire tests.
- lab-chaos-arm-by-account-traps.md — CH-14: promote at playCharacter before RESET_ENTITIES; no grouped lab_chaos imports even in tests; feature-only live-DB tests never run.
- lab-mcp-feature-gated-tool-router.md — feature-gated lab-mcp tools: second `#[tool_router(router, vis)]` block in the submodule, `+` merge in new().
- lab-record-tap-fixture-and-recorder-seams.md — tap fixtures need a cell msg_id + method_index for decode_call; Recorder test seams.
- lab-record-click-sampling-traps.md — parsed specs resolve @aliases; toml writes newline strings as `"""`; clicks and calls land a tick apart.
- lab-console-tee-misses-async-spawn-ids.md — server_console_exec never returns spawned npc ids (async, after the reply); no despawn-by-id command.
- lab-cli-json-capture-and-parallel-stubs.md — uat.ps1 -Json goes to [Console]::Out (SetOut to capture); stubs don't reach -Parallel lanes.
- lab-flow-first-live-run-findings.md — busy client after Create = bridge `dispatch timeout`; only SelfStatusWin is the HUD; outcome reads `ability.*`.

## Launcher

- launcher-index.md — sub-index: desktop launcher IPC/schema/adoption traps, release feeds, telemetry tailer, extracted mtimes, bootstrap install.
  - Files: bootstrap-launcher-install-and-test-server-port.md, desktop-repair-update-terminal-hooks.md, desktop-settings-panel-ipc-traps.md, launcher-extracted-mtimes-and-ue3-ini-version.md, launcher-native-owner-handle.md, launcher-preferences-schema-change-fanout.md, launcher-release-feed-traps.md, launcher-self-update-handoff-traps.md, launcher-state-key-and-egui-wake.md, launcher-telemetry-tailer-traps.md, launcher-windows-adoption-path-traps.md

## Injected client DLLs

- injected-client-dlls-index.md — sub-index: detours and unwind, CME handler ABI, lab bridge/watchdog, telemetry anchors, injector bitness.
  - Files: cargo-artifact-hardlink-cp-trap.md, client-handler-abi-and-static-disassembly.md, client-patch-send-natives-traps.md, client-unit-slots-and-actor-pose.md, cme-registry-is-a-factory-not-subscribe.md, dll-boot-testhost-traps.md, entity-method-stream-is-memory-ostream.md, injected-dll-unwind-and-lua-error-rules.md, injector-bitness-and-start32-helper.md, lab-event-store-and-ui-lua-hooks.md, lab-probe-traffic-starves-watchdog.md, lab-ui-reader-lua-traps.md, lab-watchdog-load-grace-main-thread-cpu.md, offline-disasm-and-minhook-detour-tests.md, telemetry-anchor-audit-and-hookgate.md, telemetry-dll-token-never-rewritten.md

## UE3 packages and navmesh

- ue3-and-navmesh-index.md — sub-index: UE3 package decode, BSP/StaticMesh, NavBuilder OBJ, Recast/Detour, map placement, occluders.
  - Files: castle-staticmesh-coverage.md, harset-nav-does-not-cover-upper-quarters.md, kismet-designer-events-and-door-state.md, map-data-placement-toolkit.md, navbuilder-obj-interop.md, navbuilder-obj-traps.md, navmesh-containment-modes.md, navmesh-onmesh-assertions-are-weak.md, navmesh-probe-and-bsp-traps.md, navmesh-recast-and-castle-topology.md, npc-ground-clamp-and-detour-traps.md, obj-slab-and-nav-inspect-probe-traps.md, occluder-sizing-and-los-truth.md, telemetry-last-valid-is-mostly-synthetic.md, ue3-absent-property-defaults.md, ue3-bsp-model-decode.md, ue3-prefab-rig-anatomy.md, ue3-staticmesh-extraction.md

## Seeds and content chains

- seeds-and-content-chains-index.md — sub-index: template/cover/name seeds, chain conditions and edge triggers, dialog binds, inventory locks, pet and trainer seeds.
  - Files: cell-startup-caches-vs-base-roundtrip.md, chain-replay-trigger-param-vacuity.md, class-split-chain-authoring.md, container-capacity-and-grant-targets.md, content-chain-authoring-traps.md, content-chain-condition-context-gaps.md, content-chain-dispatch-traps.md, content-engine-condition-gotchas.md, cover-seed-ids-and-orient-convention.md, debug-hub-npc-authoring-traps.md, dialog-button-strip-and-seed-agreement.md, dialog-set-bind-routing-and-edges.md, edge-trigger-replay-and-abandon.md, effect-nvp-generator-seed-traps.md, entity-template-seed-authoring.md, inventory-lock-keys-and-failure-injection.md, move-path-lock-layers-and-vault-verdict.md, org-cash-balance-and-overflow-traps.md, pet-template-seed-traps.md, player-loaded-edge-trigger-race.md, seed-name-columns-and-placeholders.md, seed-name-id-and-asset-naming.md, template-look-copy-and-client-evidence.md, trainer-seed-and-gm-grant-traps.md

## Debug Area

- debug-area-plaza-and-training-dummies.md — tags fire in any world; TrainingDummy mark; gm_ability_bulk; hub once-placed guards scoped to the hub.

## Base sessions

- unentered-entity-destroy-and-db-await-seam.md — never-entered entity: DestroyEntity, not DisconnectEntity (player_id-keyed squad/position); fake TCP listener for DB-await races.
- session-scoped-side-tables-and-teardown.md — clear per-address state in teardown after the owner check (covers relaunch takeover); logOff keeps the address.
- mail-expiry-and-notify-seams.md — every mail writer sets `expires_at`; `NOT quarantined` on every player path.
- connected-map-view-over-parallel-index.md — online lookups: a view over `connected` + `listed_online`, not a parallel map.
- trade-results-client-ignores-space-cash.md — client trade window acts only on results 1/2; space/cash codes 3-6 show nothing, so send a feedback line.
- tell-channel-and-ignore-copies.md — client /tell is byte 10 (CHAN_TELL since SS-C4); the Ignore list has 3 copies synced by one resync.
- chat-channel-client-display.md — server 8 opens a modal prompt, 9 is plain feedback, 7 shows nothing (nil ChannelMap).

## Cell systems

- cell-systems-index.md — sub-index: grants and loot, per-session state, abilities and effects, NPC AI, missions, pets, crafting, black market, duels, respawn, and the ammo campaign (#1026) notes.
  - Files: ability-event-sets-are-server-only.md, ability-grant-provenance-seams.md, ability-launch-fire-split.md, absorb-shield-ledger-and-cleanse-categories.md, ai-state-private-and-revert-proof-mtime.md, ammo-on-hit-effect-needs-a-script.md, ammo-reserve-round-trip.md, beneficial-cast-resolution-and-abilitydef-fields.md, black-market-escrow-and-authority.md, bm-settlement-mail-traps.md, cell-mirrors-of-base-owned-counters.md, cimmeria-side-flag-bits-collide-with-client-enums.md, client-action-bar-is-client-side.md, combat-debug-notes-and-flush-points.md, crafting-induction-engine-seams.md, crafting-verb-packet-traps.md, crafting-verb-traps.md, cross-world-transfer-flow.md, damage-seams-and-gm-index-pins.md, deployable-pulse-and-seed-traps.md, destroy-entity-vs-despawn-npc.md, duel-end-paths-and-travel-scan.md, effect-category-and-friendly-target-gaps.md, effect-flag-64-marks-sequenced-damage.md, effect-routing-and-scoped-defs.md, effect-scripts-run-after-the-death-check.md, entity-recreate-and-tautological-expectations.md, grant-paths-pick-different-containers.md, grant-paths-starting-clip-and-archetype-param.md, grant-placement-and-loot-handback-traps.md, grantitem-overcap-and-process-wide-gm-switches.md, held-toggles-and-stance-moniker.md, kill-credit-seams-and-loot-ownership.md, lab-caster-dummy-and-mental-cleanse-gap.md, live-loot-containers-and-tag-state.md, mail-placement-rule-and-fixture-types.md, mechanical-target-signal-is-body-set.md, mission-persist-hydrate-roundtrip.md, native-consumable-round-trip.md, no-movement-type-wire-and-nav-path-writers.md, npc-ai-fight-test-fixtures.md, npc-ai-tick-snapshot-and-hash-order.md, npc-caster-player-ordered-gates.md, npc-class-filter-and-dead-target-traps.md, npc-detector-telemetry-traps.md, npc-range-gate-and-weapon-range-columns.md, on-hit-fanout-and-recursive-async-send.md, one-time-tutorial-and-combat-entry-seams.md, org-vault-storage-and-lock-order.md, owner-pet-effects-and-passives.md, per-session-player-state-lifecycle.md, per-shot-damage-seam-is-damage-apply.md, pet-owner-lifecycle-hooks.md, pulsing-script-reapply-and-npc-cc.md, revert-proof-commit-first.md, ring-transport-fsm.md, session-scoped-cell-state-hooks.md, stargate-address-book-three-legs.md, start-profile-seams.md, stat-with-no-consumer-trap.md, stored-target-lifetime-and-gm-view-check.md, support-shot-inverse-gate.md, throttle-key-hides-transitions.md, timed-effect-ledger-and-stat-routing.md
- adding-a-cimmeria-world-touchpoints.md — new world id = one ADDED_WORLDS row + seed/spaces data; nav/occ client-map fallback; bounds unused.
- minigame-session-lifecycle.md — SmartFox task owns the ticket; cancel on endCurrentMinigame, suppress late onEndMinigame, ticket on MinigameResult.

## Observability

- observability-index.md — sub-index: Rule 6 sweeps and scanner, Discord named pairs, OTLP redaction, client telemetry governor, log filters, firehose.
  - Files: ability-row-event-guard-and-cast-join.md, base-side-log-naming-and-lock-traps.md, client-telemetry-governor-classify-table.md, client-telemetry-index-and-upload-traps.md, discord-named-pairs-resolvers.md, discord-noise-and-teardown-race.md, firehose-framework-crate-placement.md, fixed-window-throttle-steady-retrier.md, log-filter-parity-traps.md, named-telemetry-sweep-traps.md, observability-test-and-throttle-traps.md, otel-log-redaction-wrap-provider.md, rule6-name-pairing-patterns.md, rule6-name-pairing-sweep-techniques.md, rule6-name-pairing-traps.md, rule6-sweep-scanner-and-naming-traps.md, telemetry-ingest-naming-traps.md, tracing-span-fields-not-on-log-records.md, unpaired-id-sweep-workflow-traps.md

## Testing patterns
- reference_dw03s_h_2026_10_16.md — channel-coupled resend provenance and synchronous tick-origin capture.

- lab-chaos-flush-seams.md — tests can't install lab_chaos: lend a registry via a cfg(test) thread-local; phase-1 record 37 B, cursor = offset + 1.
- test-support-dev-dep-means-live-db-listing.md — a test-support dev-dep forces a test-live-db listing; log-capture-only crates use a local layer.
- testing-patterns-index.md — sub-index: nextest vs cargo test, revert proofs, live-DB races/ports, chain replay, encrypted test sessions, LogCapture.
  - Files: bincode-persisted-cache-format.md, cargo-test-vs-nextest-flakiness.md, chain-replay-executor-guards.md, chain-replay-single-loader-and-db-mutation-revert.md, db-test-revert-verification.md, forced-db-race-share-lock.md, interact-range-and-logcapture-traps.md, live-db-lock-race-tests.md, live-db-scratch-cluster.md, live-db-seed-iteration-without-reload.md, local-postgres-port.md, revert-proof-mutation-must-be-confirmed.md, revert-test-restore-crlf-trap.md, revert-verification-checkout-wipes-uncommitted.md, revert-verification-loses-uncommitted-fmt.md, test-file-split-without-touching-mod-rs.md, test-session-packets-are-encrypted.md, vacuous-guard-and-sentinel-collision-review.md, wireclient-passive-session-dies.md
- tracing-capture-single-dispatcher-trap.md — scoped tracing capture records nothing under `cargo test` unless a 2nd Dispatch lives; static async locks couple tests.
- cap-guard-sized-from-the-constant.md — a cap guard sized `CAP + 1` passes when the cap is loosened; use a literal size.
- aoi-fixture-introducible-and-wire-ledger.md — account_id without archetype_id hides a test player from AoI; ability sends go through `wire_ledger` (AB-T4).
- damage-apply-miss-gate-and-seeded-rolls.md — since AB-06 a miss lands nothing; a literal effect_seq may roll a miss; use `seq_rolling`.
- table-driven-live-db-refusal-cases.md — one character per refusal case + collected failures, so each dropped check shows; rolled-back dry run as precheck.
- player-cast-fixtures-need-a-mechanic.md — since AB-12 a player cast with no mechanic is refused; effectless fixtures need `seed_mechanic_effect`.
- lab-runner-paused-time-poll-tests.md — lab runner poll/timeout tests use `start_paused = true`; deadlines must be `tokio::time::Instant`.
- channel-failure-test-seams.md — mpsc send fails only when closed: one-slot channel + hang-up reader; query failure via lazy pool on `[::1]:9`; tick-in-loop test.

## Campaign judgment

- legacy-command-parity-scoping-judgment.md — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- training-points-cache-absolute-write.md — `handle_grant_xp` writes training_points absolutely from the session cache.
- loot-seed-pins-and-grant-stack-cap.md — new loot rows break exact pins on tables 3/7/8/9 (filter by NOT EXISTS); GrantItem does not cap at max stack.
- admin-api-ingest-test-seams.md — admin-api has no dev-deps: test ingest through injected `*_inner` fns and a local capture layer; statics and env are shared under `cargo test`.
