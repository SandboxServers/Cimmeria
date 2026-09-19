# Rust Gameserver Dev Memory

Index only — one line per topic file. Detail lives in the linked file.

## Build / working environment

- [build-environment.md](build-environment.md) — rust-lld override is OBSOLETE; a fresh worktree needs `external/` junction-linked; cargo stderr is block-buffered so a hung *test* looks like a hung build.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` exit 0 even when cargo failed; redirect to a file and grep `^error`.
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable, so idle branches fail on new lints unrelated to their diff; update-branch first.
- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — other sessions on the same repo means a git worktree under `.claude/worktrees/<slug>/`; junction-link the gitignored `external/`.
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha is often NOT an ancestor; find the fork point by message. Cargo.lock re-dirties every build.
- [worktree-shell-and-external-binary-tests.md](worktree-shell-and-external-binary-tests.md) — worktree Bash refuses `env VAR=x cmd`/heredoc-plus-shell-var; a from-tree C++ binary test needs an opt-in env var.

## Tooling quirks

- [python-write-mangles-utf8-and-crlf.md](python-write-mangles-utf8-and-crlf.md) — `pathlib.write_text` encodes cp1252 here, so a scripted em-dash breaks the build; always `read_bytes().decode`/`write_bytes(...encode)`.
- [tooling-filter-and-path-traps.md](tooling-filter-and-path-traps.md) — live-db-test.sh takes POSITIONAL nextest substrings (a `test()` filterset matches nothing, exit 4); `gh -F body=@file` needs a Windows path; scripted CRLF edits must `assert old in s`.
- [rustfmt-trailing-line-comment-quirk.md](rustfmt-trailing-line-comment-quirk.md) — rustfmt sucks standalone comments into the previous statement's trailing-comment column; a blank line breaks the run.
- [rustfmt-reorders-mod-declarations.md](rustfmt-reorders-mod-declarations.md) — `reorder_modules` is on, so "append your `mod` line at the END" cannot survive `cargo fmt`; expect an alphabetical merge.
- [clippy-items-after-test-module.md](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be the LAST item in a file under `-D warnings`.
- [gitignore-swallows-new-dirs.md](gitignore-swallows-new-dirs.md) — unanchored dir rules (`server/`) silently hide a new `foo/mod.rs` from `git add`; check `git check-ignore -v`.
- [sqlx-dynamic-sql-string.md](sqlx-dynamic-sql-string.md) — `sqlx::query` takes `&'static str`, so a `fn(&str) -> String` shared-SELECT helper won't compile; use `macro_rules!` + `concat!`.
- [sqlx-chain-id-is-i32-vacuous-guards.md](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is i32; a wrong decode type PASSES forever inside a "must return no rows" guard.

## Testing patterns

- [cargo-test-vs-nextest-flakiness.md](cargo-test-vs-nextest-flakiness.md) — full-suite `cargo test -p cimmeria-services` has pre-existing order-dependent LogCapture failures; validate with nextest.
- [vacuous-guard-and-sentinel-collision-review.md](vacuous-guard-and-sentinel-collision-review.md) — **review checklist for any packet branch**: vacuous guards, fixture-checks-itself asserts, live-DB-only coverage of a pure-value feature, `0x7000_xxxx` sentinel collisions.
- [db-test-revert-verification.md](db-test-revert-verification.md) — split an async DB function into pure helper + DB shell so local revert-verification works; revert-prove a seed guard with an in-place UPDATE.
- [live-db-scratch-cluster.md](live-db-scratch-cluster.md) — `db.bat init` does NOT create the db/role or load the schema; recipe for an isolated scratch Postgres on :5544.
- [local-postgres-port.md](local-postgres-port.md) — probe the dev Postgres port AND database name every run; on a wrong one `require_db_or_skip!` self-skips and still reports PASS.
- [chain-replay-executor-guards.md](chain-replay-executor-guards.md) — chain-replay must run `execute_actions`, not just `resolve_event`, when the change is an executor arm; sentinel-chain pattern.
- [bincode-persisted-cache-format.md](bincode-persisted-cache-format.md) — bincode 2 needs `config::legacy()` for 1.x files; wrong config decodes SILENTLY, so assert bytes-consumed == len.
- [test-file-split-without-touching-mod-rs.md](test-file-split-without-touching-mod-rs.md) — `tests.rs` → `tests/mod.rs` needs ZERO edits to the parent `mod.rs`; helpers stay reachable via `super::`.
- [revert-test-restore-crlf-trap.md](revert-test-restore-crlf-trap.md) — restore with `git checkout HEAD -- <file>` between revert tests; a multi-line Python replace silently no-ops on CRLF.
- [revert-verification-loses-uncommitted-fmt.md](revert-verification-loses-uncommitted-fmt.md) — `git checkout --` from a WIP checkpoint discards an uncommitted `cargo fmt`; fmt BEFORE the checkpoint.
- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unbuilt` commit may not compile; its tests encode the design it *started* from; a `TBD` validation table proves nothing.

## Wire format / protocol

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — `client-method-dispatch-table.md` vs `cell-method-dispatch-table.md` are DIFFERENT files; GM-tail index is `109 + K` counting every `<Exposed/>` in def order.
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — TWO client-method index tables; `cell/client_methods/` is authoritative, `mercury::method_idx` is a drifted partial copy.
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns BYTES CONSUMED, not the new absolute offset; chain with `offset += n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — an `add_dialog_set` bind pushes only `InteractionType(UINT64)`; `onInitialInteraction` (104) is never emitted; `topic_text` is dead data.

## UE3 / navmesh extraction

- [ue3-staticmesh-extraction.md](ue3-staticmesh-extraction.md) — **read before extracting cooked UE3 geometry.** `bCollideActors=false` actors still carry kDOP; prefab actors have two archetype chains; template names aren't unique; never inherit `Location`.
- [ue3-absent-property-defaults.md](ue3-absent-property-defaults.md) — an absent tagged property means the *class* default, which SGW licensee-modified (`Terrain.DrawScale3D` is `(100,100,100)`); recover it from world-grid arithmetic.
- [ue3-bsp-model-decode.md](ue3-bsp-model-decode.md) — **read before any UModel/BSP work.** Empty Model == exactly 108 bytes; BSP winding is the OPPOSITE of StaticMesh; the persistent .umap has no BSP.
- [navbuilder-obj-traps.md](navbuilder-obj-traps.md) — **read before touching the navmesh build chain.** UE3→BW is a Y/Z column swap, CRLF mandatory, NavBuilder exits 0 on every failure, stray `.obj` reads uninitialised bounds.
- [navbuilder-obj-interop.md](navbuilder-obj-interop.md) — the same four silent-failure traps from the OBJ-writer / walkability-test side.
- [navmesh-probe-and-bsp-traps.md](navmesh-probe-and-bsp-traps.md) — **read before diagnosing a "missing floor".** `NavGraph::locate` is XZ-first and false-negatives on stacked meshes; a geometric BSP filter tuned on Castle deletes real floors elsewhere.
- [navmesh-recast-and-castle-topology.md](navmesh-recast-and-castle-topology.md) — Recast's unchecked 24-bit `rcCompactCell::index` span cap is the real `cs` floor (silent empty mesh at exit 0); InterpActors are a dead end.
- [castle-staticmesh-coverage.md](castle-staticmesh-coverage.md) — Castle interior floors are BSP, ~0% StaticMesh; the 1,570 `bCollideActors` vetoes were decorative clutter, NOT the missing floor.
- [navmesh-containment-modes.md](navmesh-containment-modes.md) — **read before rejecting any position for being off-mesh.** Per-world `navmesh_mode`; the shared `TEST_SPACES_XML` pins space ids so adding a world breaks 4 tests; `get_nearest_point` returns its input on a miss.

## Movement / position / entity lifecycle

- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — **read before any orientation code.** `direction` is `[pitch, yaw, roll]` RADIANS (the struct doc is wrong); `update_entity_position`'s `[i8; 3]` zeroes facing; `FORCED_POSITION` carries no angles.
- [ring-transport-fsm.md](ring-transport-fsm.md) — **read before ring-travel or teardown work.** `disconnect_entity` vs `destroy_entity`; track participants by id; `BSF_MOVEMENT_LOCK`/`BSF_DEAD` are ref-counted so raw `|=` sticks the bit.
- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `destroy_entity` is bare state-removal with NO immediate LeftAoI fanout; `despawn_npc` is the primitive for observer-visible NPC removal.
- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the BACK half; `find_or_create_space` can never join an existing instance; `CreateEntity.reply_tx` has no failure channel.
- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — TWO `witness_entity_method` fns (logging wrapper + emitter); both need signature changes. idbase via `entity_is_player` (61/62).

## Observability / logging

- [observability-test-and-throttle-traps.md](observability-test-and-throttle-traps.md) — **read before adding any counter/log test in cell code.** Counter emission is unobservable (no Meter); `create_entity` leaves `is_player=false`; the LogThrottle/`suppressed` pattern + its borrow order.
- [throttle-key-hides-transitions.md](throttle-key-hides-transitions.md) — a per-entity throttle window swallows the row that says state CHANGED; key by `(entity_id, kind)`. Also: `destroy_space` is a 2nd teardown path.
- [tracing-span-fields-not-on-log-records.md](tracing-span-fields-not-on-log-records.md) — **read before any "stamp X onto every log" task.** Ancestor span fields are NOT flattened onto log records; `Option<T>` fields vanish when `None` (never `unwrap_or(0)`).
- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — cell-side `send_gm_feedback` (onPlayerCommunication m28, CHAN_FEEDBACK=8) vs base-side `send_gm_feedback_to_client`; prefer `gm_feedback_to: Option<u32>` over `notify_gm: bool`.

## Content engine / chain authoring

- [content-chain-dispatch-traps.md](content-chain-dispatch-traps.md) — **read before authoring any chain.** `display_dialog` needs an interact in history; `dialog_choice` carries NO archetype; `enabled=false` does nothing to a victory chain; deferred actions survive death but not disconnect; template-slot binds are `find_map`; a step going active in-place gets NO second `player_loaded`; `entity_interactions` has no Rust consumer.
- [content-chain-condition-context-gaps.md](content-chain-condition-context-gaps.md) — **read before authoring any `content_*` rows.** `archetype` is absent on dialog chains so `archetype neq N` fails OPEN; zero-button dialogs still fire `dialog_choice` with `button_id = -1`; `delay_ms > 0` queues, not runs.
- [content-chain-authoring-traps.md](content-chain-authoring-traps.md) — `display_dialog` drops NPC dialogs on non-interact triggers; NOTHING respawns so `entity_dead_tag` missions are one-shot; **zero-baseline rule**: `entity_templates.interaction_type` IS the runtime flag value, never clear an entity's last cue bit.
- [dialog-set-bind-routing-and-edges.md](dialog-set-bind-routing-and-edges.md) — **read before authoring any quest-icon bind.** `target_id` is a dialog_set_MAP id; NULL-`dialog_id` rows are KEPT since CA02; a bind fans to EVERY entity of the template; mission ABANDON fires no chain event.
- [player-loaded-edge-trigger-race.md](player-loaded-edge-trigger-race.md) — a `player_loaded`/`enter_region` chain gated on a state NEVER fires for the player already inside when the gate opened; add a second trigger row.
- [chain-replay-trigger-param-vacuity.md](chain-replay-trigger-param-vacuity.md) — a hand-built `TriggerEvent` missing `dialog_id`/`item_id`/`entity_tag` matches NOTHING, so every negative assertion passes vacuously.
- [content-engine-condition-gotchas.md](content-engine-condition-gotchas.md) — **read before adding a `Condition` variant.** A rejected condition row UNGATES the chain (fail-OPEN); world ids live only in `resources.worlds` (column `world`).
- [cell-startup-caches-vs-base-roundtrip.md](cell-startup-caches-vs-base-roundtrip.md) — the cell HAS a DB pool at startup and ~20 `SpaceManager` caches; a cell→base round-trip inside a content action breaks the chain's ordered action list.

## Seed authoring

- [entity-template-seed-authoring.md](entity-template-seed-authoring.md) — **read before touching `entity_templates` / `ability_sets`.** One ability per set; faction 10 is immutable and gates the damage path; a template with neither `components` nor `static_mesh` is invisible.
- [seed-name-id-and-asset-naming.md](seed-name-id-and-asset-naming.md) — **read before authoring a named NPC/prop row.** `name_id` is client-PAK-resolved so new `texts.sql` ids can never render; a moniker names the UE3 asset family.

## Gameplay systems

- [npc-range-gate-and-weapon-range-columns.md](npc-range-gate-and-weapon-range-columns.md) — **read before inventing any NPC range number.** Four range columns in `resources.items`; range is gated in TWO places; 148 melee abilities carry bogus `max_range`.
- [npc-ai-fight-test-fixtures.md](npc-ai-fight-test-fixtures.md) — `make_ai_fixture` has NO navmesh, so `find_path` returns `None` and a chase cannot be asserted via `nav_path`; assert the arm's log instead.
- [mission-persist-hydrate-roundtrip.md](mission-persist-hydrate-roundtrip.md) — **read before touching mission save/restore.** One serializer + one hydrator; the roster is rebuilt from `resources.mission_objectives`; there are NO migrations.
- [stargate-address-book-three-legs.md](stargate-address-book-three-legs.md) — **read before touching `known_stargates`.** Three copies (DB / `CellEntity` / client); a mid-session grant needs client method 66; write cell+client BEFORE confirming the DB write.
- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat in `StatList` + `PUBLIC_STATS` + the AoI payload does NOT mean anything reads it; plus the reject-don't-clamp GM-setter precedent and the publication pattern.
- [legacy-command-parity-scoping-judgment.md](legacy-command-parity-scoping-judgment.md) — recurring judgment calls when porting a legacy dot command: the real logic may be outside the given read set; check a field's own doc before porting a legacy enum; read a shared helper's signature before copying a sibling call.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — dependabot bumps `egui` and `eframe` separately; the egui-only PR is a no-op and defers all breakage to the eframe PR. Launcher clippy runs only in the Windows job.
