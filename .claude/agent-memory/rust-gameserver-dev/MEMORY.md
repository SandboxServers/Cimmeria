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

## Working Environment

- [concurrent-claude-sessions.md](concurrent-claude-sessions.md) — when other Claude sessions are running on the same repo, use a git worktree under `.claude/worktrees/<slug>/` for branch isolation. Junction-link `external/` into the worktree (`external/` is gitignored).

## Wire-format gotchas

- [gm-tail-dispatch-doc-filename-trap.md](gm-tail-dispatch-doc-filename-trap.md) — `client-method-dispatch-table.md` (interface, 0-66ish) vs `cell-method-dispatch-table.md` (full + 109+ GM tail) are DIFFERENT files, easy to cite the wrong one; GM-tail offset counting convention (`index = 109 + K`, count every `<Exposed/>` in def document order); movement-validator per-entity bypass pattern (touch_clock + update_entity_position must both still run on the bypass path).
- [method-idx-duplicate-table-drift.md](method-idx-duplicate-table-drift.md) — TWO client-method index tables; `cell/client_methods/` is authoritative, `mercury::method_idx` is a drifted partial copy (shipped vendor payload to mission handlers).
- [read-wstring-offset-semantic.md](read-wstring-offset-semantic.md) — `read_wstring` returns BYTES CONSUMED, not the new absolute offset; chain with `offset += n`, never `offset = n`.
- [dialog-set-bind-carries-no-dialog-id.md](dialog-set-bind-carries-no-dialog-id.md) — an `add_dialog_set` bind pushes only `InteractionType(UINT64 TypeId)`; NULL-dialog rows are bindable indicators, `onInitialInteraction` (104) is never emitted, `topic_text` is dead data.
- [ue3-staticmesh-extraction.md](ue3-staticmesh-extraction.md) — UE3 StaticMeshActor→Component→Mesh resolution in SGW cooked .umap: tagged-prop offset varies by class kind (Actor=32, StaticMesh=4, Component=8); ~20% of actors use prefab archetypes; kDOP tri indices reference LOD0 vertices; master .umap files exist alongside chunks.

## Content engine / chain authoring

- [content-chain-authoring-traps.md](content-chain-authoring-traps.md) — `display_dialog` silently drops NPC dialogs on non-interact triggers (monologue fallback only); `set_interaction_type` is zone-global; NOTHING respawns (`respawn_secs` NULL everywhere) so `entity_dead_tag` missions are one-shot; victory chains evaluate no conditions; label-signature asserts mask later test assertions.
- [content-chain-dispatch-traps.md](content-chain-dispatch-traps.md) — **read before authoring any chain.** `display_dialog` needs an interact in the player's history (follow-up chains have only `last_interaction_target`); `dialog_choice` carries NO archetype so splits key on dialog id; `enabled=false` does nothing to a victory chain; deferred actions survive death but not disconnect (+ the rewind-`fire_at` test pattern); button-less dialogs still raise `dialog_choice`.

## Stats / entity systems

- [stat-with-no-consumer-trap.md](stat-with-no-consumer-trap.md) — a stat existing in `StatList` + `PUBLIC_STATS` + the AoI create payload does NOT mean anything reads it (`MOVEMENT_SPEED_MOD`/`ROTATION_SPEED_MOD` had zero server-side consumers until P47); plus the reject-don't-clamp GM-setter precedent and the canonical mutate→serialize_dirty→clear_dirty→`send_entity_method` publication pattern.

## Tooling quirks

- [rustfmt-trailing-line-comment-quirk.md](rustfmt-trailing-line-comment-quirk.md) — rustfmt sucks standalone comments into the trailing-comment column of the previous statement; insert a blank line to break the run.
- [clippy-items-after-test-module.md](clippy-items-after-test-module.md) — `#[cfg(test)] mod tests` must be the LAST item in a file; clippy `-D warnings` rejects trailing free functions after it.
- [gitignore-swallows-new-dirs.md](gitignore-swallows-new-dirs.md) — unanchored `.gitignore` dir rules (`server/`) silently hide a new `foo/mod.rs` split from `git add`; `git status --short` shows nothing. Check with `git check-ignore -v`.

## GM feedback (cell ↔ base)

- [gm-feedback-cell-base.md](gm-feedback-cell-base.md) — definitive (post-commit) GM feedback for base-round-trip commands: cell-side `cell_methods::gm::feedback::send_gm_feedback` (EntityMethodCall→onPlayerCommunication m28 CHAN_FEEDBACK=8) vs base-side `base::gm_feedback::send_gm_feedback_to_client` (send_to_witness_reliable). `GrantItem`/`RemoveInventoryItem` still gate on `notify_gm: bool`; `GrantCash`/`GrantXP` were changed (P05) to `gm_feedback_to: Option<u32>` so a selected-target grant's feedback goes to the caller, not the target — apply the same pattern to GrantItem/RemoveInventoryItem/GrantExpertise/GrantAppliedSciencePoints when a dot command needs it (P06 `.giveitem` will).

## Cross-world / cross-space transfer

- [cross-world-transfer-flow.md](cross-world-transfer-flow.md) — `handle_gate_travel` is the BACK half (teardown lives cell-side in each caller); `find_or_create_space` can never join an existing instance; `resolve_space_id_fallback` + `register_space` are both fake "default instance" mechanisms; `CreateEntity.reply_tx` has no failure channel; disconnect-vs-create FIFO race leaves ghost entities.

## AoI / witness fanout

