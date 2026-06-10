use crate::cell::client_methods::{being, spawnable_entity};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        SET_AUTO_CYCLE => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::info!(entity_id, enabled, "setAutoCycle");
                if enabled {
                    // Light BSF_AUTO_CYCLING immediately so the button
                    // highlights on the very first press; without this
                    // the button looks broken until the player happens
                    // to right-click an enemy. Phase 2: if the player
                    // already has a target selected AND has fired any
                    // ability in this session, ALSO fire that ability
                    // now — the press feels like an action ("start
                    // firing"), not a mode flip. `last_fired_ability_id`
                    // is the simplest server-side proxy for "what
                    // would the player fire?" since the wire payload
                    // carries no ability id.
                    let (new_state, immediate_fire) = {
                        let entity = match space_mgr.get_entity_mut(entity_id) {
                            Some(e) => e,
                            None => return true,
                        };
                        entity.abilities.auto_cycle = true;
                        // Raw bit op — see auto_cycle module doc for why
                        // BSF_AUTO_CYCLING bypasses the ref-counted helpers.
                        let old = entity.state_field;
                        entity.state_field |= crate::cell::combat::BSF_AUTO_CYCLING;
                        let new_state = (entity.state_field != old).then_some(entity.state_field);
                        let immediate_fire = entity
                            .abilities
                            .last_fired_ability_id
                            .zip(entity.current_target_id);
                        // Persist the loop's committed ability BEFORE
                        // calling handle_use_ability. If that call
                        // rejects (out of range / cooldown / no ammo),
                        // its commit-time arm path never runs and the
                        // tick driver would have no ability id to
                        // re-fire with — BSF-armed but functionally
                        // dead. Stashing here lets the next
                        // cooldown-clear tick pick up the loop
                        // regardless of immediate-fire outcome.
                        if let Some((ability_id, _)) = immediate_fire {
                            entity.abilities.auto_cycle_ability_id = Some(ability_id);
                        }
                        (new_state, immediate_fire)
                    };
                    if let Some(new_state) = new_state {
                        super::super::super::abilities::send_entity_method(
                            entity_id,
                            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                            new_state.to_le_bytes().to_vec(),
                            tx,
                            space_mgr,
                        )
                        .await;
                        persist_state_field_bits(entity_id, new_state, tx, space_mgr).await;
                    }
                    // Gated on bit transition. CEGUI fires the Lua
                    // binding 3-4× per physical click (~150µs apart);
                    // without the gate every duplicate would re-attempt
                    // the fire and rely on the cooldown gate inside
                    // handle_use_ability to reject — wasted work + log
                    // noise.
                    //
                    // Additionally gated on the stashed ability being
                    // off cooldown. The bit-transition gate alone still
                    // produces one cooldown-rejection DEBUG per
                    // "arm while mid-cooldown" toggle (player manually
                    // right-clicks once, then arms auto-cycle while
                    // the manual shot is still cooling). The stash is
                    // already persisted above, so the next
                    // auto_cycle_tick after cooldown clear picks the
                    // re-fire up — calling handle_use_ability here
                    // when it would just reject is wasted work.
                    let stash_ready = match immediate_fire {
                        Some((ability_id, _)) => space_mgr
                            .get_entity(entity_id)
                            .is_some_and(|e| !e.abilities.is_on_cooldown(ability_id)),
                        None => false,
                    };
                    if let (Some(_), true, Some((ability_id, target_id))) =
                        (new_state, stash_ready, immediate_fire)
                    {
                        tracing::info!(
                            entity_id,
                            ability_id,
                            target_id,
                            "setAutoCycle: immediate fire on enable (last_fired + current_target ready)"
                        );
                        // The immediate-fire on auto-cycle toggle ON
                        // is a player-driven kill path — route through
                        // the kill-credit wrapper so a tap of the
                        // auto-fire button that immediately kills a
                        // quest target credits the mission, matching
                        // the manual-right-click path.
                        let _ =
                            super::super::super::abilities::handle_use_ability_with_kill_credit(
                                entity_id, ability_id, target_id, engine, tx, space_mgr,
                            )
                            .await;
                    }
                } else {
                    // Explicit disable: drop the stash AND clear the bit.
                    // clear_auto_cycle returns Some only on bit
                    // transition — re-broadcasting for an already-off
                    // player would be wire noise.
                    if let Some(new_state) =
                        crate::cell::combat::clear_auto_cycle(space_mgr, entity_id)
                    {
                        super::super::super::abilities::send_entity_method(
                            entity_id,
                            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                            new_state.to_le_bytes().to_vec(),
                            tx,
                            space_mgr,
                        )
                        .await;
                        persist_state_field_bits(entity_id, new_state, tx, space_mgr).await;
                    }
                }
            }
            true
        }

        LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                crate::cell::interactions::handle_loot_item(entity_id, index, tx, space_mgr).await;
            }
            true
        }

        TRIGGER_REGION => {
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Region IDs are wire-encoded as i32 but stored as u32 internally.
                // Reject negative values up-front rather than sign-extending them
                // into a high u32 that no real region will match.
                let (region_tag, db_set_id) = match u32::try_from(region_id) {
                    Ok(rid) => match space_mgr.get_region(rid) {
                        Some(r) => (Some(r.tag.clone()), Some(r.db_set_id)),
                        None => (None, None),
                    },
                    Err(_) => {
                        tracing::warn!(
                            entity_id,
                            region_id,
                            "triggerClientHintedGenericRegion: negative region_id, ignoring"
                        );
                        (None, None)
                    }
                };

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr
                        .get_entity(entity_id)
                        .and_then(|e| e.player_id)
                        .unwrap_or(0);

                    if b_entering {
                        crate::cell::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    } else {
                        crate::cell::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        )
                        .await;
                    }

                    // Forward to the ring transporter FSM if this region is a
                    // ring pad (point_set_id matches a loaded ring region).
                    if let Some(set_id) = db_set_id {
                        crate::cell::ring_transport::handle_region_trigger(
                            set_id, b_entering, entity_id, tx, space_mgr, engine,
                        )
                        .await;
                    }
                } else {
                    tracing::warn!(
                        entity_id,
                        region_id,
                        "Unknown region ID in triggerClientHintedGenericRegion"
                    );
                }
            }
            true
        }

        REQUEST_RELOAD => {
            if !args.is_empty() {
                let _reload_type = args[0];
                tracing::debug!(entity_id, "requestReload");
                handle_reload(entity_id, tx, space_mgr).await;
            }
            true
        }

        CHOSEN_REWARDS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
            true
        }

        SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(
                    entity_id,
                    region_id,
                    destination_id,
                    "setRingTransporterDestination"
                );
                crate::cell::ring_transport::handle_select_destination(
                    region_id,
                    destination_id,
                    entity_id,
                    tx,
                    space_mgr,
                    engine,
                )
                .await;
            }
            true
        }

        WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
            true
        }

        UPDATE_SYSTEM_OPTIONS => {
            handle_update_system_options(entity_id, args, tx, space_mgr).await;
            true
        }

        _ => false,
    }
}

