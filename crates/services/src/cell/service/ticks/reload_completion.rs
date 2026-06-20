//! Reload-completion promotion tick: refills the active bandolier slot's
//! magazine once a reload warmup deadline has elapsed, emits the stat
//! update, and queues persistence.

use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

/// Promote any reload whose warmup deadline has elapsed: refill the active
/// bandolier slot's magazine, clear `reload_complete_at`, send `onStatUpdate`
/// for the AmmoSlot{N} stat to the player, and queue a `BandolierAmmoUpdate`
/// to base for persistence.
///
/// Stage C: this is the sole refill path. The fire-path eager-promotion has
/// been removed; `handle_use_ability` reads ammo through `entity.active_ammo()`
/// and the bandolier UI updates on every fire via the AmmoSlot{N} stat.
#[tracing::instrument(
    name = "combat.reload_completion_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn reload_completion_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    // Snapshot ready-to-promote player IDs first to avoid holding a borrow on
    // `space_mgr` across the `send_entity_method` await below.
    let ready: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .and_then(|e| e.reload_complete_at)
                .is_some_and(|t| now >= t)
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    for entity_id in ready {
        // Phase 1: mutate entity state, capture stat-update payload + the
        // BandolierAmmoUpdate fields we need to send afterwards. Drop the
        // mutable borrow before any `.await`.
        let (stat_payload, persist) = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => continue,
            };

            // Refill the slot that *started* the reload, not whatever slot is
            // currently active. Without pinning, a mid-reload weapon swap
            // would mis-attribute the refill to the new weapon.
            let slot_id = match entity.reload_slot_id {
                Some(s) => s,
                None => {
                    // Defensive: shouldn't happen — reload_complete_at is only
                    // set together with reload_slot_id. Clear and move on.
                    entity.reload_complete_at = None;
                    tracing::warn!(
                        entity_id,
                        "reload tick: deadline set without slot_id, clearing"
                    );
                    continue;
                }
            };

            // Look up the clip size for the pinned slot. If the slot is
            // empty (item removed mid-reload), clear the deadline and skip
            // the wire send rather than refilling nothing.
            let clip_size = entity.bandolier_items.get(&slot_id).map(|i| i.clip_size);
            let new_ammo = match clip_size {
                Some(cs) => entity.set_slot_ammo(slot_id, cs),
                None => None,
            };
            entity.reload_complete_at = None;
            entity.reload_slot_id = None;

            if new_ammo.is_none() {
                tracing::debug!(
                    entity_id,
                    slot_id,
                    "reload tick: pinned slot empty, no refill"
                );
                continue;
            }

            // The slot was marked dirty by `set_slot_ammo`; persistence drains
            // it via the BandolierAmmoUpdate below.
            entity.bandolier_ammo_dirty.remove(&slot_id);

            let payload = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();

            // `instance_id` (sgw_inventory.item_id PK) is the persist TOCTOU
            // guard; the design id is not used here. A missing slot yields
            // instance_id 0, which the base-side bound check (`<= 0`) drops.
            let (instance_id, cur_ammo, cur_ammo_type) =
                entity.bandolier_items.get(&slot_id).map_or((0, 0, 0), |i| {
                    (i.instance_id, i.current_ammo, i.cur_ammo_type)
                });
            let persist = entity
                .player_id
                .map(|pid| (pid, slot_id, instance_id, cur_ammo, cur_ammo_type));

            (payload, persist)
        };

        // Phase 2: send onStatUpdate. Skip when no stats actually changed.
        // `serialize_dirty` always emits a 4-byte u32 count prefix, so an
        // `is_empty()` check would never fire and we'd send a zero-entry
        // payload on no-op refills. Gate on the encoded count instead.
        if stat_payload.len() > 4 {
            crate::cell::abilities::send_entity_method(
                entity_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                stat_payload,
                tx,
                space_mgr,
            )
            .await;
        }

        // Phase 3: persistence. CellToBaseMsg::BandolierAmmoUpdate is consumed
        // by base's existing handler that writes `sgw_inventory.ammo`.
        if let Some((player_id, slot_id, expected_instance_id, current_ammo, cur_ammo_type)) =
            persist
        {
            let _ = tx
                .send(CellToBaseMsg::BandolierAmmoUpdate {
                    player_id,
                    slot_id,
                    expected_instance_id,
                    current_ammo,
                    cur_ammo_type,
                })
                .await;
        }

        // Phase 4: fire the `Ability_End` sequence to signal "weapon ready
        // again" to the client. Pairs with the `Ability_Begin` sent at
        // reload-start in `handle_reload`.
        //
        // Currently inert against the production seed: ability 596 has
        // `event_set_id = NULL`, so this branch short-circuits. The legacy
        // `AbilityManager.py:671-673` reference is correct for abilities
        // that follow the begin/end pattern, but reload specifically sources
        // its animation from the player's archetype-keyed item event set
        // (`Item_Reload`, event id 4002) and is a single-sequence shape —
        // there is no separate end. The archetype lookup will replace this
        // branch outright once it lands.
        const ABILITY_RELOAD_WEAPON: i32 = 596;
        let event_set_id = space_mgr
            .ability_defs
            .get(&ABILITY_RELOAD_WEAPON)
            .and_then(|d| d.event_set_id);
        if let Some(esid) = event_set_id {
            use crate::cell::spawner::EVENT_ABILITY_END;
            if let Some(&seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ABILITY_END)) {
                let mut seq_args = Vec::with_capacity(28);
                seq_args.extend_from_slice(&seq_id.to_le_bytes());
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
                seq_args.push(1);
                seq_args.extend_from_slice(&0.0f32.to_le_bytes());
                seq_args.extend_from_slice(&0u32.to_le_bytes());
                seq_args.push(0);
                seq_args.extend_from_slice(&0i32.to_le_bytes());
                crate::cell::abilities::send_entity_method(
                    entity_id,
                    crate::cell::client_methods::spawnable_entity::ON_SEQUENCE,
                    seq_args,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn reload_completion_tick_skips_entity_with_empty_slot() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Slot 0 was removed mid-reload: reload_slot_id points to missing item.
            e.reload_slot_id = Some(0);
            e.reload_complete_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        reload_completion_tick(&tx, &mut mgr).await;

        // No messages should be sent because the slot is empty.
        assert!(
            rx.try_recv().is_err(),
            "empty slot must produce zero wire messages"
        );
        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.reload_complete_at.is_none(),
            "deadline must be cleared even when slot is empty"
        );
    }

    /// Defensive path: `reload_complete_at` set but `reload_slot_id` is None
    /// (should be impossible, but guard against state drift). The tick must
    /// clear `reload_complete_at` and produce zero wire messages.
    #[tokio::test]
    async fn reload_completion_tick_clears_deadline_when_slot_id_missing() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Deadline set but slot_id is None — the defensive guard fires.
            e.reload_complete_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            e.reload_slot_id = None;
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        reload_completion_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "missing slot_id must produce zero wire messages"
        );
        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.reload_complete_at.is_none(),
            "deadline must be cleared by the defensive guard"
        );
    }
}
