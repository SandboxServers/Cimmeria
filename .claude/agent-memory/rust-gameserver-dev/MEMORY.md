# Rust Gameserver Dev Memory

## Phase −0.5 triage status (2026-05-13)

- [build-environment.md](build-environment.md) — **[PROMOTE → user-local feedback]** — Windows linker workaround for the repo's hardcoded WSL rust-lld path. Not bible-relevant; this is operational guidance specific to this contributor's host setup. Keep in memory as user-feedback; will not promote to `docs/spec/`.

### Inline-content section status

- **Project Context section** — **[DISCARD]** — `feature/rust-rewrite` branch reference is stale (Rust lives on `main`). The file-path list cites `crates/services/src/auth.rs` and `mercury_ext.rs` — both have since become directories. Don't trust this section; re-derive from the current crate layout. The C++ reference path was rewritten to `deprecated/cpp/src/baseapp/mercury/sgw/` in the mechanical pass.
- **Audit Findings link to audit-findings.md** — **[DISCARD]** — `audit-findings.md` does not exist (broken link). Drop the reference.
- **Critical Bug (FIXED): RESOURCE_FRAGMENT u32→u16** — **[PROMOTE → spec.protocol.mercury-wire-format §"InterfaceElement length encoding"]** — V5-confirmed against `findings/mercury-protocol-internals.md`. The fix has shipped and the regression test guards it; the bug history is bible-section-4-vs-section-5 material.
- **Packet Layout Gotcha (build_outgoing)** — **[PROMOTE → spec.protocol.mercury-wire-format §"packet layout"]** — section-5 implementation detail worth recording in the chapter.
- **Entity Class IDs** — **[PROMOTE → spec.engine.entity-description-parse-chain §"class ID assignment"]** — V5-confirmable from entities.xml; the 8-entry table is canonical.
- **Account Method Indices** — **[PROMOTE → spec.engine.entity-description-parse-chain §"method index assignment"]** — same chapter as the SGWPlayer 157-method table (in `bigworld-engine-advisor/sgwplayer-method-index-table.md`); Account is a separate entity with its own 8-index inheritance from ClientCache. Worth a row in the chapter's appendix.
- **Wire Format Notes** — **[PROMOTE → spec.protocol.mercury-wire-format]** — V5-confirmed. The rotation-swap claim should be re-cross-referenced against `bigworld-engine-advisor/protocol-comparison.md`'s flagged-for-verification status before promoting to canon.
- **C++ Account.py createCharacter Flow** — **[RE-VERIFY]** — the "Rust version is missing most of this" claim is **OUT OF DATE**. Current `crates/services/src/base/character_create.rs` persists alignment, archetype, gender, bodyset, world_id, abilities, components, skin_color_id. Re-snapshot before promoting; the python-side flow description still maps cleanly to `spec.player.character-creation` section 3.
- **C++ Account.py requestCharacterVisuals Flow** — **[RE-VERIFY]** — the Rust-divergence claim (primaryTint=0, secondaryTint=0, raw skin_color_id index) needs re-verification against current `crates/services/src/base/world_entry_appearance.rs` or wherever character-visuals now lives. V5 `findings/character-creation-pipeline.md` confirms the canonical SkinTintColorID resolution to `0xRRGGBB00` packed uint32 — so the python flow is bible-ready, but the Rust gap claim may have been closed.

## Build Environment

- [build-environment.md](build-environment.md) — rust-lld override is OBSOLETE (fixed upstream); a fresh worktree needs `external/` junction-linked; cargo's stderr is block-buffered through the Bash tool so a hung *test* looks like a hung build (diagnose via `UserModeTime`).
- [stale-branch-clippy-toolchain-drift.md](stale-branch-clippy-toolchain-drift.md) — CI clippy floats to current stable (no toolchain pin), so idle branches fail on brand-new lints unrelated to their diff; update-branch before investigating.
- [lane-sh-masks-cargo-exit-code.md](lane-sh-masks-cargo-exit-code.md) — `lane.sh` / `live-db-test.sh` exit 0 even when cargo failed; redirect to a file and grep for `^error` + `[lane] released (exit 0)`.