/// Fire-and-forget persist of the user-preference `state_field` bits
/// after an explicit `setAutoCycle` toggle flipped `BSF_AutoCycling`.
///
/// Masks to [`crate::cell::combat::PERSISTED_STATE_FIELD_MASK`] so
/// transient combat bits riding the same broadcast value (BSF_Dead,
/// BSF_InCombat, BSF_MovementLock) never reach the DB — a relog must
/// always be a clean combat slate (#412).
///
/// Deliberately called from the explicit toggle site only, NOT from the
/// in-combat auto-clear paths (target death, manual fire of a different
/// ability, AF_DEACTIVATE_AUTO_CYCLE): those are session mechanics, and
/// mirroring them to the DB would make the post-relog state depend on
/// whether the player's last target happened to die — the persisted
/// value tracks the player's deliberate button choice.
///
/// Silent no-op for entities without a `player_id` (NPCs / test
/// fixtures shouldn't persist).
async fn persist_state_field_bits(
    entity_id: u32,
    state_field: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        return;
    };
    if let Err(e) = tx
        .send(CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field: state_field & crate::cell::combat::PERSISTED_STATE_FIELD_MASK,
        })
        .await
    {
        // Channel closed — the toggle still applies in-memory for this
        // session but won't survive the relog. Same level rationale as
        // the SystemOptionsUpdate persist failure path.
        tracing::warn!(
            entity_id,
            player_id,
            error = %e,
            "StateFieldUpdate send to base failed -- auto-cycle preference not persisted"
        );
    }
}

