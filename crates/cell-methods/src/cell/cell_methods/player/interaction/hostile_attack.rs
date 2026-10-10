//! Right-click on a live hostile NPC: target it and attack with the active
//! weapon (split out of `interact` along the combat-reroute seam).
//!
//! The ability is the active weapon's RANGED binding (`items_event_sets`
//! event 7). Unarmed fires `594 Strike`. A weapon with no RANGED binding
//! fires nothing and says so: since CS-07 the old `592 Pistol Shot`
//! fallback would be refused by the weapon requirement for anything but a
//! pistol, with a misleading "different weapon" line (review finding 2).

use std::time::{Duration, Instant};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

/// Unarmed right-click: `594 Strike`.
const RIGHT_CLICK_UNARMED: i32 = 594;

/// What a right-click with a weapon that has no RANGED binding shows.
pub(super) const NO_RANGED_ATTACK_TEXT: &str = "This weapon has no ranged attack.";

/// At most one `weapon_unbound` row per player per this window: the
/// right-click is client-controlled and costs nothing to repeat.
const NO_RANGED_ATTACK_LOG_INTERVAL: Duration = Duration::from_secs(10);

/// What a right-click on a hostile does with the active weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RightClick {
    /// Fire this ability.
    Fire(i32),
    /// The active weapon has no RANGED binding: fire nothing.
    NoRangedAttack { item_id: i32 },
}

fn resolve(space_mgr: &SpaceManager, entity_id: u32) -> RightClick {
    let active_item_id = space_mgr.get_entity(entity_id).and_then(|e| {
        e.bandolier_items
            .get(&e.active_bandolier_slot)
            .map(|b| b.item_id)
    });
    match active_item_id {
        None => RightClick::Fire(RIGHT_CLICK_UNARMED),
        Some(item_id) => crate::cell::abilities::ability_for_item(
            space_mgr,
            item_id,
            crate::cell::spawner::EVENT_ITEM_RANGED,
        )
        .map_or(RightClick::NoRangedAttack { item_id }, RightClick::Fire),
    }
}

/// Target `target_entity_id` (a live hostile NPC) for `entity_id` and attack
/// it with the active weapon.
pub(super) async fn right_click_attack(
    entity_id: u32,
    target_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        target_entity_id,
        target_entity_name = space_mgr.entity_label(target_entity_id as u32),
        "interact: targeting hostile NPC for combat"
    );
    // The onTargetUpdate below tells the client this NPC is its
    // target; record the same thing server-side. The auto-cycle loop
    // reads `current_target_id`, and the client sends no `setTargetID`
    // for a right-click, so a loop armed after a right-click saw no
    // target and cleared itself (2026-09-29 colo capture).
    if let Some(actor) = space_mgr.get_entity_mut(entity_id) {
        actor.current_target_id = Some(target_entity_id);
    }
    let mut reply = Vec::with_capacity(4);
    reply.extend_from_slice(&target_entity_id.to_le_bytes());
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 16,
            args: reply,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            target_entity_id,
            target_entity_name = space_mgr.entity_label(target_entity_id as u32),
            "interact: cell->base channel closed sending hostile-NPC combat method: {e}"
        );
        return;
    }

    let ability_id = match resolve(space_mgr, entity_id) {
        RightClick::Fire(ability_id) => ability_id,
        RightClick::NoRangedAttack { item_id } => {
            refuse_no_ranged_attack(entity_id, target_entity_id, item_id, tx, space_mgr).await;
            return;
        }
    };

    // Single canonical kill-credit path — see
    // `handle_use_ability_with_kill_credit` for the
    // alive→dead detection + `fire_entity_death` wrap. Every player-attack
    // path that reaches `handle_use_ability` for a single target routes
    // through this helper so quest KillCount objectives advance uniformly,
    // regardless of which entry point fired the shot (manual right-click,
    // interact, auto-cycle loop, queued attack-while-holstered).
    crate::cell::abilities::handle_use_ability_with_kill_credit(
        entity_id,
        ability_id,
        target_entity_id,
        &crate::cell::content::EngineEvents(engine),
        tx,
        space_mgr,
    )
    .await;
}

/// A right-click with a weapon that has no RANGED binding (blades, grenade
/// launchers, flamethrowers, the bare `CATEGORY_Weapons` items): no cast, no
/// cooldown, no ammo; one feedback line on every click, and a throttled INFO
/// row naming the weapon. Not a WARN: most of these weapons have no ranged
/// attack by design (the CS-07 audit lists them).
async fn refuse_no_ranged_attack(
    entity_id: u32,
    target_entity_id: i32,
    item_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if let Some(suppressed) = space_mgr.ability_refusal_log.admit(
        entity_id,
        "no_ranged_attack",
        Instant::now(),
        NO_RANGED_ATTACK_LOG_INTERVAL,
    ) {
        let who = space_mgr.player_identity(entity_id);
        tracing::info!(
            target: "abilities",
            event = "weapon_unbound",
            decision_outcome = "refused",
            reason = "no_ranged_attack",
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            target_entity_id,
            target_entity_name = space_mgr.entity_label(target_entity_id as u32),
            item_type_id = item_id,
            item_name = cimmeria_names::book().item(item_id),
            suppressed,
            "interact: the active weapon has no items_event_sets RANGED binding \
             (event 7); right-click fires nothing and tells the player"
        );
    }
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_RANGED_ATTACK_TEXT);
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
            args,
        })
        .await
        .is_err()
    {
        let who = space_mgr.player_identity(entity_id);
        tracing::warn!(
            target: "abilities",
            event = "right_click_feedback_send_failed",
            reason = "base_channel_closed",
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            entity_id,
            entity_name = who.player_name,
            "interact: the no-ranged-attack feedback line could not be queued"
        );
    }
}
