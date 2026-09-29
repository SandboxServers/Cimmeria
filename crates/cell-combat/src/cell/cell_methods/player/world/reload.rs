//! Weapon-reload state machine: the two-phase (draw-then-reload) handler,
//! the `reloadOnActivate` option trigger, and the reload ability id.

use crate::cell::client_methods::{being, spawnable_entity};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::item_sequence::fire_item_sequence;

pub const ABILITY_RELOAD_WEAPON: i32 = 596;

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
pub const UNHOLSTER_DRAW_DURATION: std::time::Duration = std::time::Duration::from_millis(1000);

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
pub async fn maybe_trigger_reload_on_activate(
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

/// `requestReload`. Reads `ammo.finite_special` once here and passes it
/// down (ammo campaign AM-02): with the flag off, every reload refills for
/// free exactly as before the campaign.
pub async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let finite = cimmeria_entity::ammo_feature::finite_special();
    handle_reload_with(entity_id, finite, tx, space_mgr).await;
}

/// [`handle_reload`] with the `ammo.finite_special` value passed in, so
/// tests can drive both modes without the process-wide switch.
pub async fn handle_reload_with(
    entity_id: u32,
    finite_special: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
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

    // Special ammo under `ammo.finite_special` (AM-02, D-AM05): the rounds
    // come from the bags. The base draws them and loads the weapon before
    // the warmup starts; its answer (`reload_reserve::handle_reload_drawn`)
    // starts the warmup, or refuses with feedback and leaves the clip as
    // it was. Default ammo and infinite-ammo players fall through.
    match super::reload_reserve::reload_gate(space_mgr, entity_id, finite_special) {
        super::reload_reserve::ReloadGate::Free => {}
        super::reload_reserve::ReloadGate::Blocked => {
            tracing::debug!(
                entity_id,
                "requestReload: a reserve request is in flight, ignoring"
            );
            return;
        }
        super::reload_reserve::ReloadGate::Draw => {
            super::reload_reserve::request_draw(entity_id, tx, space_mgr).await;
            return;
        }
    }

    start_reload_warmup(entity_id, tx, space_mgr).await;
}

/// Phase B proper: start the reload cooldown and the warmup deadline pinned
/// to the active slot, and play the `Item_Reload` sequence. The refill
/// itself happens in the reload-completion tick. Returns the deadline, or
/// `None` when the entity is gone.
pub async fn start_reload_warmup(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<std::time::Instant> {
    let reload_def = space_mgr.ability_defs.get(&ABILITY_RELOAD_WEAPON).cloned();
    let warmup = reload_def.as_ref().map_or(2.0f32, |d| d.warmup);
    let cooldown = reload_def.as_ref().map_or(1.0f32, |d| d.cooldown);
    let entity = space_mgr.get_entity_mut(entity_id)?;

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
    let deadline = std::time::Instant::now() + warmup_duration;
    entity.reload_complete_at = Some(deadline);
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
        0,
        total_time,
        // Absolute, on the client's game clock (see `game_clock`).
        crate::mercury::game_clock::game_time_secs() + total_time,
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
    Some(deadline)
}