## Working Environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — when other Claude sessions are running on the same repo, use a git worktree under `.claude/worktrees/<slug>/` for branch isolation. Junction-link `external/` into the worktree (`external/` is gitignored).
- [stacked-branch-rebase-traps.md](stacked-branch-rebase-traps.md) — a handed-down base sha is often NOT an ancestor (the parent rewrote it); find the fork point by message. Cargo.lock re-dirties every build. Use `..` in `Action::` test match arms.

## Navmesh / UE3 geometry export

- [navbuilder-obj-interop.md](navbuilder-obj-interop.md) — **read before touching the OBJ writer or any walkability test.** NavBuilder exits 0 on all four silent-failure traps: axis order (`v ue.X ue.Z ue.Y`), CRLF-only, reversed winding (`N_recast.y = -n_ue3.z`), and a stray `*.obj` in the chunk dir.
- [castle-staticmesh-coverage.md](castle-staticmesh-coverage.md) — Castle interior floors are BSP; ~0% of the Interrogation Block floor plane is StaticMesh. All 6,430 actors now resolve, 4,860 emitted (1,570 vetoed by `bCollideActors`); the 15% archetype gap was decorative clutter, NOT the missing floor.

## Wire-format gotchas

- [cooked-pak-and-dialog-override-traps.md](cooked-pak-and-dialog-override-traps.md) — `data/cache/*.pak` IS in git (docs lie); a fail-closed patcher + seed-only linter diverge silently; content-engine can't see services.

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — `client-method-dispatch-table.md` (interface, 0-66ish) vs `cell-method-dispatch-table.md` (full + 109+ GM tail) are DIFFERENT files, easy to cite the wrong one; GM-tail offset counting convention (`index = 109 + K`, count every `<Exposed/>` in def document order); movement-validator per-entity bypass pattern (touch_clock + update_entity_position must both still run on the bypass path).
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — TWO client-method index tables; `cell/client_methods/` is authoritative, `mercury::method_idx` is a drifted partial copy (shipped vendor payload to mission handlers).
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns BYTES CONSUMED, not the new absolute offset; chain with `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — an `add_dialog_set` bind pushes only `InteractionType(UINT64 TypeId)`; NULL-dialog rows are bindable indicators, `onInitialInteraction` (104) is never emitted, `topic_text` is dead data.
- [ue3-absent-property-defaults.md](ue3-absent-property-defaults.md) — **read before decoding any UE3 export whose props are optional.** An absent tagged property means the *class* default, which SGW licensee-modified (`Terrain.DrawScale3D` is `(100,100,100)`, not `(1,1,1)`); recover it from world-grid arithmetic + one known world coordinate, never from "what the majority writes".
- [ue3-staticmesh-extraction.md](ue3-staticmesh-extraction.md) — **read before extracting any cooked UE3 geometry.** `bCollideActors=false` actors STILL carry real kDOP data, so nothing below the mesh layer can tell a weather card in a doorway from a wall (1,570 in Castle); prefab actors have TWO archetype chains and the actor's is a dead end for the mesh; template object names are not unique (218 `StaticMeshComponent0` in one .upk) so match the dotted Outer path; never inherit `Location`; tagged-prop offset varies by class kind (Actor=32, StaticMesh=4, Component=8).
- [navbuilder-obj-traps.md](navbuilder-obj-traps.md) — **read before touching the navmesh build chain.** UE3→BW is a Y/Z column swap (`v x z y`), CRLF is mandatory or faces vanish, NavBuilder exits 0 on every failure, and a stray non-`<hex8>o.obj` in a chunked input dir reads uninitialised bounds. Detail in `docs/engine/navmesh-build-pipeline.md`.
- [ue3-bsp-model-decode.md](ue3-bsp-model-decode.md) — **read before any UModel/BSP work.** Empty Model == exactly 108 bytes (layout self-check); BSP node winding is the OPPOSITE of StaticMesh so fans must be reversed for NavBuilder; classify by owner export class; Castle brush-owned Models all DECODE to stubs but the cause is UNPROVEN and it is now the leading candidate for the missing interior storey connector; the persistent .umap has no BSP; ModelComponent is render-only.

## Seed authoring

- [entity-template-seed-authoring.md](entity-template-seed-authoring.md) — **read before touching `entity_templates` / `ability_sets` seed rows.** One ability per set (PK is `ability_set_id` alone); `event_set_id = NULL` on an ability means no attack animation; faction 10 is immutable AND gates both the player damage path and the right-click reroute, so talk-then-kill NPCs need two templates; mob level bands live in `texts.sql` moniker names; a template with neither `components` nor `static_mesh` is permanently invisible.

## Content engine

- [content-engine-condition-gotchas.md](content-engine-condition-gotchas.md) — **read before adding a `Condition` variant.** A rejected condition row UNGATES the chain (loader `filter_map` drops the row, keeps the chain) so reject-at-load is fail-OPEN; fail-closed is the minority convention; world ids live only in `resources.worlds` (column `world`, not `world_name`), never in spaces.xml; ~17 `event_dispatch` sites with no chokepoint; `console/net.rs` puts a space id in a world-id field.

## Content engine / executor

- [cell-startup-caches-vs-base-roundtrip.md](cell-startup-caches-vs-base-roundtrip.md) — the cell HAS a DB pool at startup and ~20 `SpaceManager` caches; a cell→base round-trip inside a content action breaks the chain's ordered action list. Adding a cache is always the same 4 edits; `dialog_screens` text IS reachable (`screen_id` globally unique).

## Content engine / chain authoring

- [content-chain-authoring-traps.md](content-chain-authoring-traps.md) — `display_dialog` silently drops NPC dialogs on non-interact triggers (monologue fallback only); `set_interaction_type` is zone-global; NOTHING respawns (`respawn_secs` NULL everywhere) so `entity_dead_tag` missions are one-shot; victory chains evaluate no conditions; label-signature asserts mask later test assertions.
- **zero-baseline rule** (in the same file) — `entity_templates.interaction_type` IS the runtime `interaction_type_flags` value (`spawn.rs:127`); never clear an entity's last cue bit or it goes unclickable zone-wide, and NO test type can observe it.
- [content-chain-dispatch-traps.md](content-chain-dispatch-traps.md) — **read before authoring any chain.** `display_dialog` needs an interact in the player's history; `dialog_choice` carries NO archetype; `enabled=false` does nothing to a victory chain; deferred actions survive death but not disconnect; button-less dialogs still raise `dialog_choice`; template-slot binds are `find_map` (one dialog-carrying bind per slot); the H9 edge race has a `player_loaded` form that bites CROSS-MISSION; negatives omitting the trigger key pass vacuously; `entity_interactions` has no Rust consumer.
- [player-loaded-edge-trigger-race.md](player-loaded-edge-trigger-race.md) — a `player_loaded`/`enter_region` chain gated on a state NEVER fires for the player who was already inside when the gate opened; fix with a second trigger row on the state-change event.
- [edge-trigger-replay-and-abandon.md](edge-trigger-replay-and-abandon.md) — H52 `enter_region` replay (mission-gate filter, `SpaceManager` re-entrancy guard, rings/stargates live OUTSIDE `fire_enter_region`) and H54 `mission_abandoned` (context populated AFTER the removal; four abandon entry points; unbind-before-rebind).
- [chain-replay-trigger-param-vacuity.md](chain-replay-trigger-param-vacuity.md) — a hand-built `TriggerEvent` missing `dialog_id`/`item_id`/`entity_tag` matches NOTHING, so every negative assertion passes on a trigger miss.

## NPC combat / ability selection

- [npc-range-gate-and-weapon-range-columns.md](npc-range-gate-and-weapon-range-columns.md) — **read before inventing any NPC range number.** `resources.items` has four range columns and both `NPC_ATTACK_RANGE`/`NPC_MELEE_RANGE` derive from them; range is gated in TWO places and `handle_use_ability`'s is still melee-blind (player path); 148 melee abilities carry bogus `max_range` 100-2500; layered-selector pattern for adding a filter without touching existing tests.
- [npc-ai-fight-test-fixtures.md](npc-ai-fight-test-fixtures.md) — `make_ai_fixture` has NO navmesh, so `find_path` returns `None` and a chase cannot be asserted via `nav_path`; assert the arm's INFO log instead. `SpawnRecord` has no `Default`.

## Mission persistence

- [mission-persist-hydrate-roundtrip.md](mission-persist-hydrate-roundtrip.md) — **read before touching mission save/restore.** One serializer (`missions/persist.rs`) + one hydrator (`player_init/mission_restore.rs`); the roster is rebuilt from `resources.mission_objectives` (self-heals pre-#657 rows, and there are NO migrations); `complete_objective` returns bool so a no-op cannot fake a live executor arm; the hand-built-fixture trap that let #657 survive.

## Gate travel / address book

- [stargate-address-book-three-legs.md](stargate-address-book-three-legs.md) — **read before touching `known_stargates` or any mid-session persistent-list grant.** Three copies (DB / `CellEntity` / client); the client gets the whole book only at map load so a grant needs client method 66; write the cell + client BEFORE confirming the DB write; `address_origin` is a glyph not an id; idempotency needed at both ends.

## Stats / entity systems

- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat existing in `StatList` + `PUBLIC_STATS` + the AoI create payload does NOT mean anything reads it (`MOVEMENT_SPEED_MOD`/`ROTATION_SPEED_MOD` had zero server-side consumers until P47); plus the reject-don't-clamp GM-setter precedent and the canonical mutate→serialize_dirty→clear_dirty→`send_entity_method` publication pattern.
- [seed-name-id-and-asset-naming.md](seed-name-id-and-asset-naming.md) — **read before authoring any named NPC/prop seed row or concluding a map asset is missing.** `name_id` is client-PAK-resolved so new `texts.sql` moniker ids can never render (and NULL ships a nameless NPC silently); a moniker names the UE3 *asset family*, which is how to find map assets an English-keyword scan misses; binary `grep` on a chunk-compressed `.umap` gives false negatives.

## Navmesh / Recast

- [navmesh-recast-and-castle-topology.md](navmesh-recast-and-castle-topology.md) — Recast's UNCHECKED 24-bit `rcCompactCell::index` span cap is the real `cs` floor (silent empty mesh at exit 0, not the 16-bit caps); a bin target's `mod tests;` needs `#[path]`; Castle's 11 probes are 3 components at EVERY parameter set and InterpActors are a dead end.
## Handover / resuming work

