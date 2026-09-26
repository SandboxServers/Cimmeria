# Rust Gameserver Dev Memory

One line per topic file; the detail lives in the file. Keep hooks short (this index must stay under ~17 KB).

## Build environment

- [build-environment.md](build-environment.md) — rust-lld override obsolete; worktrees need the `external/` junction; a hung test looks like a hung build (check `UserModeTime`).
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable; idle branches fail on new lints. Update the branch first.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` exit 0 on a failed cargo; grep the captured file for `^error` and the `[lane] released` line.
- [dependency-dedupe-blockers.md](dependency-dedupe-blockers.md) — duplicate dep versions pinned upstream (sqlx, axum ws, reqwest, rmcp); machete false positives.
- [services-split-extraction-traps.md](services-split-extraction-traps.md) — extracting a crate from services: test-support dev-dep forces the live-DB list; allowlist edges vanish; unreachable_pub; split tests out; partial-move shims.

## Working environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — other sessions on the repo: work in `.claude/worktrees/<slug>/`, junction `external/`.
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha may not be an ancestor; find the fork point by message. Cargo.lock re-dirties.
- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unverified` commit may not compile; its tests encode the starting design; port hunks by hand.

## Tooling quirks

- [python-write-mangles-utf8-and-crlf.md](python-write-mangles-utf8-and-crlf.md) — `write_text` encodes cp1252: use bytes + restore CRLF; `sed -i` strips CR; the Bash tool turns `\\` into `\` (use scratch scripts).
- [rustfmt-trailing-line-comment-quirk.md](rustfmt-trailing-line-comment-quirk.md) — rustfmt pulls a standalone comment into the previous line's trailing column; add a blank line.
- [rustfmt-reorders-mod-declarations.md](rustfmt-reorders-mod-declarations.md) — `reorder_modules` sorts `mod` lines, so "append at the end" never survives `cargo fmt`.
- [clippy-items-after-test-module.md](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be last; clippy 1.98+ wants `as_chunks::<2>()` over `chunks_exact(2)`.
- [tooling-filter-and-path-traps.md](tooling-filter-and-path-traps.md) — `live-db-test.sh` takes positional substrings, not filtersets; `gh -F body=@file` needs a Windows path.
- [sqlx-dynamic-sql-string.md](sqlx-dynamic-sql-string.md) — `sqlx::query` needs `&'static str`; share SELECTs with `macro_rules!` + `concat!`.
- [sqlx-chain-id-is-i32-vacuous-guards.md](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type hides inside "no rows" guards.
- [gitignore-swallows-new-dirs.md](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules hide a new `foo/mod.rs`; check with `git check-ignore -v`.
- [worktree-shell-and-external-binary-tests.md](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd` and heredoc appends; C++-binary tests need an opt-in env var.

## Wire format

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — client- vs cell-method dispatch tables are different files; GM tail is `109 + K`.
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — `cell/client_methods/` is authoritative; `mercury::method_idx` is a drifted partial copy.
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns bytes consumed: `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — a bind pushes only `InteractionType`; method 104 is never emitted.
- [cooked-pak-and-dialog-override-traps.md](cooked-pak-and-dialog-override-traps.md) — `data/cache/*.pak` IS in git; fail-closed patcher vs seed linter diverge silently.
- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — four method-28 serializers; `CHAN_FEEDBACK` is 9; `notify_gm` -> `gm_feedback_to` migration still owed.
- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — two `witness_entity_method` fns; idbase 61 player / 62 NPC matters for index >= 61.
- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — `direction` is `[pitch, yaw, roll]` radians for all entities; `[i8; 3]` param zeroes facing.

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

## Cell systems

- [npc-range-gate-and-weapon-range-columns.md](npc-range-gate-and-weapon-range-columns.md) — four item range columns; range gated in two places; bogus melee `max_range`.
- [npc-ai-fight-test-fixtures.md](npc-ai-fight-test-fixtures.md) — `make_ai_fixture` has no navmesh; assert the INFO log, not `nav_path`.
- [npc-detector-telemetry-traps.md](npc-detector-telemetry-traps.md) — AI-path statics race across tests (use task_local); release detector state on destroy.
- [npc-class-filter-and-dead-target-traps.md](npc-class-filter-and-dead-target-traps.md) — `all_npc_entity_ids` is mob-only; HEALTH alone is not dead.
- [ai-state-private-and-revert-proof-mtime.md](ai-state-private-and-revert-proof-mtime.md) — write `ai_state` via `npc_ai::set_ai_state`; restored files keep old mtimes.
- [no-movement-type-wire-and-nav-path-writers.md](no-movement-type-wire-and-nav-path-writers.md) — no movement-type wire exists; nav_path writes go through `movement_stop`.
- [mission-persist-hydrate-roundtrip.md](mission-persist-hydrate-roundtrip.md) — one serializer, one hydrator; roster rebuilt from `mission_objectives`.
- [stargate-address-book-three-legs.md](stargate-address-book-three-legs.md) — three copies (DB, cell, client); a grant needs client method 66.
- [cell-mirrors-of-base-owned-counters.md](cell-mirrors-of-base-owned-counters.md) — base-owned counters must be messaged to the cell.
- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat in `StatList` may have no reader; the dirty-publish pattern.
- [ring-transport-fsm.md](ring-transport-fsm.md) — `disconnect_entity` vs `destroy_entity`; `BSF_*` bits are ref-counted.
- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the back half; fake default-instance mechanisms.
- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `destroy_entity` sends no LeftAoI; use `despawn_npc` for visible removals.
- [effect-scripts-run-after-the-death-check.md](effect-scripts-run-after-the-death-check.md) — `abilities::death::resolve_death` is the only kill path.
- [throttle-key-hides-transitions.md](throttle-key-hides-transitions.md) — key throttles by `(entity_id, kind)`; `destroy_space` is a second teardown path.

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
- [vacuous-guard-and-sentinel-collision-review.md](vacuous-guard-and-sentinel-collision-review.md) — review checklist: vacuous guards and `0x7000_xxxx` collisions.

## Campaign judgment

- [legacy-command-parity-scoping-judgment.md](legacy-command-parity-scoping-judgment.md) — porting a legacy dot command: verify the reference files, don't port diverged enums.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — the egui-only dependabot PR is a no-op; the eframe PR carries the breakage.
