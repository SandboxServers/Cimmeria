//! Strip the partner's effects at the end of a duel.
//!
//! The harm gate runs at launch and at fire, but an effect already on a
//! duelist (a bleed, a stun, a snare) keeps pulsing from the effect tick,
//! which never re-checks hostility. Without this, a DoT the partner applied
//! would keep hurting after the duel ended. [`strip_from`] removes every
//! active effect on `target` whose invoker is the partner's engaged entity,
//! with the same cleanup the pulse sweep and the channel cancel use: the
//! script's `on_remove`, the stat flush, and a zero `onTimerUpdate` so the
//! client drops the icon.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_timer_update, TIMER_DURATION_EFFECT};
use cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Remove every active effect on `target` invoked by `invoker`. Returns how
/// many were removed. `target` is a player, so every send goes to its own
/// client (as the pulse sweep's `send_entity_method` does for a player).
pub(super) async fn strip_from(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    target: u32,
    invoker: u32,
) -> usize {
    let removed: Vec<(i32, i32)> = {
        let Some(entity) = mgr.get_entity_mut(target) else {
            return 0;
        };
        let mut removed = Vec::new();
        entity.active_effects.retain(|inst| {
            if inst.invoker_id == invoker {
                removed.push((inst.effect_id, inst.ability_id));
                false
            } else {
                true
            }
        });
        removed
    };
    let id = mgr.player_identity(target);
    for &(effect_id, _ability_id) in &removed {
        if let Some(def) = mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script) = def.script_name.clone() {
                let mut ctx = crate::cell::effects::EffectContext {
                    source_id: invoker,
                    target_id: target,
                    effect: &def,
                    space_mgr: mgr,
                };
                crate::cell::effects::dispatch_on_remove(&script, &mut ctx);
            }
        }
        let dirty = mgr.get_entity_mut(target).map(|e| {
            let d = e.stats.serialize_dirty();
            e.stats.clear_dirty();
            d
        });
        if let Some(dirty) = dirty.filter(|d| !d.is_empty()) {
            send(
                tx,
                id,
                target,
                cimmeria_wire::mercury::method_idx::ON_STAT_UPDATE,
                dirty,
            )
            .await;
        }
        let zero = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            invoker as i32,
            effect_id,
            0.0,
            0.0,
        );
        send(tx, id, target, ON_TIMER_UPDATE, zero).await;
    }
    removed.len()
}

async fn send(
    tx: &mpsc::Sender<CellToBaseMsg>,
    id: cimmeria_entity::cell_entity::PlayerIdentity,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
) {
    if tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        })
        .await
        .is_err()
    {
        tracing::warn!(
            target: "duel",
            event = "duel.send_failed",
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id,
            method_index,
            reason = "cell_to_base_closed",
            "duel effect cleanup could not be queued to the base"
        );
    }
}
