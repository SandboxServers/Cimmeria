# Telemetry Gaps Work Packets: W3 and W4

> Type: how-to (packet specifications). Audience: the coordinator and packet workers.
> Updated: 2026-10-10. The dispatch rules, shared contract, doc-row shorthand and file-collision table are in [work-packets.md](work-packets.md); every packet here follows them. Status is in [README.md](README.md#packet-status).

Each packet: meta line (wave, severity, agent, state, dependencies, crates for the lane, doc rows), then the change, then the test with the ship line (branch · worktree · commit subject). Lane checks are the standard four from the dispatch rules, run for every crate on the meta line.

## W3: Per-system gaps

### AoI and witness

### TG-AOI-01 Cap drop names the entity and kind

W3 · high · packet-coder · Ready · crates `cimmeria-base-session`, `cimmeria-base-world-entry` · docs: neg
- **Change:** `crates/base-session/src/base/deferred_aoi.rs` (`push_deferred`, :215-221): the cap WARN gains `event = "deferred_dropped"`, `reason = "buffer_full"`, `kind` (`entered|left|method|witness_method|invisible`), `entity_id` from the `DeferredAoiMsg`, `witness_name`. `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi_dispatch.rs` (`entered_aoi` :253, `left_aoi` :336): stop discarding `DeferOutcome`; `SessionGone` logs DEBUG `reason = "session_gone"`.
- **Test:** extend `push_deferred_drops_at_cap` (deferred_aoi.rs:516) with LogCapture: WARN with `event=deferred_dropped`, `entity_id` = the pushed id. Reverting loses `entity_id`. **Ship:** `telemetry-gaps/TG-AOI-01-cap-drop-entity` · `tg-aoi-01` · `feat(base-session): TG-AOI-01 name the entity on a deferred-AoI cap drop`

### TG-AOI-02 Appearance rebroadcast success row

W3 · med · packet-coder · Ready · crates `cimmeria-cell` · docs: obs
- **Change:** `crates/cell/src/cell/service/base_messages/mod.rs` (`BroadcastToWitnesses` arm, :331): keep the `usize` from `send_entity_method_to_witnesses`; INFO `target: "aoi.appearance"`, `event = "appearance_rebroadcast"`, entity pair, identity, `method_index`, `witness_count`. Pin per the procedure if INFO doesn't export by default.
- **Test:** unit, LogCapture, `appearance_rebroadcast_counts_witnesses`: one witness gives `witness_count=1`, none gives `0`. Removing the row fails it. **Ship:** `telemetry-gaps/TG-AOI-02-appearance-rebroadcast` · `tg-aoi-02` · `feat(cell): TG-AOI-02 log appearance rebroadcasts with witness count`

### TG-AOI-04 Flush-source `send_kind` for deferred-flush bundles

W3 · med · packet-coder · Ready · crates `cimmeria-base-session`, `cimmeria-base-world-entry` · docs: obs
- **Change:** `crates/base-session/src/base/helpers/mod.rs` (:551-585): `send_bundle_to_witness_reliable_kind(.., kind: &'static str)`; the old fn delegates with `"witness_bundle"`. `crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs` (:349, :374) passes `"aoi_flush_create_base"` and `"aoi_flush_cascade"`.
- **Test:** unit, LogCapture, `deferred_flush_bundle_has_its_send_kind`: flush two buffered `EnteredAoI`, the "AoI bundle: flushed" row has `send_kind=aoi_flush_create_base`. **Ship:** `telemetry-gaps/TG-AOI-04-flush-send-kind` · `tg-aoi-04` · `feat(base-session): TG-AOI-04 label deferred-flush bundles by send kind`

### TG-AOI-09 Per-message manifest on multi-message flush bundles

Aliases: T1. W3 · high · packet-coder · BlockedDependency (TG-AOI-04, TG-AOI-10) · crates `cimmeria-base-session`, `cimmeria-base-world-entry`, `cimmeria-server` · docs: obs
- **Change:** `crates/base-session/src/base/helpers/reliable_fit.rs`: when `send_kind` starts `aoi_flush_` and the bundle has more than one message, DEBUG `target: "aoi.bundle_manifest"` with `base_seq`, `msg_offsets`, `msg_ids`, `entity_ids` (comma-joined, capped at 64). `deferred_flush.rs` passes the per-message list. Pin `aoi.bundle_manifest=debug`. The client's `unpack_fault` offset then joins to the lost message.
- **Test:** unit, LogCapture, `flush_bundle_manifest_lists_each_message`: two entities give `entity_ids="a,b"` and two offsets. Reverting gives no row. **Ship:** `telemetry-gaps/TG-AOI-09-bundle-manifest` · `tg-aoi-09` · `feat(base-session): TG-AOI-09 per-message manifest on AoI flush bundles`

### TG-AOI-05 Classify `not_in_witness_aoi`

W3 · med · packet-coder · Ready · crates `cimmeria-cell` · docs: neg
- **Change:** `crates/cell/src/cell/service/base_messages/request_entity_update.rs` (:90-117): add `cause` ∈ {`entity_gone`, `other_space`, `not_introduced_yet`, `outside_aoi`} from `get_entity` and the space accessors. Keep WARN and the existing reason token.
- **Test:** extend the refuse test: no such entity gives `cause=entity_gone`; an NPC in another space gives `other_space`. **Ship:** `telemetry-gaps/TG-AOI-05-classify-not-in-aoi` · `tg-aoi-05` · `feat(cell): TG-AOI-05 say why an entity update was refused`

### TG-AOI-06 Close the cinematic-hold lifecycle

W3 · low · packet-coder · BlockedDependency (TG-AOI-01) · crates `cimmeria-base-world-entry`, `cimmeria-base-session` · docs: neg
- **Change:** `crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs` (`release`, :168-198): each silent `return` logs DEBUG `event = "hold_release_skipped"`, `reason` ∈ {`session_gone`, `already_releasing`, `token_mismatch`, `no_hold`}. `deferred_aoi.rs` (`log_discarded_on_teardown`): non-empty buffer during a hold → INFO `event = "hold_abandoned"`, `discarded`, `held_ms`.
- **Test:** unit, LogCapture: `release` with no session gives `session_gone`; teardown with a non-empty buffer gives the INFO. **Ship:** `telemetry-gaps/TG-AOI-06-cinematic-hold-lifecycle` · `tg-aoi-06` · `feat(base-world-entry): TG-AOI-06 log skipped and abandoned cinematic holds`

### TG-AOI-07 AoI counters and witness-set gauge

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world`, `cimmeria-cell` · docs: obs
- **Change:** `crates/cell-world/src/cell/space_manager/aoi.rs`: `counter!("aoi_introductions_total", world)` in `push_introduction` (:96), `aoi_leaves_total` in the leave loop (:329); the cell AoI tick sets gauge `aoi_witness_set_size{world}`.
- **Test:** unit with the metrics test recorder (`rg "npc_respawns_total" crates` for the precedent), `aoi_counters_match_compute_aoi_changes`. **Ship:** `telemetry-gaps/TG-AOI-07-aoi-metrics` · `tg-aoi-07` · `feat(cell-world): TG-AOI-07 AoI introduction and leave counters`

### TG-AOI-08 Leave and invisible send-failure row

W3 · low · packet-coder · Ready · crates `cimmeria-base-world-entry` · docs: neg
- **Change:** `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi.rs` (`left_aoi` :229-249, `entity_invisible` :456-478): keep the outcome; on non-`Sent` WARN `target: "aoi.create_send_failed"`, `phase = "leave" | "invisible"`, the `log_create_emit` fields via `AoiNames`.
- **Test:** unit next to `left_aoi_fans_out_one_packet_per_witness_to_each_addr`: unmapped witness gives WARN `phase=leave` with `entity_id`. **Ship:** `telemetry-gaps/TG-AOI-08-leave-send-failure` · `tg-aoi-08` · `feat(base-world-entry): TG-AOI-08 warn when an AoI leave is not sent`

### Engine and Mercury

### TG-MER-01 Numeric identity on `mercury.reliable_send`

W3 · high · packet-coder · Ready · crates `cimmeria-base-session` · docs: none
- **Change:** `crates/base-session/src/base/helpers/reliable_send.rs`, `shadow_register_reliable_send_with_details` (:198, :200, :221, :223): `player_id = state.active_player_id`, `entity_id = state.player_entity_id` (bare `Option`, not `?`).
- **Test:** unit, LogCapture next to `reliable_fit_tests.rs`, `reliable_send_player_id_is_numeric`: `active_player_id = Some(71)` gives `player_id="71"`. `?` gives `"Some(71)"`. **Ship:** `telemetry-gaps/TG-MER-01-reliable-send-ids` · `tg-mer-01` · `fix(base-session): TG-MER-01 numeric player and entity ids on reliable_send`

### TG-MER-02 Cause fields on `mercury.retransmit`

W3 · high · packet-coder · BlockedDependency (TG-MER-07) · crates `cimmeria-mercury`, `cimmeria-base-session` · docs: obs
- **Change:** `crates/mercury/src/channel/state.rs` (`TxEntry.first_sent`), `channel_core.rs` `check_timeouts` (:583): INFO gains `event = "retransmit"`, `age_ms`, `srtt_ms`, `peer_last_rx_ms`, `send_kind`, `msg_id`, `fragment_index`, `wire_fingerprint`. `crates/base-session/src/base/tick_sync.rs:198`: delete the duplicate "RTO fired before ACK" DEBUG, keep the send-error row.
- **Test:** unit in base-session, `retransmit_row_has_age_and_no_duplicate`: an expired entry via `collect_pending_retransmits` gives `event=retransmit` with `age_ms`, and no "RTO fired" row. **Ship:** `telemetry-gaps/TG-MER-02-retransmit-cause` · `tg-mer-02` · `feat(mercury): TG-MER-02 cause fields on retransmit rows`

### TG-MER-03 Channel log identity on retransmit and backpressure rows

W3 · high · packet-coder · BlockedDependency (TG-MER-02) · crates `cimmeria-mercury`, `cimmeria-base-session` · docs: nt
- **Change:** `state.rs`: `LogIdentity { account_id, account_name, player_id, player_name }` (all `Option`, `Default`); `channel_core.rs`: `set_log_identity`, fields on the retransmit and backpressure rows; `crates/base-session/src/base/helpers/mod.rs` `collect_pending_retransmits` syncs it from `session_identity` each tick.
- **Test:** unit, `retransmit_row_names_the_account`: account 6 / player 72 state, expired entry, assert both. Without the sync the fields are absent. **Ship:** `telemetry-gaps/TG-MER-03-channel-identity` · `tg-mer-03` · `feat(mercury): TG-MER-03 account and player on channel rows`

### TG-MER-04 Identity on `tx_hole` and `rx_order` rows

W3 · med · packet-coder · BlockedDependency (TG-MER-03) · crates `cimmeria-mercury`, `cimmeria-base-session` · docs: nt
- **Change:** `crates/mercury/src/channel/ack.rs` (`refresh_tx_hole` :193/:205, `check_tx_hole_named` :310) and `channel/rx_order.rs` (:194-219, :383): the `LogIdentity` fields.
- **Test:** base-session LogCapture, `tx_hole_open_names_the_account`: a footer acking seq 3 before 2 gives `tx_hole_open` with `account_id`. **Ship:** `telemetry-gaps/TG-MER-04-hole-identity` · `tg-mer-04` · `feat(mercury): TG-MER-04 identity on tx_hole and rx_order rows`

### TG-MER-05 Base→cell send seam: closed and backpressure WARNs

W3 · high · packet-coder (domain review: bigworld-engine-advisor) · Ready · crates `cimmeria-base` · docs: neg, obs
- **Change:** new `crates/base/src/base/connect_loop/cell_send.rs` with `send_to_cell(tx, msg, kind, addr, who, method_index)`; used at `cell_arms.rs:159, :172` and `encrypted/mod.rs:395`. `Err` → WARN `event = "cell_send_failed"`, `reason = "cell_channel_closed"`, identity, `msg_kind`, method pair. An await of 50 ms or more → WARN `event = "cell_channel_backpressure"`, `waited_ms`, `queue_free`, `msg_kind`, Pattern D (global key, 5 s), counter `base_cell_channel_stalls_total`.
- **Test:** unit, LogCapture: dropped receiver gives `cell_channel_closed`; a capacity-1 channel drained after 100 ms gives the backpressure row; a second stall in the window is suppressed. **Ship:** `telemetry-gaps/TG-MER-05-cell-send-seam` · `tg-mer-05` · `feat(base): TG-MER-05 log failed and stalled base-to-cell sends`

### TG-MER-06 Inbound bundle-walk drops out of TRACE

W3 · med · packet-coder · BlockedDependency (TG-MER-05) · crates `cimmeria-base` · docs: neg
- **Change:** `crates/base/src/base/connect_loop/encrypted/mod.rs`: :291 WARN `reason = "bundle_truncated"` with `offset`, `body_len`; :598 WARN `reason = "unhandled_msg_id"`; :267 AUTHENTICATE DEBUG → TRACE (135k a week); :339 DEBUG `reason = "no_player_entity"`. `account_arms.rs:201` WARN `reason = "unhandled_account_method"`. `cell_arms.rs`: :81 DEBUG `before_world_entry`; :92 DEBUG `map_loaded_pending` with method pair and `dropped_rest_of_bundle = true`; :122 else-arm WARN `subslot_missing_index`. Identity on each.
- **Test:** unit table test, LogCapture, `bundle_walk_drops_have_reasons`: one case per reason with its level pinned. **Ship:** `telemetry-gaps/TG-MER-06-bundle-walk-drops` · `tg-mer-06` · `feat(base): TG-MER-06 reasons on dropped inbound messages`

### TG-MER-11 Identity and reason on the handler-error and cell catch-alls

Aliases: TG-NET-06 (`connect_loop/mod.rs:88` half). W3 · low · packet-coder · BlockedDependency (TG-MER-10) · crates `cimmeria-base`, `cimmeria-cell` · docs: neg
- **Change:** `crates/base/src/base/connect_loop/mod.rs:88`: `identity_for_addr` fields, `reason = "handler_error"`, `error` as a field. `crates/cell/src/cell/dispatch/router.rs:169`: identity from `SpaceManager::player_identity`.
- **Test:** extend `unhandled_cell_method_warns_with_method_index_and_args_len` to assert `account_id`. **Ship:** `telemetry-gaps/TG-MER-11-catch-all-identity` · `tg-mer-11` · `feat(base): TG-MER-11 identity on catch-all handler warnings`

### TG-MER-13 Client version and cooked-reply fields

Aliases: T4 (server half; `client_version` is already on `cooked_data.version_reply`). W3 · low · packet-coder · BlockedDependency (TG-MER-06) · crates `cimmeria-base`, `cimmeria-base-session` · docs: none
- **Change:** `crates/base/src/base/connect_loop/account_arms.rs:198`: `onClientVersion` logs INFO `event = "client_version"` with the version the client sent and identity. `crates/base-session/src/base/cooked_data.rs`: `server_version` bare `Option`, not `?`; the too-short WARN (:51) gains `payload_len`, `reason = "malformed_args"`, identity.
- **Test:** unit, LogCapture, `version_reply_server_version_is_numeric`. **Ship:** `telemetry-gaps/TG-MER-13-client-version` · `tg-mer-13` · `feat(base): TG-MER-13 log the client version and numeric cooked versions`

### Combat

### TG-CMB-01 Fix the target-scan blind spot; export `player.respawn` DEBUG

W3 · high · packet-coder · Ready · crates `cimmeria-server` · docs: obs
- **Change:** `crates/server/src/logging/target_scan_tests.rs`, `strip_test_module`: stop only at an inline `#[cfg(test)] mod name {`, not at a `mod tests;` declaration (verified: today it truncates at either). `filters.rs`: add `player.respawn=debug`. If the wider scan flags other targets, pin them in this packet.
- **Test:** unit `strip_test_module_keeps_code_after_a_test_mod_declaration`; the existing `every_source_target_reaches_signoz_at_its_level` then guards the pin. **Ship:** `telemetry-gaps/TG-CMB-01-target-scan-blind-spot` · `tg-cmb-01` · `fix(server): TG-CMB-01 scan past test-module declarations and pin player.respawn`

### TG-CMB-02 Stale combat-state detector

W3 · high · packet-coder (domain review: combat-systems-advisor) · Ready · crates `cimmeria-cell`, `cimmeria-cell-combat`, `cimmeria-cell-world` · docs: neg
- **Change:** `crates/cell/src/cell/service/ticks/vitals.rs` (`vitals_sample_tick`) calls a new `log_combat_state_stale` in `crates/cell-combat/src/cell/combat/vitals.rs`; a `LogThrottle` field on `SpaceManager`, released in `destroy_entity`. WARN `target: "threat"`, `event = "combat_state_stale"`, `reason` ∈ {`mob_gone`, `mob_dead`, `bit_without_threat`, `threat_without_bit`}, identity, mob pair, `threat_count`, `state_field`, `world`, Pattern D per entity (60 s). Every living player is checked.
- **Test:** LogCapture, one case per reason, plus burst (3 ticks → 1 row, then `suppressed=2`) and independence. **Ship:** `telemetry-gaps/TG-CMB-02-stale-combat-state` · `tg-cmb-02` · `feat(cell-combat): TG-CMB-02 warn when combat state goes stale`

### TG-CMB-03 `effect_inert` WARN for effects that do nothing

W3 · med · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: neg
- **Change:** `crates/cell-combat/src/cell/abilities/effect_plan.rs` (`PlannedEffect::log`): when `path == skipped && reason == no_script`, also WARN `abilities.effect`, `event = "effect_inert"`, ability, effect, caster and target pairs, `caster_kind`, Pattern D keyed by `(ability_id, effect_id)` (60 s, static map). Live case: ability 710 / effect 736.
- **Test:** LogCapture: one no-script plan → one WARN; three → one row then `suppressed=2`; another effect id isn't swallowed. **Ship:** `telemetry-gaps/TG-CMB-03-effect-inert` · `tg-cmb-03` · `feat(cell-combat): TG-CMB-03 warn on effects with no script`

### TG-CMB-05 `bandolier` rows: Rule 5 and numeric ids

Aliases: TG-ITM-08 (bandolier half). W3 · med · packet-coder · BlockedDependency (TG-ITM-11, TG-NET-17) · crates `cimmeria-cell-combat`, `cimmeria-cell` · docs: neg, nt
- **Change:** `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs:412`, `bandolier/weapon_abilities.rs:117`, `crates/cell/src/cell/service/base_messages/player_init/mod.rs:43`: identity quartet; `item_id` as `Option<i32>` value; `resolved_ranged_ability` → `resolved_ability_id` plus `resolved_ability_name`; `?removed`/`?added` as numbers. Rename goes in the `neg` key-change table.
- **Test:** LogCapture on `weapon_ability_swap` and `active_slot_change`: `player_id == "72"`, `item_id == "55"`. **Ship:** `telemetry-gaps/TG-CMB-05-bandolier-ids` · `tg-cmb-05` · `fix(cell-combat): TG-CMB-05 identity and numeric ids on bandolier rows`

### TG-CMB-06 Death and revive rows: Rule 5/6 keys and world

W3 · med · packet-coder · BlockedDependency (TG-CMB-09) · crates `cimmeria-cell-combat`, `cimmeria-cell-methods` · docs: neg, nt
- **Change:** `crates/cell-combat/src/cell/abilities/death/mod.rs:451` (`target_killed`), `death/side_effects.rs:513` (`player.death`), `crates/cell-methods/src/cell/cell_methods/player/combat/mod.rs:47, :75`: `attacker` → `attacker_id`, `target` → `target_id`, `killer` → `killer_id`; player-side identity; `killer_name` `Option` (no `""`); `world` bare; `ability_id` `Option<i32>` (no `-1`); identity and `world` on `callForAid` and `player revived`.
- **Test:** LogCapture: `player.death` has `world == "Castle"` and no `killer_name` for an environment kill; `target_killed` has the player attacker's `player_id`. **Ship:** `telemetry-gaps/TG-CMB-06-death-row-keys` · `tg-cmb-06` · `fix(cell-combat): TG-CMB-06 Rule 5 and 6 keys on death and revive rows`

### TG-CMB-07 No-witness WARNs say why a player counted as present

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world`, `cimmeria-cell-combat` · docs: neg
- **Change:** `crates/cell-world/src/cell/space_manager/player_presence.rs`: `player_present_via(entity_id, counterpart) -> Option<&'static str>` (`player_side`, `aoi`, `threat_list`); `player_present` becomes `.is_some()`. `crates/cell-combat/src/cell/abilities/messaging.rs:245` and `use_ability/sequence.rs` (`no_witnesses`): `present_via` and `world`; `threat_list` logs DEBUG.
- **Test:** LogCapture: threat-listed player outside AoI gives DEBUG `present_via=threat_list`; a player in AoI with an empty witness list gives WARN `present_via=aoi`. **Ship:** `telemetry-gaps/TG-CMB-07-present-via` · `tg-cmb-07` · `feat(cell-combat): TG-CMB-07 say why a no-witness send counted a player`

### TG-CMB-08 Proximity aggro: say the combat flag was not announced

W3 · low · packet-coder · BlockedDependency (TG-NPC-11) · crates `cimmeria-cell-combat` · docs: none
- **Change:** `crates/cell-combat/src/cell/service/npc_ai/idle_aggro.rs:164`: when `generate_threat` returned `Some` on the proximity path, INFO `threat`, `event = "enter_combat_unannounced"`, `reason = "proximity_carveout"`, identity, mob pair.
- **Test:** LogCapture through `idle_aggro` with a hostile player in range gives one row. **Ship:** `telemetry-gaps/TG-CMB-08-unannounced-combat` · `tg-cmb-08` · `feat(cell-combat): TG-CMB-08 log combat entered without a client flag`

### TG-CMB-10 Log a failed `onEndAidWait` send

Aliases: TG-MOV-13. W3 · low · packet-coder · Ready · crates `cimmeria-cell-interactions` · docs: neg
- **Change:** `crates/cell-interactions/src/cell/respawn/mod.rs`: :155 `let _` → WARN `event = "wire_send_failed"`, `method = "onEndAidWait"`, `reason = "base_channel_closed"`, identity; the cross-world respawn INFO (:221) gains the identity quartet.
- **Test:** LogCapture with a closed `tx`: the WARN with `reason`. **Ship:** `telemetry-gaps/TG-CMB-10-end-aid-wait-send` · `tg-cmb-10` · `feat(cell-interactions): TG-CMB-10 warn when onEndAidWait is not sent`

### Items

### TG-ITM-01 Appearance refresh outcome rows

Aliases: TG-AOI-03. W3 · high · packet-coder · Ready · crates `cimmeria-base-methods` · docs: neg
- **Change:** `crates/base-methods/src/base/world_entry/methods/inventory/appearance.rs`: after `query_player_load_data` (:80), when `player_data.player_id != player_id`, ERROR `event = "appearance_refresh_degraded"`, `cause = "player_load_failed"`, identity (behaviour unchanged here; the abort is TG-ITM-14). Success: DEBUG `event = "appearance_refreshed"`, identity, `holstered`, `component_count`, `bodyset`. Skips (:53, :68, :99): `event = "appearance_refresh_skipped"`; DEBUG `reason = "witness_session_ended"` when `witness_recently_departed`, else WARN `reason = "entity_to_addr_miss" | "client_state_missing" | "entity_missing"`.
- **Test:** unit, LogCapture, `appearance_refresh_logs_degraded_and_skips`: `db_pool = None` gives the ERROR; an unmapped, not-departed entity gives the WARN; after `note_witness_departed` only DEBUG. **Ship:** `telemetry-gaps/TG-ITM-01-appearance-outcomes` · `tg-itm-01` · `feat(base-methods): TG-ITM-01 appearance refresh outcome rows`

### TG-ITM-14 Abort an appearance refresh on a failed load

W3 · high · packet-coder · BlockedDecision (D-TG11) and BlockedDependency (TG-ITM-01) · crates `cimmeria-base-methods` · docs: neg
- **Change (behaviour):** in the TG-ITM-01 branch, return before the `cached_appearance_args` write and both sends, so the default "naked human male" model is never cached or broadcast.
- **Test:** unit, `failed_load_does_not_cache_or_send_default_appearance`: cache stays `None`, no packet sent. Reverting caches and sends. **Ship:** `telemetry-gaps/TG-ITM-14-appearance-abort` · `tg-itm-14` · `fix(base-methods): TG-ITM-14 don't broadcast a default appearance after a failed load`

### TG-ITM-02 Loot roll outcome row

W3 · high · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: neg
- **Change:** `crates/cell-combat/src/cell/abilities/loot_drop.rs` (`generate_loot_on_death`): :37 DEBUG `event = "loot_skipped"`, `reason = "no_loot_table"`; end of roll DEBUG `event = "loot_rolled"`, `loot_table_id`, `loot_table_name`, `entries`, `dropped`; `loot_table_empty` (:44) → WARN `reason = "table_has_no_rows"`; :59-62 `reason = "entity_missing"`.
- **Test:** LogCapture: probability-0 table gives `loot_rolled dropped=0`; an unknown table gives the WARN. **Ship:** `telemetry-gaps/TG-ITM-02-loot-roll-outcome` · `tg-itm-02` · `feat(cell-combat): TG-ITM-02 log every loot roll's outcome`

### TG-ITM-04 Inline move refusals get `event` and `reason`

W3 · med · packet-coder · Ready · crates `cimmeria-base-methods` · live-DB · docs: neg
- **Change:** `crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs` (:278, :375, :448) and `move_/finish.rs` (:60, :388): keep each WARN; add `target: "inventory"`, `event = "move_rejected"`, `reason` ∈ {`invalid_slot`, `source_not_found`, `over_stack`, `not_allowed_in_container`, `split_onto_occupied`}, source container pair, `account_id`. Vault branches unchanged.
- **Test:** extend `move_/named_log_tests.rs`, one case per reason. **Ship:** `telemetry-gaps/TG-ITM-04-move-refusals` · `tg-itm-04` · `feat(base-methods): TG-ITM-04 reasons on inventory move refusals`

### TG-ITM-05 After-commit: no silent drops, richer move row

W3 · med · packet-coder · Ready · crates `cimmeria-base-methods` · live-DB · docs: neg
- **Change:** `move_/after_commit.rs`: :92 `let _ = cell_tx.send` → WARN `event = "move_cell_notify_failed"`, `reason = "cell_channel_closed"`; :154 `.ok().flatten().unwrap_or(false)` → ERROR `event = "move_appearance_lookup_failed"` on `Err`; the :80 row gains source and target container ids, `swapped_item_id`, `source_deleted`.
- **Test:** live-DB + LogCapture (`refusal_resync_tests.rs` style): dropped receiver gives the WARN; the persisted row has the new fields. **Ship:** `telemetry-gaps/TG-ITM-05-after-commit-drops` · `tg-itm-05` · `feat(base-methods): TG-ITM-05 log dropped post-move notifications`

### TG-ITM-07 Free vendor repair and recharge through `VendorLog`

W3 · med · packet-coder · BlockedDecision (D-TG12) · crates `cimmeria-base-methods` · live-DB · docs: neg
- **Change:** `crates/base-methods/src/base/world_entry/methods/vendor/repair.rs` (`handle_repair_inventory_items`) and `vendor/recharge.rs`: `VendorLog::new("repair"|"recharge", ..)` with `vendor_template_id = None`; `completed`, `refused("nothing_to_do")`, `failed("db_error")`. Waits until the free-repair path is confirmed legitimate, so the row doesn't bless an authority hole.
- **Test:** live-DB sentinel tests in both files gain LogCapture: `target == "vendor"`, `event = "transaction"`, `action`. **Ship:** `telemetry-gaps/TG-ITM-07-free-repair-vendorlog` · `tg-itm-07` · `feat(base-methods): TG-ITM-07 log free repairs and recharges as vendor transactions`

### TG-ITM-08 Numeric ids and identity on `item_sequence`

W3 · med · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: nt
- **Change:** `crates/cell-combat/src/cell/cell_methods/player/world/item_sequence.rs:26`: `archetype_id`, `event_set_id`, `seq_id` as bare `Option`s; identity quartet; `event = "item_sequence_lookup"`. (The bandolier half moved to TG-CMB-05.)
- **Test:** LogCapture, `item_sequence_ids_are_numeric`: `seq_id` is `"1873"`, not `"Some(1873)"`; `player_id` present. **Ship:** `telemetry-gaps/TG-ITM-08-item-sequence-ids` · `tg-itm-08` · `fix(cell-combat): TG-ITM-08 numeric ids and identity on item_sequence`

### TG-ITM-10 Remove rows: origin and stack before/after

W3 · med · packet-coder · Ready · crates `cimmeria-base-methods` · live-DB · docs: none
- **Change:** `inventory/core/remove_instance.rs` (:307) and `core/remove_by_type.rs`: `stack_before`, `stack_after`, `removed_all`, `origin` (`consume_for_use` for `AccessOp::Use`, `gm` when `notify_gm`, `remove`, `remove_by_type`), container pair.
- **Test:** live-DB LogCapture, `partial_remove_logs_stack_counts`: `stack_before=3`, `stack_after=2`. **Ship:** `telemetry-gaps/TG-ITM-10-remove-origin` · `tg-itm-10` · `feat(base-methods): TG-ITM-10 origin and stack counts on item removal`

### TG-ITM-12 Grant refusal levels and the duplicate full-bag row

W3 · low · packet-coder · Ready · crates `cimmeria-base-methods` · docs: none
- **Change:** `inventory/grant/persist.rs` (:305, :319): the two "container full" WARNs → DEBUG. `grant/grant_item.rs` (:412): `grant_refused` at WARN for `DatabaseError` or `NotGrantable`, INFO otherwise.
- **Test:** extend `full_bag_tests.rs`: a full bag gives exactly one INFO `grant_refused` and no WARN. **Ship:** `telemetry-gaps/TG-ITM-12-grant-refusal-levels` · `tg-itm-12` · `fix(base-methods): TG-ITM-12 one row per refused grant at the right level`

### TG-ITM-13 Double-click `useItem` level; identity on cell entry rows

W3 · low · packet-coder · Ready · crates `cimmeria-base-methods`, `cimmeria-cell-methods` · docs: nt
- **Change:** `inventory/core/use_instance.rs:175`: WARN → INFO `event = "use_refused"`, `reason = "entity_missing"`. `crates/cell-methods/src/cell/cell_methods/inventory/item_ops.rs`: `event` and identity on `removeItem`, `moveItem`, `useItem`, `listItems`; WARN `reason = "malformed_args"` on `repairItemRequest`'s truncated branch.
- **Test:** LogCapture in `use_instance_tests.rs` (INFO level) and `tests/use_item.rs` (`player_id`). **Ship:** `telemetry-gaps/TG-ITM-13-use-item-rows` · `tg-itm-13` · `fix(base-methods): TG-ITM-13 double-click use is info; identity on item calls`

### Minigames

### TG-MG-01 Base minigame seam: lost result and disabled server

W3 · high · packet-coder · Ready · crates `cimmeria-base-world-entry` · docs: neg
- **Change:** `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`: `minigame_result` `let _ = cell_tx.send` → ERROR `reason = "cell_channel_closed"`, entity pair, `result_code`, `result`, `chain_count`; WARN `reason = "no_cell_channel"` when `None`. `start_minigame`: WARN `reason = "minigame_server_disabled"` when the registry is `None`; `game_name`, `reason = "register_refused"` on "Failed to register".
- **Test:** LogCapture in `cell_dispatch/tests.rs`: dropped receiver gives the ERROR with `chain_count`; a `None` registry gives the WARN. **Ship:** `telemetry-gaps/TG-MG-01-minigame-result-seam` · `tg-mg-01` · `feat(base-world-entry): TG-MG-01 log lost minigame results and a disabled server`

### TG-MG-02 MinigamePlayer cell stubs log every call per the .def

W3 · high · packet-coder · Ready (coordinate with #1303) · crates `cimmeria-cell-methods` · docs: neg
- **Change:** `crates/cell-methods/src/cell/cell_methods/minigame.rs` (`dispatch`): every arm logs with `args_len`; parse only as `entities/defs/interfaces/MinigamePlayer.def` declares (INT32 for `debugStartMinigame`, `spectateMinigame`, `minigameCallAccept`, `minigameCallDecline`, `minigameContactRequest`; 3×INT32 for `debugMinigameInstance`; none for the rest; `args_len` only for the two `*RegisterToMinigameHelp`). Drop the invented `winner`/`loser`. 113 `endCurrentMinigame` calls logged 0 rows.
- **Test:** replace `end_current_minigame_row_names_the_caller_winner_and_loser` with `end_current_minigame_logs_with_wire_payload`: 4-byte `00000000` payload gives the INFO row. **Ship:** `telemetry-gaps/TG-MG-02-minigame-stub-rows` · `tg-mg-02` · `fix(cell-methods): TG-MG-02 minigame stubs log every call per the def`

### TG-MG-03 Session end reason, outcome and duration

W3 · med · packet-coder · Ready · crates `cimmeria-minigame` · docs: neg
- **Change:** `crates/minigame/src/minigame/server/mod.rs`: `run_session` returns an `EndReason`; `Minigame session ended` gains `event = "minigame.session_end"`, `end_reason` ∈ {`victory`, `defeat`, `client_closed`, `read_error`, `frame_too_long`, `idle_timeout`, `send_failed`}, `outcome`, `duration_ms`, `room_id`. The abort row (:564) takes `end_reason`; its text becomes "Minigame aborted without a result". Optionally `server/framing.rs` reports timeout vs error.
- **Test:** `timeout_tests.rs` harness: idle timeout gives `end_reason=idle_timeout outcome=canceled`; FIN gives `client_closed`. **Ship:** `telemetry-gaps/TG-MG-03-session-end-reason` · `tg-mg-03` · `feat(minigame): TG-MG-03 say why a minigame session ended`

### TG-MG-04 Duplicate-session row says why

W3 · med · packet-coder · BlockedDependency (TG-MG-01) · crates `cimmeria-minigame` · docs: neg
- **Change:** `crates/minigame/src/minigame/session.rs` (`register`): `reason = "duplicate_session"`, `existing_connected`, `existing_age_ms`, `existing_game`, `requested_game`; INFO when the existing session is pending and under 2 s old (a double interact), WARN otherwise (the #1303 shape).
- **Test:** LogCapture: two registers → INFO `existing_connected=false`; register, claim, register → WARN `existing_connected=true`. **Ship:** `telemetry-gaps/TG-MG-04-duplicate-session-why` · `tg-mg-04` · `feat(minigame): TG-MG-04 duplicate-session row says why`

### TG-MG-05 Livewire rejection rows name the player

W3 · med · packet-coder · Ready · crates `cimmeria-minigame` · docs: nt
- **Change:** `crates/minigame/src/minigame/games/livewire/mod.rs`: `LivewireGame::new` stores entity, player id and name; every rejection WARN (:199-273) gets them plus `reason` ∈ {`not_started`, `unknown_wire`, `already_cut`, `playfield_inactive`, `illegal_prefix`, `unknown_command`}.
- **Test:** LogCapture in `livewire/tests.rs`: `processmove` with an unknown wire gives `entity_id` and `reason=unknown_wire`. **Ship:** `telemetry-gaps/TG-MG-05-livewire-identity` · `tg-mg-05` · `feat(minigame): TG-MG-05 identity and reasons on Livewire rejections`

### TG-MG-06 `minigame.connection` span

W3 · med · packet-coder · BlockedDependency (TG-MG-03) · crates `cimmeria-minigame` · docs: none
- **Change:** `server/mod.rs` (`handle_connection`): `info_span!("minigame.connection", peer, entity_id = Empty, player_id = Empty, game = Empty)`, `.instrument()` the body, `record` after login.
- **Test:** span-recording layer: a framing send-error row inside a session has the span as parent. **Ship:** `telemetry-gaps/TG-MG-06-connection-span` · `tg-mg-06` · `feat(minigame): TG-MG-06 span per minigame connection`

### TG-MG-07 Victory rows carry `validation`

W3 · low · packet-coder · BlockedDependency (TG-MG-06) · crates `cimmeria-minigame` · docs: none
- **Change:** `crates/minigame/src/minigame/games/mod.rs`: `pub fn validation(game_name) -> &'static str` (`server` for Livewire, `client_declared` for placeholders); `server/mod.rs` victory and failure rows carry it; the unknown-type WARN (:20) gains `entity_id`.
- **Test:** unit for `validation`; a server test that plays `Hack`, sends `victory`, asserts `validation=client_declared`. **Ship:** `telemetry-gaps/TG-MG-07-victory-validation` · `tg-mg-07` · `feat(minigame): TG-MG-07 say who validated a minigame victory`

### TG-MG-08 Login row: peer, player, ticket age

W3 · low · packet-coder · Ready · crates `cimmeria-minigame` · docs: none
- **Change:** `crates/minigame/src/minigame/server/handshake.rs` (`read_and_handle_login`): `peer`, `player_id`, `player_name`, `ticket_age_ms` (`session.created_at.elapsed()`), `event = "minigame.claimed"`.
- **Test:** listener test asserting the fields on the login row. **Ship:** `telemetry-gaps/TG-MG-08-login-row` · `tg-mg-08` · `feat(minigame): TG-MG-08 peer, player and ticket age on minigame login`

### TG-MG-09 Cell result for an unknown entity

W3 · low · packet-coder · Ready · crates `cimmeria-cell` · docs: neg
- **Change:** `crates/cell/src/cell/service/base_messages/minigame.rs`: when `get_entity` misses on a victory (today `.unwrap_or(0)` fires chains with player 0), WARN `reason = "entity_missing"`, `entity_id`, `chain_count`, `result`. Behaviour unchanged.
- **Test:** LogCapture in `base_messages/tests/minigame.rs`. **Ship:** `telemetry-gaps/TG-MG-09-result-unknown-entity` · `tg-mg-09` · `feat(cell): TG-MG-09 warn on a minigame result for an unknown entity`

### TG-MG-10 `pending_sessions` on the D-TG1 refusal

Aliases: T10. W3 · low · packet-coder · BlockedDependency (PR #1348) · crates `cimmeria-minigame` · docs: none
- **Change:** `crates/minigame/src/minigame/server/accept.rs` and `session.rs` (`pending_count()`): `pending_sessions` on the throttled `unexpected_peer` INFO; drop the per-socket DEBUG duplicate.
- **Test:** listener test with one pending session and an unexpected peer: `pending_sessions=1`. **Ship:** `telemetry-gaps/TG-MG-10-pending-sessions` · `tg-mg-10` · `feat(minigame): TG-MG-10 pending session count on refused peers`

### Missions and content

### TG-MIS-01 Identity and `event=` on mission lifecycle rows

W3 · high · packet-coder · Ready · crates `cimmeria-cell-content` · docs: nt
- **Change:** `crates/cell-content/src/cell/missions/lifecycle.rs` (`accept_mission`, `abandon_mission`) and `progression.rs` (`advance_step`, `complete_objective`, `complete_mission_direct`): identity and `world` on every INFO/WARN; `event` ∈ {`mission_accepted`, `mission_accept_refused`, `mission_abandoned`, `mission_step_advanced`, `objective_completed`, `mission_completed` with `path = auto|direct`}; bare `old_step_id`/`current_step_id`; silent `None => return` at `lifecycle.rs:118`, `progression.rs:94, :482` → WARN `reason = "entity_missing"`.
- **Test:** LogCapture in each file's tests: `event` and numeric `player_id` on accept, advance, complete. **Ship:** `telemetry-gaps/TG-MIS-01-mission-lifecycle-identity` · `tg-mis-01` · `feat(cell-content): TG-MIS-01 identity and events on mission lifecycle rows`

### TG-MIS-02 Identity and context on `playtest.friction` rows

W3 · high · packet-coder · BlockedDependency (TG-NET-17) · crates `cimmeria-cell-world` · docs: none
- **Change:** `crates/cell-world/src/cell/playtest_friction_watch/mod.rs` (`PlayerWatch`, `player_tick`, `emit`, `objectives_never_completed`) and `crates/cell-world/src/cell/playtest_friction.rs` (four `emit` sites): cache `PlayerIdentity` and world; emit identity and `world` on every row; `step_stalled` also gets `x`, `z`, `open_objectives`.
- **Test:** extend `playtest_friction_watch/tests.rs`: drive past `STEP_STALL_AFTER`, the WARN has `player_id` and `world`. **Ship:** `telemetry-gaps/TG-MIS-02-friction-identity` · `tg-mis-02` · `feat(cell-world): TG-MIS-02 identity and context on friction rows`

### TG-MIS-03 `mission.step_anchor` row at step activation

W3 · high · `needs-domain-agent: mission-systems-advisor` (design), then packet-coder · BlockedDependency (TG-MIS-09) · crates `cimmeria-content-engine`, `cimmeria-cell-content`, `cimmeria-server` · docs: obs
- **Change:** `crates/content-engine/src/chain/mod.rs`: `ChainEngine::step_anchors(mission_id, step_id) -> StepAnchors { advancers, tags, templates, dialog_ids }`. `crates/cell-content/src/cell/content/event_dispatch/step_activation/mod.rs`: resolve to entities in the player's space, emit INFO `target: "mission.step_anchor"`, `event = "step_anchor"`, identity, mission and step pairs, `advancers`, `anchors`, up to 5 `anchor_ids`/`anchor_tags`/`anchor_in_witness_set`/`anchor_distance`. Pin `mission.step_anchor=info`. Catches #1341 (anchor never created) and step 4037 (no advancer) before anyone stalls. The agent may split it into engine and dispatch packets.
- **Test:** unit `step_anchors` in `chain/tests.rs`; LogCapture asserts `advancers = 0` for an ungated step. **Ship:** `telemetry-gaps/TG-MIS-03-step-anchor` · `tg-mis-03` · `feat(cell-content): TG-MIS-03 log each step's anchors at activation`

### TG-MIS-05 `missions_restored` login snapshot

W3 · med · packet-coder · BlockedDependency (TG-CMB-05) · crates `cimmeria-cell` · docs: none
- **Change:** `crates/cell/src/cell/service/base_messages/player_init/mod.rs`, after `build_restored_missions`: INFO `event = "missions_restored"`, identity, `world`, `saved_count`, `active` (`"622:2113,1360:4037"`, hidden marked `h`), `completed_count`, `failed_count`.
- **Test:** LogCapture on the `InitPlayerState` fixture asserting `active`. **Ship:** `telemetry-gaps/TG-MIS-05-missions-restored` · `tg-mis-05` · `feat(cell): TG-MIS-05 snapshot restored missions at login`

### TG-MIS-06 Executor mission rows carry `player_id`

W3 · med · packet-coder · Ready · crates `cimmeria-cell-content` · docs: nt
- **Change:** `crates/cell-content/src/cell/content/executor/mission.rs` (nine rows): `player_id`, `account_id`, `player_name`; `world` and entity pair on `No mission_defs entry` (:105).
- **Test:** LogCapture in `offer_guard_tests`: `player_id` on `Content: accepting mission`. **Ship:** `telemetry-gaps/TG-MIS-06-executor-identity` · `tg-mis-06` · `feat(cell-content): TG-MIS-06 identity on content mission rows`

### TG-MIS-08 WARN on dropped mission client frames

W3 · med · packet-coder · BlockedDependency (TG-MIS-01) · crates `cimmeria-cell-content` · docs: neg
- **Change:** `crates/cell-content/src/cell/missions/mod.rs`: `send_mission_frame(tx, entity_id, method_index, args, frame, mission_id)` WARNs `reason = "base_channel_closed"` with `frame` and identity. Replace the `let _ =` sends at `lifecycle.rs:153,165,180,288` and `progression.rs:218,231,367,421,440,562,576,590`.
- **Test:** LogCapture with a closed receiver on `accept_mission`: WARN `frame = "onMissionUpdate"`. **Ship:** `telemetry-gaps/TG-MIS-08-mission-frame-drops` · `tg-mis-08` · `feat(cell-content): TG-MIS-08 warn on dropped mission journal frames`

### TG-MIS-09 Numeric ids on `content.resolve`

W3 · med · packet-coder · Ready · crates `cimmeria-content-engine` · docs: nt
- **Change:** `crates/content-engine/src/chain/mod.rs` (:456-490): `source_entity = ?ctx.source_entity_id` → `entity_id = ctx.source_entity_id.map(|e| e.0)`; add `world_id`.
- **Test:** LogCapture in `chain/tests.rs`: a failing condition logs numeric `entity_id`. **Ship:** `telemetry-gaps/TG-MIS-09-resolve-ids` · `tg-mis-09` · `fix(content-engine): TG-MIS-09 numeric entity id on content.resolve`

### TG-MIS-10 Malformed-args and missing-player rows on interact and dialog choice

W3 · low · packet-coder · Ready · crates `cimmeria-cell-methods` · docs: neg
- **Change:** `crates/cell-methods/src/cell/cell_methods/player/interaction/dialog.rs` (:19, :106) and `interact.rs` (:18, :170, :211): short payload → WARN `reason = "malformed_args"`, `args_len`, `need`; missing `player_id` → WARN `reason = "player_id_missing"` before the chain fires (log only).
- **Test:** LogCapture in `dialog_choice_gate_tests.rs` with a 4-byte payload. **Ship:** `telemetry-gaps/TG-MIS-10-interact-malformed` · `tg-mis-10` · `feat(cell-methods): TG-MIS-10 log malformed interact and dialog calls`

### TG-MIS-11 Base mission persist rows: step, reason, error

W3 · low · packet-coder · BlockedDependency (TG-DB-07) · crates `cimmeria-base-methods` · live-DB · docs: neg
- **Change:** `crates/base-methods/src/base/world_entry/methods/missions/mod.rs`: `current_step_id` and name on `Mission state persisted`; failures get `reason` ∈ {`db_write_failed`, `query_failed`}, `error`, `status`, `current_step_id`; no-pool read gives DEBUG `reason = "no_db_pool"`.
- **Test:** live-DB test in `missions/tests.rs` with LogCapture on `current_step_id`. **Ship:** `telemetry-gaps/TG-MIS-11-mission-persist-rows` · `tg-mis-11` · `feat(base-methods): TG-MIS-11 step and reason on mission persist rows`

### TG-MIS-13 OTEL row for content-engine conditions; drop the dead `mission` target

W3 · low · packet-coder · Ready · crates `cimmeria-server` · docs: obs
- **Change:** `crates/server/src/logging/filters.rs`: add `cimmeria_content_engine::conditions=debug`, remove the dead `mission=info`; `crates/server/src/logging/parity_tests/crate_rows.rs`: move content-engine out of `NO_OWN_ROW`. Fail-closed conditions are invisible today.
- **Test:** existing parity and crate-row guards, plus `content_engine_condition_reaches_the_server_index`. **Ship:** `telemetry-gaps/TG-MIS-13-content-engine-row` · `tg-mis-13` · `fix(server): TG-MIS-13 export content-engine condition rows`

### Movement and teleport

### TG-MOV-01 Truthful, joinable forced-position row

W3 · high · packet-coder · Ready · crates `cimmeria-base-world-entry` · docs: neg
- **Change:** `crates/base-world-entry/src/base/world_entry/teleport.rs` (`handle_teleport_player`): bind the `BundleSendOutcome` (:118); read `player_class_id` in the lock at :68. `wire.out.forced_position` gains `send`, `base_seq`, `packets`, `client_class` (`SGWPlayer|SGWGmPlayer|unknown`), `streaming_hint = "method_116"`, `player_id`, `player_name`. Not sent → WARN `reason = "send_failed"` with identity, and no "sent" row.
- **Test:** LogCapture beside `teleport_early_returns_*`: GM-class client gives `client_class=SGWGmPlayer send=sent`; a gone client gives the WARN and no "sent" row. **Ship:** `telemetry-gaps/TG-MOV-01-forced-position-row` · `tg-mov-01` · `fix(base-world-entry): TG-MOV-01 forced-position row reports the real send outcome`

### TG-MOV-02 `movement.teleport` row for every server-authoritative move

W3 · high · packet-coder · BlockedDependency (TG-MOV-10) · crates `cimmeria-cell-world`, `cimmeria-server` · docs: obs
- **Change:** `crates/cell-world/src/cell/space_manager/movement_telemetry/mod.rs`: `log_authorized_teleport(&self, entity_id)`; one call line in `client_move.rs::note_authorized_teleport` (671 lines, no logic there). DEBUG `target: "movement.teleport"`, `event = "authorized_teleport"`, identity, `space_id`, `world`, `x`/`y`/`z`, `navmesh_mode`, `dest_on_mesh`, `is_player`; WARN `reason = "teleport_dest_off_mesh"` and counter `movement_teleport_dest_off_mesh_total{world}` for a player off-mesh in an enforce space. Pin `movement.teleport=debug`.
- **Test:** LogCapture in `movement_telemetry/tests.rs` with the fixture mesh: off-mesh in enforce gives the WARN; on-mesh gives DEBUG only. **Ship:** `telemetry-gaps/TG-MOV-02-authorized-teleport-row` · `tg-mov-02` · `feat(cell-world): TG-MOV-02 log every server-authoritative move`

### TG-MOV-04 Ring passenger release row

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world` · docs: none
- **Change:** `crates/cell-world/src/cell/ring_transport/runtime/teardown.rs` (`dispatch_release_effects`, `ShowPlayer` arm): INFO `event = "ring.passenger_released"`, identity, `world`.
- **Test:** LogCapture feeding `[ShowPlayer{id}, UnlockMovement{id}]`: the row has `account_id`. **Ship:** `telemetry-gaps/TG-MOV-04-ring-release-row` · `tg-mov-04` · `feat(cell-world): TG-MOV-04 one release row per ring passenger`

### TG-MOV-05 Ring FSM transition rows

W3 · med · packet-coder · Ready · crates `cimmeria-cell-content` · docs: none
- **Change:** `crates/cell-content/src/cell/ring_transport/runtime/tick.rs` (`run_one_deadline`): DEBUG `event = "ring.transition"`, region pair, `deadline` (`hide|warmup|remote_warmup|cooldown|stall`), `state_before`, `state_after`, `passengers`; the `return false` arms at :268, :275, :278 log DEBUG `reason = "deadline_unapplied"`.
- **Test:** LogCapture with `ring_transport/tests` fixtures: one hide deadline gives one `ring.transition deadline=hide`. **Ship:** `telemetry-gaps/TG-MOV-05-ring-transitions` · `tg-mov-05` · `feat(cell-content): TG-MOV-05 log ring transport transitions`

### TG-MOV-06 Split `gm/travel.rs`

W3 · low · packet-coder · Ready · crates `cimmeria-cell-console` · docs: none
- **Change:** `crates/cell-console/src/cell/console/gm/travel.rs` (704 lines, over the hard cap) → `gm/travel/{mod.rs, goto_xyz.rs, goto_location.rs, dhd.rs, goto_summon.rs}` with `mod.rs` re-exports. No behaviour change.
- **Test:** `gm/tests/travel.rs` passes unchanged. **Ship:** `telemetry-gaps/TG-MOV-06-split-gm-travel` · `tg-mov-06` · `refactor(cell-console): TG-MOV-06 split gm travel into a directory`

### TG-MOV-07 Rule 5 on GM travel rows

W3 · med · packet-coder · BlockedDependency (TG-MOV-06) · crates `cimmeria-cell-console` · docs: nt
- **Change:** the split `gm/travel/*.rs`: identity quartet on every row (match `console/travel/mod.rs:144`); gmGoto's "caller entity not found" branch gets WARN `reason = "entity_missing"`.
- **Test:** LogCapture running gmGotoXYZ for a seeded GM: `account_id` on "teleporting GM". **Ship:** `telemetry-gaps/TG-MOV-07-gm-travel-identity` · `tg-mov-07` · `feat(cell-console): TG-MOV-07 identity on GM travel rows`

### TG-MOV-08 Content teleport identity; silent move-waypoint return

W3 · med · packet-coder · Ready · crates `cimmeria-cell-content` · docs: nt
- **Change:** `crates/cell-content/src/cell/content/executor/transport.rs` (:46, :114, :160, :202): identity quartet. `content/executor/world/movement.rs:181`: DEBUG `reason = "entity_missing"` before the `return`.
- **Test:** LogCapture running `teleport` for a seeded player: `account_id`. **Ship:** `telemetry-gaps/TG-MOV-08-content-teleport-identity` · `tg-mov-08` · `feat(cell-content): TG-MOV-08 identity on content teleport rows`

### TG-MOV-09 Method 116 to `SGWGmPlayer`: RE the drop

W3 · med · `needs-domain-agent: game-archaeology-specialist` + movement-teleport-advisor · Ready (RE) then D-TG17 · crates `cimmeria-base-world-entry` · docs: RE finding
- **Change:** RE why `SGWGmPlayer` drops `onPlayerTeleport` (195 of 200 `method_dropped` rows) and whether the streaming pre-load is lost; then either skip 116 for class 0x03 (`streaming_hint = "skipped_gm_class"`) or route the hint differently. TG-MOV-01's `client_class` classifies the drop meanwhile.
- **Test:** per the decision, a wire-format test that a GM-class teleport bundle omits or keeps 116. **Ship:** `telemetry-gaps/TG-MOV-09-method-116-gm` · `tg-mov-09` · `fix(base-world-entry): TG-MOV-09 method 116 policy for GM-class clients`

### TG-MOV-12 `navmesh_missing` counter by world

W3 · low · packet-coder · Ready · crates `cimmeria-cell-world` · docs: obs
- **Change:** `crates/cell-world/src/cell/space_manager/lifecycle.rs` (:46, :59): `movement_navmesh_missing_total{world, reason}`, `reason` ∈ {`missing`, `load_failed`}.
- **Test:** create a space for a world with no `.nav` in a temp dir; assert the WARN and the counter. **Ship:** `telemetry-gaps/TG-MOV-12-navmesh-missing-counter` · `tg-mov-12` · `feat(cell-world): TG-MOV-12 count worlds with no navmesh`

### NPC AI and spawns

### TG-NPC-04 A death that will not respawn says so

W3 · med · packet-coder · BlockedDependency (TG-CMB-09) · crates `cimmeria-cell-combat` · docs: obs
- **Change:** `crates/cell-combat/src/cell/combat/state.rs` (`mark_npc_dead`): the INFO gets `event = "respawn_scheduled"`, `world`, `space_id`; new else branch INFO `target: "npc_ai.respawn"`, `event = "respawn_not_scheduled"`, `reason = "content_spawn"` (no `spawn_id`) or `"no_respawn_secs"`, NPC and template pairs, `world`, `space_id`. 22 of 769 deaths last week were silent.
- **Test:** LogCapture `death_without_respawn_secs_logs_reason` and `respawn_scheduled_row_carries_world`. **Ship:** `telemetry-gaps/TG-NPC-04-respawn-not-scheduled` · `tg-npc-04` · `feat(cell-combat): TG-NPC-04 say when a dead NPC will not respawn`

### TG-NPC-05 NPC attack-not-fired: event, throttle, identity

W3 · med · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: neg
- **Change:** `crates/cell-combat/src/cell/service/npc_ai/fight.rs` (:433-450): `target: "npc_ai"`, `event = "attack_not_fired"`, template pair, `world`, `space_id`, Pattern D via `npc_detectors.admit_warn(npc_id, "attack_not_fired", now, 15 s)`. Re-WARNs twice a second today.
- **Test:** LogCapture: 3 fast retries give one WARN then `suppressed=2`; a second NPC's first row isn't swallowed. **Ship:** `telemetry-gaps/TG-NPC-05-attack-not-fired` · `tg-npc-05` · `fix(cell-combat): TG-NPC-05 throttle and name the NPC attack-not-fired warning`

### TG-NPC-06 Instance population summary and unknown-world skips

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world` · docs: neg
- **Change:** `crates/cell-world/src/cell/space_manager/npc_population.rs`: `spawn_instance_npcs_from_records` → INFO `target: "spawner"`, `event = "instance_populated"`, `world`, `space_id`, `record_count`, `spawned`, `failed`; `spawn_npcs_from_records` → one WARN per unknown `world_name`, `event = "spawn_world_unknown"`, `records`.
- **Test:** LogCapture: summary fields; a `"NoSuchWorld"` record WARNs once. **Ship:** `telemetry-gaps/TG-NPC-06-instance-populated` · `tg-npc-06` · `feat(cell-world): TG-NPC-06 instance population summary`

### TG-NPC-07 `player_id` holding an entity id

W3 · med · packet-coder · BlockedDependency (TG-NPC-03) · crates `cimmeria-cell-world`, `cimmeria-cell-combat` · docs: neg, nt
- **Change:** `crates/cell-world/src/cell/service/npc_ai/detectors/aggro_scan.rs:131` (`candidate_rejected`, assist rows): key → `witness_id` + `witness_name`; `crates/cell-combat/src/cell/service/npc_ai/leash/begin.rs:30`: → `entity_id` + `entity_name`; both add the identity quartet. Rename in the `neg` key-change table.
- **Test:** LogCapture per row: `player_id` equals the sgw player id, not the entity id. **Ship:** `telemetry-gaps/TG-NPC-07-player-id-keys` · `tg-npc-07` · `fix(npc-ai): TG-NPC-07 stop logging entity ids as player_id`

### TG-NPC-08 Target player identity on leash, stuck and damage_ignored

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world` · docs: nt
- **Change:** `crates/cell-world/src/cell/service/npc_ai/detectors/leash.rs` (`on_enter`, `loop`, `on_damage_while_leashing`) and `detectors/sweep.rs` (`stuck`): `target_account_id`, `target_player_id` and names; `damage_ignored` uses the `attacker_` prefix.
- **Test:** LogCapture in `detector_tests/leash.rs`: fields present for a player target, absent for an NPC. **Ship:** `telemetry-gaps/TG-NPC-08-leash-target-identity` · `tg-npc-08` · `feat(cell-world): TG-NPC-08 target player identity on leash and stuck rows`

### TG-NPC-10 Cover decisions and `path_fail` attributable

W3 · low · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: nt
- **Change:** `crates/cell-combat/src/cell/service/npc_ai/fight_cover.rs` (:129, :144, :256) and `npc_ai/path_failure/mod.rs`: `world` (via `world_label`), `space_id`, template pair.
- **Test:** extend the cover and path-fail LogCapture tests to assert `world`. **Ship:** `telemetry-gaps/TG-NPC-10-cover-path-fail-world` · `tg-npc-10` · `feat(cell-combat): TG-NPC-10 world and template on cover and path-fail rows`

### TG-NPC-11 Zone on tick rows; aggro outcome slot

W3 · low · packet-coder · BlockedDependency (TG-NPC-01) · crates `cimmeria-cell-combat`, `cimmeria-cell` (test) · docs: obs
- **Change:** `crates/cell-combat/src/cell/service/npc_ai/dispatch.rs` (`log_ai_tick`): `world`, `space_id`. `npc_ai/idle_aggro.rs`: `record_decision_outcome("aggro_acquired")` on engage (1,728 rows read `outcome=""`). Add the outcome to the catalog enum.
- **Test:** `tick_row.rs`: `world` on the row; the engaging tick has `decision_outcome = "aggro_acquired"`. **Ship:** `telemetry-gaps/TG-NPC-11-tick-world` · `tg-npc-11` · `feat(npc-ai): TG-NPC-11 world on tick rows and the aggro outcome`

### TG-NPC-12 Respawn tick negative gaps

W3 · low · packet-coder · Ready · crates `cimmeria-cell` · docs: neg
- **Change:** `crates/cell/src/cell/service/ticks/npc_respawn/mod.rs`: :277 `let _` → WARN `reason = "base_channel_closed"` (`onLootDisplay` close); WARN `event = "respawn_no_spawn_position"` when `spawn_pos` is `None`. Target `spawner.npc_respawn`, NPC and template pairs.
- **Test:** LogCapture in `npc_respawn/tests/`: dropped receiver and no-position NPC each give a row. **Ship:** `telemetry-gaps/TG-NPC-12-respawn-tick-gaps` · `tg-npc-12` · `feat(cell): TG-NPC-12 log respawn sends and missing spawn positions`

### TG-NPC-13 DoT kill-credit miss WARN

W3 · low · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: neg
- **Change:** `crates/cell-combat/src/cell/effects/pulsing/pulse.rs` (:141-145): mirror `kill_credit.rs:171`: WARN `abilities`, `event = "kill_credit_no_player"`, `reason = "no_credited_player"`, `source = "dot"`, tag and invoker identity.
- **Test:** LogCapture pulse test with an invoker pet whose owner is gone. **Ship:** `telemetry-gaps/TG-NPC-13-dot-kill-credit` · `tg-npc-13` · `feat(cell-combat): TG-NPC-13 warn when a DoT kill credits no player`

### TG-NPC-15 `spawner.npc_behaviour` sentinels and Debug options

W3 · low · packet-coder · BlockedDependency (TG-NPC-06) · crates `cimmeria-cell-world` · docs: none
- **Change:** `npc_population.rs` (:138-175): `world`, `space_id`, `template_id` as `Option`s (no `""`/`0`); `respawn_secs`, `aggression_override` as numbers.
- **Test:** LogCapture on a fixture NPC: numeric `respawn_secs`; no `world` for an unregistered space. **Ship:** `telemetry-gaps/TG-NPC-15-npc-behaviour-fields` · `tg-npc-15` · `fix(cell-world): TG-NPC-15 no sentinels on spawner.npc_behaviour`

### Social

### TG-SOC-04 Presence skips Ignore-list watchers

W3 · high · `needs-domain-agent: social-systems-engineer` (+ RE or lab check of the Ignore panel) · BlockedDecision (D-TG10) and BlockedDependency (TG-SOC-05) · crates `cimmeria-base-session` · live-DB · docs: neg
- **Change (behaviour):** filter flags-301 lists out of `find_watchers` and `notify_online_contacts` (`crates/base-session/src/base/contact_list/persistence/mod.rs`).
- **Test:** live-DB `live_db_ignorer_gets_no_presence`: an ignorer receives no CM 89. **Ship:** `telemetry-gaps/TG-SOC-04-presence-ignore` · `tg-soc-04` · `fix(base-session): TG-SOC-04 presence skips players who ignore the subject`

### TG-SOC-05 Presence fan-out counts, including ignoring watchers

W3 · med · packet-coder · Ready · crates `cimmeria-base-session` · live-DB · docs: none
- **Change:** `contact_list/persistence/mod.rs`: `find_watchers_with_flags` returns `(player_id, flags)`; `crates/base-session/src/base/contact_list/handlers/presence_fanout.rs` (`fanout_contact_event`) ends with DEBUG `event = "contacts.presence_fanout"`, `watchers`, `ignoring_watchers`, `online_watchers`, `sent`, subject pair; zero watchers logged too.
- **Test:** live-DB: a subject in one friend list and one Ignore list gives `ignoring_watchers = 1`. **Ship:** `telemetry-gaps/TG-SOC-05-presence-counts` · `tg-soc-05` · `feat(base-session): TG-SOC-05 presence fan-out counts`

### TG-SOC-06 Cell trade events, identity and cancel reason

W3 · med · packet-coder · Ready · crates `cimmeria-cell-interactions`, `cimmeria-cell-methods` · docs: neg
- **Change:** `crates/cell-interactions/src/cell/trade/state.rs` (`cancel_session` takes `reason` ∈ {`user_cancel`, `bad_proposal`, `out_of_range`, `handoff_out_of_range`, `disconnect`, `send_failed`}), `crates/cell-methods/src/cell/cell_methods/player/trade/handlers.rs`, `trade/handoff.rs`: `event` ∈ {`trade.session_opened`, `trade.session_refused`, `trade.session_cancelled`, `trade.execute_requested`}; identity.
- **Test:** extend the handler tests: a disconnect cancel has `reason=disconnect`. **Ship:** `telemetry-gaps/TG-SOC-06-trade-events` · `tg-soc-06` · `feat(cell-interactions): TG-SOC-06 events and cancel reasons on trade rows`

### TG-SOC-07 Spatial chat send failures and identity

W3 · med · packet-coder · Ready · crates `cimmeria-cell-console` · docs: neg
- **Change:** `crates/cell-console/src/cell/console/chat/spatial.rs` (:124, :156): `let _` → WARN `event = "chat.spatial_send_failed"`, `reason = "base_channel_closed"`; broadcast row (:112) gets `event = "chat.spatial_broadcast"`, identity, `ignored`.
- **Test:** LogCapture with a dropped receiver gives the WARN. **Ship:** `telemetry-gaps/TG-SOC-07-spatial-chat` · `tg-soc-07` · `feat(cell-console): TG-SOC-07 log failed spatial chat sends`

### TG-SOC-08 Base chat malformed and relay refusals

W3 · med · packet-coder · Ready · crates `cimmeria-base` · docs: neg
- **Change:** new `crates/base/src/base/dispatch/chat_malformed.rs` (rows live here; `chat.rs` is 977 lines), called from `crates/base/src/base/dispatch/chat.rs` (:113-138, :444-451 → WARN `event = "chat.malformed"`, `reason` ∈ {`empty_payload`, `bad_target`, `bad_text`, `bad_channel_name`}, `payload_len`; :326-328, :683 → WARN `event = "chat.relay_failed"`, `reason` ∈ {`cell_channel_closed`, `entity_missing`}); register in `dispatch/mod.rs`.
- **Test:** LogCapture, one case per reason. **Ship:** `telemetry-gaps/TG-SOC-08-chat-refusals` · `tg-soc-08` · `feat(base): TG-SOC-08 log malformed and unrelayed chat`

### TG-SOC-09 Cell contact-list short-args rows

W3 · med · packet-coder · Ready · crates `cimmeria-cell-methods` · docs: neg
- **Change:** `crates/cell-methods/src/cell/cell_methods/contact_list/mod.rs` (:150, :189, :239, :281, :331): WARN `event = "contacts.call_malformed"`, `reason = "malformed_args"`, `op`, `args_len`, `need`, entity pair, `account_id`.
- **Test:** drive `dispatch` with 2-byte args per op. **Ship:** `telemetry-gaps/TG-SOC-09-contact-short-args` · `tg-soc-09` · `feat(cell-methods): TG-SOC-09 log malformed contact-list calls`

### TG-SOC-10 Contact header ops: `event`, `account_id`, create reason

W3 · med · packet-coder · Ready · crates `cimmeria-base-session` · live-DB · docs: neg
- **Change:** `crates/base-session/src/base/contact_list/handlers/header_ops.rs`: `event` ∈ {`contacts.list_created`, `list_deleted`, `list_renamed`, `list_flags_updated`}, `account_id`/`account_name`; create failure classified with `is_unique_violation()` → `reason = "duplicate_name"`, else `"db_write_failed"`.
- **Test:** live-DB duplicate create gives `reason=duplicate_name`. **Ship:** `telemetry-gaps/TG-SOC-10-contact-header-events` · `tg-soc-10` · `feat(base-session): TG-SOC-10 events and reasons on contact list ops`

### TG-SOC-12 Discord drop accounting

W3 · med · packet-coder · Ready · crates `cimmeria-discord` · docs: discord, obs
- **Change:** `crates/discord/src/sender/handle.rs`, `sender/task.rs`, `sender/stats.rs`: counter `discord_events_total{outcome}` (`sent`, `filtered`, `dropped_full`, `dropped_closed`, `no_webhook`, `rate_limited`, `failed`); throttled WARN `event = "discord.dropped"` on `cimmeria_discord` (first per outcome, then one per 60 s with `suppressed`).
- **Test:** stalled `MockSender`, fill the queue: one WARN `suppressed=0`, counter; burst and independence per Pattern D. **Ship:** `telemetry-gaps/TG-SOC-12-discord-drop-accounting` · `tg-soc-12` · `feat(discord): TG-SOC-12 count and warn on dropped Discord events`

### TG-SOC-14 User-channel teardown leaves

W3 · low · packet-coder · BlockedDependency (TG-NET-17) · crates `cimmeria-base-session`, `cimmeria-base` · docs: none
- **Change:** `crates/base-session/src/base/user_channels/registry.rs` (`leave_all` returns `Vec<(u8, String, bool)>`); `session_teardown.rs` and `dispatch/session.rs` log DEBUG `event = "chat.channel_left"`, `reason = "session_end"`, `wire_id`, `display_name`, `deleted`, identity.
- **Test:** registry unit on the return; LogCapture in the existing teardown test. **Ship:** `telemetry-gaps/TG-SOC-14-channel-leaves` · `tg-soc-14` · `feat(base-session): TG-SOC-14 log user-channel leaves at session end`

### TG-SOC-15 Death presence without a character name

W3 · low · packet-coder · BlockedDependency (TG-CMB-06) · crates `cimmeria-cell-combat` · docs: none
- **Change (small behaviour):** `crates/cell-combat/src/cell/abilities/death/mod.rs:85`: when `character_name` is `None`, skip the `entity:{eid}` placeholder send and log DEBUG `event = "contacts.presence_skipped"`, `reason = "no_character_name"`.
- **Test:** an unnamed player test entity's death sends no `ContactListPresenceEvent` and logs the row. **Ship:** `telemetry-gaps/TG-SOC-15-death-presence-name` · `tg-soc-15` · `fix(cell-combat): TG-SOC-15 skip death presence with no character name`

### Auth and network

### TG-NET-04 Phase 1/2 refusals each log a reason

W3 · med · packet-coder · Ready · crates `cimmeria-auth` · docs: neg
- **Change:** `crates/auth/src/auth/handlers.rs`: INFO with `reason` and `peer_ip` at `unknown_sku` (:106), `plaintext_length` (:140), `no_shards` (WARN, :215), `sid_missing` (:284), `sid_unknown` (:295), `sid_expired` (:292, with account and `sid_age_secs`), `unknown_shard` (:332). Rename `user=` → `account_name=` at :134, :162, :173, :183, :191, :207.
- **Test:** POST Phase 2 with no cookie → `reason=sid_missing`; unknown SKU → `unknown_sku`. **Ship:** `telemetry-gaps/TG-NET-04-auth-refusal-reasons` · `tg-net-04` · `feat(auth): TG-NET-04 reasons on every login refusal`

### TG-NET-05 Reaper logs each unconsumed ticket and SID

W3 · med · packet-coder · Ready · crates `cimmeria-auth` · docs: neg
- **Change:** `crates/auth/src/auth/service.rs`: extract `reap_expired(sessions, pending, now)`; drain and log each: INFO `reason = "ticket_unconsumed" | "sid_unconsumed"`, account pair, `issued_ip`, `age_ms`, `ticket_prefix`. Keep the DEBUG summary.
- **Test:** `reap_expired` with a ticket backdated 31 s gives the INFO with `account_id`. **Ship:** `telemetry-gaps/TG-NET-05-reaper-rows` · `tg-net-05` · `feat(auth): TG-NET-05 log each expired ticket and SID`

### TG-NET-06 Phase 3 refusal context; account-arm identity

Aliases: TG-NET-07 (the `connect_loop/mod.rs:88` half moved to TG-MER-11). W3 · med · packet-coder · BlockedDependency (TG-MER-06) · crates `cimmeria-base` · docs: neg, nt
- **Change:** `crates/base/src/base/login/mod.rs`: `%addr` and `reason = "ticket_unknown"` at :64, :102; `%addr` at :179; account pair at :285; `%addr` + account at :581. `crates/base/src/base/connect_loop/account_arms.rs` (:122, :134, :173): account pair from `identity_for_addr`.
- **Test:** LogCapture: `handle_login` with an unknown ticket gives WARN with `addr` and `reason=ticket_unknown`; `createCharacter` dispatch has `account_id`. **Ship:** `telemetry-gaps/TG-NET-06-phase3-context` · `tg-net-06` · `feat(base): TG-NET-06 address, account and reason on login rows`

### TG-NET-09 GM gate and lab console rows carry identity and reason

W3 · med · packet-coder · Ready · crates `cimmeria-cell-world`, `cimmeria-cell` · docs: nt
- **Change:** `crates/cell-world/src/cell/dispatch/gm_gate.rs` and `crates/cell/src/cell/service/base_messages/lab_console.rs`: identity quartet on the authorized INFO and both rejects; `reason = "entity_missing"` when `get_entity` is `None`, `"not_gm"` otherwise.
- **Test:** LogCapture in the gm_gate tests: a known non-GM gives WARN `player_id`, `reason=not_gm`; a missing entity gives `entity_missing`. **Ship:** `telemetry-gaps/TG-NET-09-gm-gate-identity` · `tg-net-09` · `feat(cell-world): TG-NET-09 identity and reason on GM gate rows`

### TG-NET-12 Rate-limited INFO for datagrams from unknown addresses

W3 · low · packet-coder · BlockedDependency (TG-MER-11) · crates `cimmeria-base` · docs: neg
- **Change:** `crates/base/src/base/connect_loop/mod.rs:213`: TRACE → INFO `event = "unknown_peer_datagram"`, at most once per addr per 60 s from a bounded map (≤ 256), `addr`, `flags`, `suppressed`. A reaped client still sending shows here.
- **Test:** unit on the limiter: burst and independence. **Ship:** `telemetry-gaps/TG-NET-12-unknown-peer-datagrams` · `tg-net-12` · `feat(base): TG-NET-12 log datagrams from unknown peers, rate limited`

### Persistence

### TG-DB-05 Slow cell-to-base dispatch WARN

W3 · med · packet-coder · BlockedDependency (TG-NET-10) · crates `cimmeria-wire`, `cimmeria-base` · docs: neg
- **Change:** new `crates/wire/src/cell/messages/cell_to_base_kind.rs` (`CellToBaseMsg::kind() -> &'static str`, exhaustive); new `crates/base/src/base/dispatch_timing.rs`; `crates/base/src/base/service.rs` (:312-326) times each `route_cell_message`. Over 250 ms: WARN `event = "cell_dispatch_slow"`, `msg_kind`, `elapsed_ms`, `queue_depth`, Pattern D per kind (10 s).
- **Test:** LogCapture on the throttle-and-emit fn (burst, independence); `kind()` distinct for three variants. **Ship:** `telemetry-gaps/TG-DB-05-slow-cell-dispatch` · `tg-db-05` · `feat(base): TG-DB-05 warn on slow cell-to-base dispatch`

### TG-DB-06 Pool configuration, saturation WARN, slow-statement WARN

W3 · med · packet-coder · Ready · crates `cimmeria-services`, `cimmeria-server` · docs: obs
- **Change:** `crates/services/src/database.rs`: `PgPoolOptions` (max 10, 30 s acquire, as today) and `log_slow_statements(Warn, 250 ms)`; INFO `event = "db_pool_configured"`. New `crates/services/src/db_pool_monitor.rs`: 15 s sampler, target `db.pool`, DEBUG `size`/`idle`, WARN `event = "db_pool_saturated"` (Pattern D). Pin `db.pool=debug`.
- **Test:** LogCapture on the pure `sample(size, idle, max, &mut state)`: saturation WARNs, the next is suppressed, unsaturated resets. **Ship:** `telemetry-gaps/TG-DB-06-pool-monitor` · `tg-db-06` · `feat(services): TG-DB-06 pool configuration, saturation and slow-statement rows`

### TG-DB-08 Character-create refusals carry `event` and `reason`

W3 · med · packet-coder · Ready · crates `cimmeria-base` · docs: neg
- **Change:** new `crates/base/src/base/character_create/refusal_log.rs` (`log_refusal(addr, account_id, account_name, reason, error_code)`); every early return in `character_create/mod.rs` (:79-212, :262, :319-381, :630-640) calls it with `event = "character_create_failed"` and a stable `reason` (`name_parse_failed`, `name_rejected`, `extra_name_rejected`, `malformed_args`, `invalid_skin_tint`, `unknown_char_def`, `no_db_pool`, `query_failed`, `invalid_visual_group`, `forced_group_choice`, `invalid_choice`, `missing_optional_choice`, `name_taken`, `db_write_failed`). Levels unchanged.
- **Test:** `fail_code_tests`: `unknown_char_def` and `invalid_skin_tint` assert `reason` and `account_id`. **Ship:** `telemetry-gaps/TG-DB-08-character-create-refusals` · `tg-db-08` · `feat(base): TG-DB-08 reasons on character-create refusals`

### TG-DB-09 Startup cache-load failures and summary

W3 · low · packet-coder · BlockedDependency (TG-CMB-04, TG-DB-05) · crates `cimmeria-cell`, `cimmeria-base` · docs: neg
- **Change:** new `crates/cell/src/cell/service/startup_report.rs` (`cache_failed(cache, error, consequence)`); replace each `warn!("Failed to load …")` in `startup.rs` (738 lines; this shrinks it) and `crates/base/src/base/service.rs:251`. WARN `event = "cache_load_failed"`, `cache`, `error`, `consequence`; one INFO `event = "startup_caches"`, `failed_count`, `failed_caches`.
- **Test:** LogCapture on the helper and summary: two failures give `failed_count = 2`. **Ship:** `telemetry-gaps/TG-DB-09-startup-cache-report` · `tg-db-09` · `feat(cell): TG-DB-09 structured cache-load failures and a boot summary`

### Telemetry pipeline (server side)

### TG-PIPE-10 Busy refusal names its pool

W3 · med · packet-coder · BlockedDependency (TG-NET-17 for `session_kind` wording only) · crates `cimmeria-admin-api` · docs: dst
- **Change:** `crates/admin-api/src/routes/telemetry/upload_slots.rs` (`take_slot` → `Result<_, BusyPool>`), `upload_gate.rs:332`, `dto.rs` (`Busy { pool, limit }`): the refusal row has `pool` (`peer_share|route`), `limit`, `cimmeria.session_kind`.
- **Test:** the `upload_limits_tests::gate` busy case asserts `pool="peer_share"`. **Ship:** `telemetry-gaps/TG-PIPE-10-busy-pool` · `tg-pipe-10` · `feat(admin-api): TG-PIPE-10 busy refusals name their pool`

### TG-PIPE-15 Bundle lines carry a parsed `level`

W3 · low · packet-coder · BlockedDependency (TG-PIPE-01) · crates `cimmeria-admin-api` · docs: dst
- **Change:** `crates/admin-api/src/routes/telemetry/bundle_unzip.rs:189`: parse the log4cxx level token after the timestamp into `level`; severity unchanged.
- **Test:** LogCapture on one ERROR line: `level="ERROR"`. **Ship:** `telemetry-gaps/TG-PIPE-15-bundle-level` · `tg-pipe-15` · `feat(admin-api): TG-PIPE-15 parse the level of replayed bundle lines`

### TG-PIPE-16 `entity_labels_unavailable` carries `session_id`

W3 · low · packet-coder · Ready · crates `cimmeria-admin-api` · docs: none
- **Change:** `crates/admin-api/src/routes/telemetry/entity_labels.rs:384`: pass `sid` into the row.
- **Test:** LogCapture on `resolve` with a busy permit. **Ship:** `telemetry-gaps/TG-PIPE-16-labels-session` · `tg-pipe-16` · `fix(admin-api): TG-PIPE-16 session id on entity_labels_unavailable`

---

## W4: Client DLL and launcher (D-TG4)

Every DLL packet ships with the next signed launcher release, after a lab load check of the built DLL. `cimmeria-client-telemetry` and `sgw-launcher` are outside the CI workspace, so their lane checks are the only build proof. Server-side halves (TG-PIPE-04) merge independently.

### TG-PIPE-02 DLL sends its pid with every chunk

W4 · high · packet-coder · Ready · crates `cimmeria-client-telemetry` · docs: client, dst
- **Change:** `crates/client-telemetry/src/uploader.rs` (`post_batch`): header `X-Cimmeria-Client-Pid: <GetCurrentProcessId>`; `boot.rs`: `pid` on `client.dll.attached`. One lab `session_id` spans up to 29 launches today.
- **Test:** extend `uploader_ships_batch_to_local_server` to assert the header. **Ship:** `telemetry-gaps/TG-PIPE-02-dll-pid` · `tg-pipe-02` · `feat(client-telemetry): TG-PIPE-02 send the process id with each chunk`

### TG-PIPE-04 Ingest stamps `client_pid`

W4 (server) · high · packet-coder · Ready · crates `cimmeria-admin-api` · docs: obs, dst
- **Change:** `crates/admin-api/src/routes/telemetry/chunk.rs` (read and validate the header as u32), `replay.rs` and `replay_native.rs` (`client_pid` record attribute). Respect the 32-field cap. Absent header → attribute absent, never 0.
- **Test:** `client_index_tests`-style replay with and without the header. **Ship:** `telemetry-gaps/TG-PIPE-04-ingest-client-pid` · `tg-pipe-04` · `feat(admin-api): TG-PIPE-04 stamp client_pid on replayed rows`

### TG-PIPE-03 DLL final flush at process exit

W4 · high · `needs-domain-agent: game-archaeology-specialist` (exit hook RE) then packet-coder · BlockedDependency (TG-PIPE-02) · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/boot.rs:342` (a real stop flag), `uploader.rs` (bounded final POST, at most 1.5 s for `governor_finish`), an exit hook (IAT `ExitProcess` or a UE3 shutdown site; RE picks). Final health row has `final: true`.
- **Test:** `run_uploader` with a stop callback: the last POST holds a health row with `final: true`. **Ship:** `telemetry-gaps/TG-PIPE-03-dll-final-flush` · `tg-pipe-03` · `feat(client-telemetry): TG-PIPE-03 flush telemetry at process exit`

### TG-CLI-04 `client.session.end` from the DLL

Aliases: T5 (client half). W4 · high · packet-coder · BlockedDependency (TG-PIPE-03) · crates `cimmeria-client-telemetry`, `cimmeria-admin-api` · docs: client
- **Change:** `boot.rs`: on the TG-PIPE-03 exit path emit `client.session.end {reason}` (`exit_process`, `ue3_shutdown`, `crash` from the existing exception sink); `crates/admin-api/src/routes/telemetry/session_budget.rs`: add `client.session.` to `PRIORITY_PREFIXES`.
- **Test:** unit: the stop path emits one `client.session.end` before the final POST; the budget test keeps it under pressure. **Ship:** `telemetry-gaps/TG-CLI-04-client-session-end` · `tg-cli-04` · `feat(client-telemetry): TG-CLI-04 client.session.end at exit`

### TG-PIPE-12 Launcher `session_meta kind=ended`

W4 · low · packet-coder · Ready · crates `sgw-launcher` · docs: dst
- **Change:** `crates/launcher/src/telemetry/events.rs` (`Ended`), `telemetry/runner.rs:96`: enqueue `kind=ended {exit_code, pid}` before the final flush; the `let _` at :78 becomes a WARN.
- **Test:** serde round-trip, plus a runner test with a fake exit. **Ship:** `telemetry-gaps/TG-PIPE-12-launcher-ended` · `tg-pipe-12` · `feat(launcher): TG-PIPE-12 report the game's exit code`

### TG-PIPE-13 DLL upload-failure accounting

W4 · low · packet-coder · BlockedDependency (TG-PIPE-03) · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `uploader.rs:213` (count `Err(_)`), `governor/mod.rs` (`ExternalDrops` gains `upload_failures`, `last_status`), `queue.rs`: health reports `upload_failures_total`, `last_upload_status`. Correct the false idempotency claim at `uploader.rs:262`.
- **Test:** `retains_batch_after_failed_post` asserts `upload_failures_total=1`. **Ship:** `telemetry-gaps/TG-PIPE-13-upload-failures` · `tg-pipe-13` · `feat(client-telemetry): TG-PIPE-13 count failed uploads on the health row`

### TG-PIPE-14 DLL boot failures as events

W4 · low · packet-coder · BlockedDependency (TG-PIPE-13) · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `boot.rs` (:344, :362, :379, :434), via an extracted `report_bridge_result`: WARN `client.dll.uploader_spawn_failed`, `client.dll.bridge_failed {reason}`, `client.dll.input_unhooked`, `client.hooks.install_lock_unavailable`.
- **Test:** unit on `report_bridge_result`. **Ship:** `telemetry-gaps/TG-PIPE-14-dll-boot-failures` · `tg-pipe-14` · `feat(client-telemetry): TG-PIPE-14 report DLL boot failures`

### TG-MER-12 Client join keys on non-happy Mercury rows

W4 · med · `needs-domain-agent: bigworld-engine-advisor` (recv-to-filter thread order) · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/hooks/inline_hooks/mercury_recv.rs` (:528-565) and `hooks/mercury_recv/report.rs` (`packet()`): `seq` and `wire_fingerprint` (stashed per thread by the `recvfrom` detour, `mercury_recv/wire.rs:55`) on `unpack_fault`, `request_misparse` and non-happy `packet_in`; drop the duplicate `client.mercury.error` WARN for `old_duplicate`; `old_duplicate` for seq ≤ 2 → DEBUG.
- **Test:** `report.rs` unit: a non-happy packet carries `wire_fingerprint`. **Ship:** `telemetry-gaps/TG-MER-12-client-join-keys` · `tg-mer-12` · `feat(client-telemetry): TG-MER-12 seq and fingerprint on Mercury faults`

### TG-CLI-01 `client.viewport.unknown_entity` per entity

Aliases: T2. W4 · high · packet-coder · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/hooks/sinks/bw_message.rs`: recognise the `svidFollow ... is unknown` BigWorld message, parse the entity id, and emit `client.viewport.unknown_entity {entity_id, count, first_ts}` once per entity (first immediately, later counts on the rollup) instead of the free-text row (9.4k rows in the triage). `bw_message_detour.rs` only forwards.
- **Test:** unit on the classifier: three messages for one id give one event then `count=3`; another id is independent. **Ship:** `telemetry-gaps/TG-CLI-01-unknown-entity` · `tg-cli-01` · `feat(client-telemetry): TG-CLI-01 one unknown_entity row per entity`

### TG-CLI-02 Tag BigWorld errors after an unpack fault

Aliases: T3. W4 · med · packet-coder · BlockedDependency (TG-MER-12, TG-CLI-01) · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `inline_hooks/mercury_recv.rs`: on `unpack_fault`, set a thread-local `(seq, deadline)`; `hooks/sinks/bw_message.rs`: messages on that thread before the next bundle get `after_unpack_fault = true` and `seq`.
- **Test:** unit: a message after a fault on the same thread is tagged; one on another thread is not. **Ship:** `telemetry-gaps/TG-CLI-02-after-unpack-fault` · `tg-cli-02` · `feat(client-telemetry): TG-CLI-02 tag errors that follow an unpack fault`

### TG-CLI-03 Cooked pak cache write result and tier

Aliases: T4 (client half). W4 · med · `needs-domain-agent: game-archaeology-specialist` (write site) · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/hooks/inline_hooks/cooked_cache/` and `hooks/seams/file_io.rs`: when a pushed pak is written, emit `client.cooked_cache.write {category_id, version, tier = cache|source_cache, result}`.
- **Test:** unit on the field builder with both tiers. **Ship:** `telemetry-gaps/TG-CLI-03-cooked-cache-write` · `tg-cli-03` · `feat(client-telemetry): TG-CLI-03 log cooked cache writes and their tier`

### TG-CLI-05 Main-thread phase on `client.engine.hitch`

Aliases: T7. W4 · med · packet-coder · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** new `crates/client-telemetry/src/hooks/seams/phase.rs` (an atomic last-phase name and timestamp, set by seams); `seams/level_streaming.rs` sets it; `seams/frame_health.rs` (`hitch_fields`) adds `phase` and `phase_age_ms`.
- **Test:** unit: a phase set 2 s before a hitch appears with `phase_age_ms ≈ 2000`. **Ship:** `telemetry-gaps/TG-CLI-05-hitch-phase` · `tg-cli-05` · `feat(client-telemetry): TG-CLI-05 last main-thread phase on hitch rows`

### TG-CLI-06 Audio stop names its cue

Aliases: T9 (client half). W4 · med · packet-coder · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/hooks/seams/fmod.rs`: remember the name per event pointer in `start_detour` (bounded map) and use it in `stop_detour`; a successful stop with an unresolved name logs DEBUG, not WARN (about 7k WARNs in the triage).
- **Test:** extend `start_and_stop_report_the_event_name_and_pass_results_through`: an unreadable name at stop reuses the start name. **Ship:** `telemetry-gaps/TG-CLI-06-audio-stop-name` · `tg-cli-06` · `fix(client-telemetry): TG-CLI-06 audio stop rows name their cue`

### TG-CLI-07 IAT slot mismatch names its owner

Aliases: T11. W4 · med · packet-coder · Ready · crates `cimmeria-client-telemetry` · docs: client
- **Change:** `crates/client-telemetry/src/hooks/iat_hooks/mod.rs:215` and `hooks/sinks/install.rs:141`: add `owner_module` from `GetModuleHandleExW(FROM_ADDRESS)` and `owner_rva`. The pipeline review found the slot at a fixed RVA in 121 of 139 attaches, and `socket_recv` fired in 0 lab sessions; this tells whether receive telemetry is blind.
- **Test:** unit on the field builder with a fake resolver. **Ship:** `telemetry-gaps/TG-CLI-07-slot-owner` · `tg-cli-07` · `feat(client-telemetry): TG-CLI-07 name the module that owns a mismatched IAT slot`