/// Parse the `updateSystemOptions(ARRAY <of> NameValuePair)` payload and
/// apply each recognised option to the player's [`SystemOptions`] block.
///
/// Wire layout (BigWorld FIXED_DICT array of NameValuePair):
///
/// ```text
/// [count: u32 LE]
///   for each NameValuePair:
///     [name_char_count: u32 LE][name: u16 LE × char_count]    -- WSTRING
///     [val_char_count : u32 LE][value: u16 LE × char_count]   -- WSTRING
/// ```
///
/// Unrecognised option names are logged at `debug!` so a typo or a
/// not-yet-implemented option (we only honour `autoReload` and
/// `reloadOnActivate` today) doesn't get silently swallowed. The full
/// option surface is in `SGWGame/Content/XML/SystemOptions.xml`; extending
/// is mechanical — add a field on `SystemOptions` and a match arm in
/// [`cimmeria_entity::cell_entity::SystemOptions::apply`].
async fn handle_update_system_options(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let pairs = match parse_name_value_pairs(args) {
        Ok(pairs) => pairs,
        Err(e) => {
            // Log loud — a parse failure means the client sent a payload
            // we don't understand, and silently dropping it would leave
            // the user with toggles that look saved but never apply.
            tracing::warn!(
                entity_id,
                error = %e,
                body_len = args.len(),
                "updateSystemOptions: parse failed; options unchanged"
            );
            return;
        }
    };

    // Snapshot post-apply state inside the mutable borrow so we can
    // release the borrow before the `.send()` await — `tx.send` is
    // async and `space_mgr.get_entity_mut` returns a non-Send guard.
    let persist = {
        let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
            tracing::warn!(
                entity_id,
                "updateSystemOptions: entity not found; options dropped"
            );
            return;
        };

        let mut applied = 0usize;
        let mut unknown: Vec<String> = Vec::new();
        for (name, value) in &pairs {
            if entity.system_options.apply(name, value) {
                applied += 1;
            } else {
                unknown.push(name.clone());
            }
        }

        tracing::info!(
            entity_id,
            count = pairs.len(),
            applied,
            auto_reload = entity.system_options.auto_reload,
            reload_on_activate = entity.system_options.reload_on_activate,
            "updateSystemOptions: applied"
        );
        if !unknown.is_empty() {
            tracing::debug!(
                entity_id,
                ?unknown,
                "updateSystemOptions: unknown option names (not yet supported)"
            );
        }

        // Only fire the persist message if (a) we have a player_id (NPCs
        // and unattached entities shouldn't persist) AND (b) at least one
        // known option actually applied (skip the wire round-trip on a
        // pure no-op payload of all-unknown options).
        if applied > 0 {
            entity.player_id.map(|pid| {
                (
                    pid,
                    entity.system_options.auto_reload,
                    entity.system_options.reload_on_activate,
                )
            })
        } else {
            None
        }
    };

    if let Some((player_id, auto_reload, reload_on_activate)) = persist {
        // Fire-and-forget. Mirrors `ActiveSlotUpdate`'s pattern — the
        // base-side handler logs `warn!` on DB write failure; we don't
        // block the cell tick on the round-trip.
        let _ = tx
            .send(CellToBaseMsg::SystemOptionsUpdate {
                player_id,
                auto_reload,
                reload_on_activate,
            })
            .await;
    }
}