- [resuming-a-dead-workers-wip.md](resuming-a-dead-workers-wip.md) — a `wip(...) unbuilt, unverified` commit may not compile (H06's called a function nobody wrote); its tests encode the design it *started* from; its integration requests go stale; a `TBD` validation table means nothing is proven. Port hunks by hand across a file→directory split, never resolve modify/delete by taking a side.

## Tooling quirks

- [python-write-mangles-utf8-and-crlf.md](python-write-mangles-utf8-and-crlf.md) — `pathlib.write_text` encodes cp1252 on this host, so a scripted edit that adds an em-dash writes byte `0x97` and the crate stops compiling; always `read_bytes().decode("utf-8")` / `write_bytes(...encode("utf-8"))` and restore CRLF by hand.

- [rustfmt-trailing-line-comment-quirk.md](rustfmt-trailing-line-comment-quirk.md) — rustfmt sucks standalone comments into the trailing-comment column of the previous statement; insert a blank line to break the run.
- [rustfmt-reorders-mod-declarations.md](rustfmt-reorders-mod-declarations.md) — `reorder_modules` is on by default, so a coordinator's "append your `mod` line at the END of the shared mod.rs" cannot survive `cargo fmt`; expect an alphabetical three-way merge.
- [clippy-items-after-test-module.md](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be LAST in a file; and clippy 1.98+ denies `chunks_exact(2)` in a WSTRING decoder (use `as_chunks::<2>()`).
- [tooling-filter-and-path-traps.md](tooling-filter-and-path-traps.md) — live-db-test.sh takes POSITIONAL nextest substrings (a `test()` filterset matches nothing, exit 4, after a 30s reload); `gh -F body=@file` needs a Windows path; a scripted CRLF doc edit must never put a carriage return in its replacement text (one bare CR makes git rewrite the whole file) and must `assert old in s` or it silently no-ops.
- [sqlx-dynamic-sql-string.md](sqlx-dynamic-sql-string.md) — `sqlx::query` takes `&'static str` only, so a `fn(&str) -> String` shared-SELECT helper won't compile; use a `macro_rules!` + `concat!` re-exported with `pub(crate) use`.
- [sqlx-chain-id-is-i32-vacuous-guards.md](sqlx-chain-id-is-i32-vacuous-guards.md) — `content_*.chain_id` is `integer` (i32), not i64; a wrong decode type PASSES forever inside a "must return no rows" guard and then panics with `ColumnDecode` instead of the assertion message on the day it catches something.
- [gitignore-swallows-new-dirs.md](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules (`server/`) silently hide a new `foo/mod.rs` split from `git add`; `git status --short` shows nothing. Check with `git check-ignore -v`.
- [worktree-shell-and-external-binary-tests.md](worktree-shell-and-external-binary-tests.md) — worktree-isolated Bash refuses `env VAR=x cmd`, heredoc-plus-shell-var and appends; and a test asserting a from-tree C++ binary's behaviour needs an explicit opt-in env var (bin64/ holds whatever branch built it last).

## Navmesh extraction (UE3 → NavBuilder)

- [navmesh-probe-and-bsp-traps.md](navmesh-probe-and-bsp-traps.md) — **read before diagnosing a "missing floor".** `NavGraph::locate` is XZ-containment-first and manufactures false negatives on stacked meshes (use `locate_within`); the NavBuilder walkable convention is `N_recast.y = -n_ue3.z` of the emitted order (two odd permutations cancel); a geometric-only BSP filter that works on Castle deletes real floors on Castle_CellBlock — always validate on a second map; and removing a big flat sheet *costs* vertices rather than saving them.

## GM feedback (cell ↔ base)

- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — **read before any method-28 work.** Four copies of the `onPlayerCommunication` serializer; `chat.rs` channel constants diverge from `enumerations.xml` for every channel ≥7 (only `CHAN_say=0` is agreed); `CHAN_FEEDBACK` is 9, not 8. Plus the cell-vs-base GM feedback split and the `notify_gm: bool` → `gm_feedback_to: Option<u32>` migration still owed by GrantItem/RemoveInventoryItem/GrantExpertise/GrantAppliedSciencePoints.

## Ring transport / entity teardown

- [ring-transport-fsm.md](ring-transport-fsm.md) — **read before any ring-travel or entity-teardown work.** `disconnect_entity` (async, has `tx`, emits LeftAoI now) vs `destroy_entity` (sync, no `tx`, deferred) and which hook goes where; the cross-world hand-off destroy trap; track participants by id not count; abort order is show-then-unlock; `BSF_MOVEMENT_LOCK`/`BSF_DEAD` are ref-counted so raw `|=`/`&= !` sticks the bit; reuse `cimmeria_mercury::clock::Clock` for injectable time in services.

## Cross-world / cross-space transfer

- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the BACK half (teardown lives cell-side in each caller); `find_or_create_space` can never join an existing instance; `resolve_space_id_fallback` + `register_space` are both fake "default instance" mechanisms; `CreateEntity.reply_tx` has no failure channel; disconnect-vs-create FIFO race leaves ghost entities.

## AoI / witness fanout

- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — `WitnessEntityMethod` has TWO `witness_entity_method` fns (logging wrapper in aoi_dispatch.rs + emitter in aoi.rs); both need signature changes. idbase via `entity_is_player` (61 player / 62 NPC, matters for method idx ≥61).

## Orientation / position

- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — **read before any orientation code.** `direction` is `[pitch, yaw, roll]` RADIANS for players AND NPCs (the struct doc comment is wrong); `update_entity_position`'s `[i8; 3]` param zeroes facing for 6 callers; `FORCED_POSITION` carries no angles; inbound client direction is never unpacked (live bug); `EntityMoved` carries direction every tick so rotation needs no fan-out.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — dependabot bumps `egui` and `eframe` separately; the egui-only PR is a no-op for the launcher (two egui versions coexist in the lock) and defers all API breakage to the eframe PR. Launcher clippy only runs in the Windows job of `launcher-build.yml`.

## Dialog buttons / seed-patch agreement

- [dialog-button-strip-and-seed-agreement.md](dialog-button-strip-and-seed-agreement.md) — **read before stripping or moving a dialog button.** A linter vacuity floor calibrated on today's data blocks the packet that changes it (floor is 1); a patch-vs-seed test needs a roster pin or deleting a row silently stops checking; the close-path `-1` IS the discard so eviction cannot double-emit; `fire_dialog_choice` sets no `archetype`.

## Content chains (seed authoring)

- [content-chain-condition-context-gaps.md](content-chain-condition-context-gaps.md) — **read before authoring any `content_*` rows.** `archetype` is NOT in the context on dialog chains so `archetype neq N` fails OPEN; zero-button dialogs DO fire `dialog_choice` with `button_id = -1` (adding a button kills the chain); `complete_objective` auto-complete sends the WRONG status byte; `delay_ms > 0` queues not runs; multi-trigger chains need `load_chain_expansions_for_test`; `set_interaction_type` is zone-wide so clearing can break other players.
- [dialog-set-bind-routing-and-edges.md](dialog-set-bind-routing-and-edges.md) — **read before authoring any quest-icon bind.** `add_dialog_set target_id` is a dialog_set_MAP id; NULL-`dialog_id` rows are KEPT since CA02 (any "dropped at load" comment is stale) and are the right choice when an `interact_tag` chain supplies the dialog; a bind fans to EVERY entity of the template; `player_loaded` is an EDGE and needs a `mission_completed` partner when the gate opens in the same world; mission ABANDON fires no chain event at all (campaign-wide gap, Rust fix); Route A kills the `last_interaction_target` pin a later `display_dialog` needs.

## Observability / logging

- [observability-test-and-throttle-traps.md](observability-test-and-throttle-traps.md) — **read before adding any counter/log test in cell code.** Counter emission is unobservable in tests (no Meter); `create_space_instance`'s nav path is CWD-relative so its log branch needs extraction to be testable; `create_entity` leaves `is_player=false`; the LogThrottle/`suppressed` pattern + its two mandatory guards; the immutable-then-mutable borrow order these helpers force.

- [tracing-span-fields-not-on-log-records.md](tracing-span-fields-not-on-log-records.md) — **read before any "stamp X onto every log" task.** `opentelemetry-appender-tracing` does NOT flatten ancestor span fields onto log records, so span-only enrichment is invisible in SigNoz Logs; spans don't cross the base↔cell mpsc boundary; `Option<T>` tracing fields are omitted when `None` (never `unwrap_or(0)`); `LogCapture` sees only event-own fields.

## Navmesh / containment

- [harset-nav-does-not-cover-upper-quarters.md](harset-nav-does-not-cover-upper-quarters.md) - harset.nav covers only the plaza (component 187); the Jaffa Zone, OP-CORE, towers and palace terrace have NO mesh at their real floor, so "is_point_valid on every spawn" cannot pass there.
- [navmesh-containment-modes.md](navmesh-containment-modes.md) - **read before any code that rejects a position/arrival/ring trip for being off-mesh.** Per-world `navmesh_mode` names (`enforces_navmesh_containment`, `load_world_rows`, `stamp_world_rows` - the old `*_world_ids` names are gone); the shared `TEST_SPACES_XML` fixture pins space counts and ids so adding a world breaks 4 tests; `get_nearest_point` returns its input on a miss; harset.nav's two floors and the Z -200..-228 hole.
- [obj-slab-and-nav-inspect-probe-traps.md](obj-slab-and-nav-inspect-probe-traps.md) — **read before deriving a coordinate from map data.** obj_slab's chunk pre-filter makes a `--column` contradict itself; the topmost up-facing surface in a roofed room is the ROOF; nav_inspect's `h` is Y-biased.

## AoI / entity lifecycle

- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `SpaceManager::destroy_entity` is bare state-removal with NO immediate LeftAoI fanout; `despawn_npc` is the correct primitive for any observer-visible NPC removal (content chains, GM commands). `content::executor::world::destroy_tagged_entity` had this exact gap until C08b fixed it.
- [effect-scripts-run-after-the-death-check.md](effect-scripts-run-after-the-death-check.md) — **read before any new kill path.** Effect scripts write HEALTH AFTER `damage_apply`'s lethality probe (sync `EffectContext`, can't await), so they made 0-HP NPCs that kept fighting; `abilities::death::resolve_death` is now the ONLY kill path, plus the bleed-kill test-fixture recipe.

## Testing patterns

- [cargo-test-vs-nextest-flakiness.md](cargo-test-vs-nextest-flakiness.md) — full-suite `cargo test -p cimmeria-services` has PRE-EXISTING order-dependent failures (LogCapture thread bleed); validate with `cargo nextest`, don't assume you broke it.
- [db-test-revert-verification.md](db-test-revert-verification.md) — split async DB-touching function into pure sync helper + DB shell; unit-test the helper so local revert-verification works when live-DB is the canonical guard. Also: revert-prove a *seed-content* guard with an in-place UPDATE (no second reload), and fit code+seed proofs into ONE lane hold.
- [bincode-persisted-cache-format.md](bincode-persisted-cache-format.md) — bincode 2 needs `config::legacy()` for 1.x-written files; wrong config decodes SILENTLY, so assert bytes-consumed == len and use an old-version byte fixture (round-trip alone can't catch it).
- [live-db-scratch-cluster.md](live-db-scratch-cluster.md) — `db.bat init` does NOT create the db/role or load the schema; recipe for an isolated scratchpad Postgres on :5544 so live-DB guards can actually be revert-verified.
- [chain-replay-executor-guards.md](chain-replay-executor-guards.md) — chain-replay must run `execute_actions` (not just `resolve_event`) when the change is an executor arm; sentinel-chain pattern for verbs with zero seed rows; `0x7000_5000` reserved.
- [local-postgres-port.md](local-postgres-port.md) — probe the dev Postgres port AND database name before every live-DB run (both move; parallel campaigns use per-campaign scratch DBs like `sgw_harset`); on a wrong one `require_db_or_skip!` self-skips and still reports PASS, so green means nothing until you check the skip count.
- [test-file-split-without-touching-mod-rs.md](test-file-split-without-touching-mod-rs.md) — `tests.rs` → `tests/mod.rs` + `tests/newfile.rs` needs ZERO edits to the shared parent `mod.rs` (`mod tests;` resolves identically either way); private helpers stay reachable via `super::` with no visibility changes.
- [revert-test-restore-crlf-trap.md](revert-test-restore-crlf-trap.md) — restore with `git checkout HEAD -- <file>` between revert tests; a multi-line Python `str.replace` silently no-ops on CRLF sources and the NEXT revert then runs against a still-broken tree.
- [revert-verification-checkout-wipes-uncommitted.md](revert-verification-checkout-wipes-uncommitted.md) — a `git checkout -- crates/` restore step in a revert loop deletes ALL uncommitted work; checkpoint per packet, scope the restore to one file, revert seed rows in-place via psql.
- [revert-verification-loses-uncommitted-fmt.md](revert-verification-loses-uncommitted-fmt.md) — `git checkout --` restoring from a WIP checkpoint silently discards an uncommitted `cargo fmt` pass; fmt BEFORE the checkpoint. Also: `git add crates/services` stages the gitignored `logs/`.
- [vacuous-guard-and-sentinel-collision-review.md](vacuous-guard-and-sentinel-collision-review.md) — **review checklist for any packet branch**: vacuous guards (revert proof names the wrong test), fixture-checks-itself asserts, live-DB-only coverage of a pure-value feature (`require_db_or_skip!` PASSes on skip), and cross-branch `0x7000_xxxx` sentinel collisions + the current claim registry.

## legacy-command-parity campaign

- [legacy-command-parity-scoping-judgment.md](legacy-command-parity-scoping-judgment.md) — recurring judgment calls when porting a legacy dot command: verify a packet's "read-only reference" file list actually contains the real logic (P02's `.facing` geometry was in a file NOT in the initial read set); don't port a legacy enum/name table onto a Rust field whose numbering has already diverged (check the field's own doc comment first); confirm a named "stop condition" by reading the actual struct, don't just assume; prefer a genuinely scoped-down partial implementation over blocking the whole command; read a shared helper's SIGNATURE before copying a sibling command's call verbatim (P18 — the sibling may carry a latent bug); a legacy `x=None, y=None, z=None` signature gated on one arg is a partial-tuple bug to correct, not a feature.
- [navmesh-onmesh-assertions-are-weak.md](navmesh-onmesh-assertions-are-weak.md) - **read before asserting a coordinate is on the navmesh.** `is_point_valid` and `find_path(..).is_some()` both pass on the WRONG component.
- [map-data-placement-toolkit.md](map-data-placement-toolkit.md) - **read before deriving a spawn/region coordinate from a cooked map.** obj_slab chunk pre-filter, heading = atan2(dx,dz), 4-corner BoundingBox convention.
- [telemetry-last-valid-is-mostly-synthetic.md](telemetry-last-valid-is-mostly-synthetic.md) - **read before using last_valid_* as walkable evidence.** 77% of Harset rejects are (0,0,0); use only the cleaned list.

## UE3 packages / map data

- [ue3-prefab-rig-anatomy.md](ue3-prefab-rig-anatomy.md) — **before decoding a component export or scoping a .umap patch.** Component props start at byte 8; prefab meshes live on imported archetypes; Matinee keys are relative; `.upk` uncompressed vs `.umap` LZO; `crates/upk` is read-only.