- [witness-entity-method-dual-fn.md](witness-entity-method-dual-fn.md) — `WitnessEntityMethod` has TWO `witness_entity_method` fns (logging wrapper in aoi_dispatch.rs + emitter in aoi.rs); both need signature changes. idbase via `entity_is_player` (61 player / 62 NPC, matters for method idx ≥61).

## Orientation / position

- [cell-entity-direction-semantics.md](cell-entity-direction-semantics.md) — **read before any orientation code.** `direction` is `[pitch, yaw, roll]` RADIANS for players AND NPCs (the struct doc comment is wrong); `update_entity_position`'s `[i8; 3]` param zeroes facing for 6 callers; `FORCED_POSITION` carries no angles; inbound client direction is never unpacked (live bug); `EntityMoved` carries direction every tick so rotation needs no fan-out.

## Dependency bumps

- [egui-eframe-split-version-bumps.md](egui-eframe-split-version-bumps.md) — dependabot bumps `egui` and `eframe` separately; the egui-only PR is a no-op for the launcher (two egui versions coexist in the lock) and defers all API breakage to the eframe PR. Launcher clippy only runs in the Windows job of `launcher-build.yml`.

## Content chains (seed authoring)

- [content-chain-condition-context-gaps.md](content-chain-condition-context-gaps.md) — **read before authoring any `content_*` rows.** `archetype` is NOT in the context on dialog chains so `archetype neq N` fails OPEN; zero-button dialogs DO fire `dialog_choice` with `button_id = -1` (adding a button kills the chain); `complete_objective` auto-complete sends the WRONG status byte; `delay_ms > 0` queues not runs; multi-trigger chains need `load_chain_expansions_for_test`; `set_interaction_type` is zone-wide so clearing can break other players.

## Observability / logging

- [tracing-span-fields-not-on-log-records.md](tracing-span-fields-not-on-log-records.md) — **read before any "stamp X onto every log" task.** `opentelemetry-appender-tracing` does NOT flatten ancestor span fields onto log records, so span-only enrichment is invisible in SigNoz Logs; spans don't cross the base↔cell mpsc boundary; `Option<T>` tracing fields are omitted when `None` (never `unwrap_or(0)`); `LogCapture` sees only event-own fields.

## AoI / entity lifecycle

- [destroy-entity-vs-despawn-npc.md](destroy-entity-vs-despawn-npc.md) — `SpaceManager::destroy_entity` is bare state-removal with NO immediate LeftAoI fanout; `despawn_npc` is the correct primitive for any observer-visible NPC removal (content chains, GM commands). `content::executor::world::destroy_tagged_entity` had this exact gap until C08b fixed it.

## Testing patterns

- [cargo-test-vs-nextest-flakiness.md](cargo-test-vs-nextest-flakiness.md) — full-suite `cargo test -p cimmeria-services` has PRE-EXISTING order-dependent failures (LogCapture thread bleed); validate with `cargo nextest`, don't assume you broke it.
- [db-test-revert-verification.md](db-test-revert-verification.md) — split async DB-touching function into pure sync helper + DB shell; unit-test the helper so local revert-verification works when live-DB is the canonical guard. Also: revert-prove a *seed-content* guard with an in-place UPDATE (no second reload), and fit code+seed proofs into ONE lane hold.
- [bincode-persisted-cache-format.md](bincode-persisted-cache-format.md) — bincode 2 needs `config::legacy()` for 1.x-written files; wrong config decodes SILENTLY, so assert bytes-consumed == len and use an old-version byte fixture (round-trip alone can't catch it).
- [live-db-scratch-cluster.md](live-db-scratch-cluster.md) — `db.bat init` does NOT create the db/role or load the schema; recipe for an isolated scratchpad Postgres on :5544 so live-DB guards can actually be revert-verified.
- [chain-replay-executor-guards.md](chain-replay-executor-guards.md) — chain-replay must run `execute_actions` (not just `resolve_event`) when the change is an executor arm; sentinel-chain pattern for verbs with zero seed rows; `0x7000_5000` reserved.
- [local-postgres-port.md](local-postgres-port.md) — dev Postgres is on **5544**, not the documented 5433; on the wrong port `require_db_or_skip!` self-skips and still reports PASS, so green means nothing until you check the skip count.
- [test-file-split-without-touching-mod-rs.md](test-file-split-without-touching-mod-rs.md) — `tests.rs` → `tests/mod.rs` + `tests/newfile.rs` needs ZERO edits to the shared parent `mod.rs` (`mod tests;` resolves identically either way); private helpers stay reachable via `super::` with no visibility changes.
- [revert-verification-loses-uncommitted-fmt.md](revert-verification-loses-uncommitted-fmt.md) — `git checkout --` restoring from a WIP checkpoint silently discards an uncommitted `cargo fmt` pass; fmt BEFORE the checkpoint. Also: `git add crates/services` stages the gitignored `logs/`.

## legacy-command-parity campaign

- [legacy-command-parity-scoping-judgment.md](legacy-command-parity-scoping-judgment.md) — recurring judgment calls when porting a legacy dot command: verify a packet's "read-only reference" file list actually contains the real logic (P02's `.facing` geometry was in a file NOT in the initial read set); don't port a legacy enum/name table onto a Rust field whose numbering has already diverged (check the field's own doc comment first); confirm a named "stop condition" by reading the actual struct, don't just assume; prefer a genuinely scoped-down partial implementation over blocking the whole command; read a shared helper's SIGNATURE before copying a sibling command's call verbatim (P18 — the sibling may carry a latent bug); a legacy `x=None, y=None, z=None` signature gated on one arg is a partial-tuple bug to correct, not a feature.