/// Decode an `ARRAY <of> NameValuePair`. Visible to tests so the
/// wire-format round-trip can be pinned.
pub(super) fn parse_name_value_pairs(buf: &[u8]) -> Result<Vec<(String, String)>, String> {
    use crate::mercury::read_wstring;

    if buf.len() < 4 {
        return Err(format!(
            "ARRAY<NameValuePair>: need 4 bytes for count, have {}",
            buf.len()
        ));
    }
    let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    // Hard cap against a corrupt or hostile count field that would
    // otherwise drive `Vec::with_capacity` into a multi-gigabyte
    // allocation. SystemOptions.xml has ~140 options total (only 2 of
    // them server-synced), so a real client never sends more than a
    // few dozen at once; 256 is well past that with no expectation of
    // tightness. The per-pair `read_wstring` calls below also bounds-
    // check against the buffer, so we don't need a separate
    // remaining-buffer check here — a corrupt count that gets past
    // this cap will still surface as a typed read error.
    if count > 256 {
        return Err(format!(
            "ARRAY<NameValuePair>: implausible count {count} (cap 256)"
        ));
    }
    let mut pairs = Vec::with_capacity(count);
    let mut offset = 4;
    for i in 0..count {
        let (name, n) = read_wstring(buf, offset).map_err(|e| {
            format!(
                "ARRAY<NameValuePair>[{i}].name: {e} (offset {offset}, remaining {})",
                buf.len().saturating_sub(offset)
            )
        })?;
        offset += n;
        let (value, n) = read_wstring(buf, offset).map_err(|e| {
            format!(
                "ARRAY<NameValuePair>[{i}].value: {e} (offset {offset}, remaining {})",
                buf.len().saturating_sub(offset)
            )
        })?;
        offset += n;
        pairs.push((name, value));
    }
    Ok(pairs)
}

const ABILITY_RELOAD_WEAPON: i32 = 596;

/// How long the draw animation needs to play before the reload sequence
/// can fire. Hand needs to reach the hold position and grip the weapon
/// mesh; firing `Item_Reload` while the hand is mid-air mid-draw plays
/// the reload animation on a model that isn't in the reload-ready pose
/// — the client either ignores the request or visually skips the
/// reload anim (the symptom the user reported in playtest: "weapon
/// teleports into my hand and I still need to hit reload again").
///
/// Empirically tuned to 1 second; matches the rough length of the
/// `KIS-handling` kismet script's draw branch. Bump if the reload still
/// chains into the draw mid-animation; lower if the gap between draw
/// and reload becomes visually obvious.
pub(crate) const UNHOLSTER_DRAW_DURATION: std::time::Duration =
    std::time::Duration::from_millis(1000);

/// Fire a `Item_*` kismet sequence (`Item_Equip` 4000 / `Item_Unequip`
/// 4001 / `Item_Reload` 4002 / `Item_Use` 4003) keyed off the player's
/// archetype-keyed "Item handling" event set. Mirrors
/// `python/cell/SGWBeing.py:getItemSequence(eventId)` + `playSequence`.
///
/// No-op (with a debug log) when the archetype, the event set, or the
/// per-event sequence is missing — matches the python's `if eventSet
/// else None` fallthrough so callers don't crash on edge entities.
pub(crate) async fn fire_item_sequence(
    entity_id: u32,
    event_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let archetype_id = space_mgr.get_entity(entity_id).and_then(|e| e.archetype_id);
    let event_set = archetype_id.and_then(crate::cell::spawner::archetype_item_event_set);
    let seq_id = event_set.and_then(|esid| space_mgr.sequence_map.get(&(esid, event_id)).copied());
    tracing::info!(
        entity_id,
        event_id,
        archetype_id = ?archetype_id,
        event_set_id = ?event_set,
        seq_id = ?seq_id,
        "fire_item_sequence: archetype-keyed sequence lookup"
    );
    let Some(seq_id) = seq_id else {
        return;
    };
    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire
    // path so animations are emitted consistently with weapon-fire and
    // reload animations).
    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&seq_id.to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
    seq_args.push(1);
    seq_args.extend_from_slice(&0.0f32.to_le_bytes());
    seq_args.extend_from_slice(&0u32.to_le_bytes());
    seq_args.push(0);
    seq_args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: spawnable_entity::ON_SEQUENCE,
            args: seq_args,
        })
        .await;
}

/// Trigger `handle_reload` for the player IF their `reloadOnActivate`
/// client option is set, the active slot holds a weapon, and the slot's
/// clip is below max. No-op otherwise.
///
/// Called from both bandolier-activate paths — the manual F1-F4 swap
/// (`handle_request_active_slot_change`) and the in-game equip from
/// inventory (`handle_update_bandolier_item`) — so the user-facing
/// semantics are uniform: switching to a weapon you've been carrying
/// around with a partial clip auto-tops it up.
///
/// We intentionally don't fire this from the unholster path: Phase A of
/// `handle_reload` IS itself the draw animation, and triggering an
/// auto-reload there would loop. The option's wording in
/// `SystemOptions.xml` ("Reloads weapon when activated") matches
/// slot-activate / inventory-equip semantics, not weapon-draw.
pub(crate) async fn maybe_trigger_reload_on_activate(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let should_reload = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.is_player
                && e.system_options.reload_on_activate
                && e.active_clip_size() > 0
                && e.active_ammo() < e.active_clip_size()
                // Same gates as `maybe_trigger_auto_reload`: don't
                // queue on top of an in-flight reload.
                && e.reload_complete_at.is_none()
                && e.pending_reload_at.is_none()
        }
        None => false,
    };
    if !should_reload {
        return;
    }
    tracing::info!(
        entity_id,
        "bandolier-activate: reload-on-activate triggered"
    );
    handle_reload(entity_id, tx, space_mgr).await;
}

pub(crate) async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let reload_def = space_mgr.ability_defs.get(&ABILITY_RELOAD_WEAPON).cloned();
    let warmup = reload_def.as_ref().map_or(2.0f32, |d| d.warmup);
    let cooldown = reload_def.as_ref().map_or(1.0f32, |d| d.cooldown);

    // Phase A — reload-while-holstered: defer the actual reload until
    // the draw animation has had time to play. Fires `Item_Equip`
    // (event 4000), the bandolier-equip animation, as a stand-in for an
    // explicit draw sequence (the 2009 client never shipped one — the
    // archaeology agent confirmed `Event_NetOut_ChangeWeaponState` is
    // dead scaffolding). Re-running `handle_reload` after
    // `UNHOLSTER_DRAW_DURATION` (via `pending_reload_tick`) lands us in
    // Phase B with the weapon already drawn.
    //
    // Gate: weapon currently holstered + threatened_mobs empty (OOC) +
    // no pending phase already in flight. In-combat reload skips this
    // entirely (weapon's already drawn).
    let needs_phase_a = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.weapon_holstered && e.threatened_mobs.is_empty() && e.pending_reload_at.is_none()
        }
        None => false,
    };
    if needs_phase_a {
        // Don't accidentally start a phase A for a player whose mag is
        // already full — the early-return below would catch it after
        // Phase B too, but skipping the wasted draw animation is the
        // right move.
        let ammo_already_full = match space_mgr.get_entity(entity_id) {
            Some(e) => e.active_ammo() >= e.active_clip_size() && e.reload_complete_at.is_none(),
            None => true,
        };
        if !ammo_already_full {
            if let Some(e) = space_mgr.get_entity_mut(entity_id) {
                e.combat_exit_at = Some(std::time::Instant::now());
                e.set_weapon_holstered(false);
                e.pending_reload_at = Some(std::time::Instant::now() + UNHOLSTER_DRAW_DURATION);
                // Cancel any in-flight holster Phase 2 — reload draws
                // the weapon BACK out, so a stale Phase 2 would yank
                // the mesh away mid-reload.
                e.holster_animation_complete_at = None;
            }
            tracing::info!(
                entity_id,
                draw_duration_ms = UNHOLSTER_DRAW_DURATION.as_millis() as u64,
                "reload-while-holstered: phase A — drawing weapon, reload deferred"
            );
            crate::cell::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
            fire_item_sequence(
                entity_id,
                crate::cell::spawner::EVENT_ITEM_EQUIP,
                tx,
                space_mgr,
            )
            .await;
            return;
        }
    }

    // Reject second-press during the Phase A draw window. If
    // `pending_reload_at` is set and the timestamp hasn't elapsed yet,
    // the only legitimate entry path is the tick — but the tick fires
    // strictly after the timestamp, so a `now < pending_reload_at`
    // observation here means the player pressed R again mid-draw.
    // Without this gate, the second press falls through to Phase B,
    // clears `pending_reload_at` ahead of schedule, and starts the
    // reload cooldown immediately — defeating the draw window.
    if let Some(t) = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.pending_reload_at)
    {
        if std::time::Instant::now() < t {
            tracing::debug!(
                entity_id,
                "requestReload: ignoring while draw window in progress"
            );
            return;
        }
    }

    // Phase B (or a normal already-drawn reload). When entered from the
    // `pending_reload_tick`, clear the deferred-reload stamp so a
    // racing tick won't re-fire phase B.
    //
    // Also cancel any in-flight OOC holster Phase 2
    // (`holster_animation_complete_at`). Reload semantics imply the
    // weapon stays drawn — without this clear, the Phase 2 deadline
    // would still elapse mid-reload, flip `weapon_holstered = true`,
    // and broadcast `BeingAppearance` with no weapon attached. The
    // user would see the reload animation play then the weapon
    // mesh vanish on the next AoI tick. The Phase A block above
    // performs the same clear at its `set_weapon_holstered(false)`
    // call; mirror that here so Phase B reloads against an
    // already-drawn weapon also cancel the pending holster.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.pending_reload_at = None;
        e.holster_animation_complete_at = None;
    }

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "requestReload: entity not found");
            return;
        }
    };

    if entity.active_ammo() >= entity.active_clip_size() && entity.reload_complete_at.is_none() {
        tracing::debug!(entity_id, "requestReload: already at max ammo");
        return;
    }

    let old = entity.active_ammo();
    let target_ammo = entity.active_clip_size();

    let total_time = warmup + cooldown;
    entity.abilities.start_ability_cooldown(
        ABILITY_RELOAD_WEAPON,
        std::time::Duration::from_secs_f32(total_time),
    );

    // Defer the actual ammo refill until after the warmup. The reload-completion
    // tick promotes pending refills; the fire-path gates on `reload_complete_at`
    // to prevent shooting during the warmup.
    //
    // Pin the reload to the slot that started it. If the player swaps weapons
    // mid-reload, the tick must refill *this* slot — not whatever slot is
    // active when the deadline elapses.
    let warmup_duration = std::time::Duration::from_secs_f32(warmup.max(0.0));
    entity.reload_complete_at = Some(std::time::Instant::now() + warmup_duration);
    entity.reload_slot_id = Some(entity.active_bandolier_slot);

    tracing::info!(
        entity_id,
        old,
        target = target_ammo,
        warmup,
        cooldown,
        "Weapon reload started"
    );

    let timer_args = cimmeria_entity::abilities::serialize_timer_update(
        ABILITY_RELOAD_WEAPON,
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        total_time,
        0.0,
    );
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: being::ON_TIMER_UPDATE,
            args: timer_args,
        })
        .await;

    // BSF_InCombat is intentionally not touched here: a reload in
    // isolation (no aggro) must not flip the in-combat HUD/cursor. The
    // bit is derived from `threatened_mobs` and only flips on via
    // `combat::generate_threat` → `enter_player_combat` when the player
    // actually generates threat on a surviving NPC.

    // Re-stamp `combat_exit_at` so the OOC holster timer fires
    // `OOC_HOLSTER_DELAY` seconds from reload start, never mid-animation.
    // (Phase A already stamped this; Phase B re-stamps so a normal
    // already-drawn reload also resets the OOC countdown.) In-combat
    // reload is untouched — `threatened_mobs.is_empty()` is false, so
    // we skip entirely and `combat_exit_at` stays None until the fight
    // ends naturally.
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        if e.threatened_mobs.is_empty() {
            e.combat_exit_at = Some(std::time::Instant::now());
        }
    }

    // Fire the `Item_Reload` (event 4002) animation — the visible
    // drop-mag / insert-mag / chamber sequence. Mirrors
    // `python/cell/SGWBeing.py:863-874`'s `getItemSequence` +
    // `playSequence`. Archetype lookup + sequence dispatch lives in
    // `fire_item_sequence`.
    fire_item_sequence(
        entity_id,
        crate::cell::spawner::EVENT_ITEM_RELOAD,
        tx,
        space_mgr,
    )
    .await;

    // Ammo-type update after reload. propId 3 = `GENERICPROPERTY_AmmoTypeId`;
    // propId 7 is `AccessLevel` — sending the ammo type under propId 7 plants
    // a stray `setAccessLevel(<ammo_type_value>)` on the client (issue #168).
    // The HUD's ammo-type indicator usually still updates because the
    // bandolier sync path independently emits propId 3, but that's masking,
    // not correctness.
    let ammo_type = space_mgr
        .get_entity(entity_id)
        .map_or(0, |e| e.active_ammo_type());
    let args = crate::cell::cell_methods::inventory::build_entity_property_args(
        crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID,
        ammo_type,
    );
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: spawnable_entity::ON_ENTITY_PROPERTY,
            args,
        })
        .await;
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod system_options_tests;
