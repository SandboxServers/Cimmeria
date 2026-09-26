//! Out-of-combat health and focus regeneration tick.

use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

/// Out-of-combat health and focus regeneration.
///
/// For each connected, alive player whose `threatened_mobs` set is empty,
/// advance any pool whose `cur < max` toward `max` by its regen-stat
/// value (with a floor of 1 per pool so a freshly-rolled archetype —
/// every player class seeds `healthRegen` and `focusRegen` to 0 in
/// `SGWBeing.statsTemplate` — still recovers between fights). All
/// pool changes for a given player are bundled into a single
/// `onStatUpdate` via `serialize_dirty`.
///
/// **Pool coverage**:
/// - `HEALTH` — primary HP pool, damaged by `HealthDamage` effects.
/// - `FOCUS` — "mental HP", damaged by `FocusDamage` effects (every
///   human archetype rolls in with `focus = 1570`, see
///   `db/resources/Archetypes/Seed/archetypes.sql`).
/// - `ENERGY_POOL` — *not* regenerated here. The archetypes table has no
///   `energy` column and `SGWBeing.statsTemplate` defaults it to
///   `Stat(0, 0, 0)`; for player classes the pool is dead and the
///   `cur < max` gate would skip it anyway. Adding it would just be
///   noise in the wire payload.
///
/// Why `threatened_mobs.is_empty()` instead of `BSF_IN_COMBAT == 0`: the
/// flag has three setters (`use_ability.rs`, reload in
/// `cell_methods/player/world.rs`, `enter_player_combat`) but only one
/// clear path (death-driven `clear_dead_npc_from_all_player_threat`), so
/// the bit gets stuck after one-shot kills, reload in isolation, and
/// no-target self-casts. `threatened_mobs` is the actual source of
/// aggro truth — empty set means no NPC currently has the player on its
/// threat list.
///
/// Cadence is 1 Hz — the caller must drive this on every 10th 100ms AoI
/// tick. The per-call delta is therefore "points per second"; if cadence
/// ever changes, the floors and the regen values need to be scaled
/// together.
pub(in crate::cell::service) async fn regen_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use crate::cell::combat::state::BSF_DEAD;
    use cimmeria_entity::stats::{FOCUS, FOCUS_REGEN, HEALTH, HEALTH_REGEN};

    /// (pool_id, regen_stat_id) pairs the tick advances. Add new pools
    /// here; the inner loop handles `cur < max` skip and floor-of-1 the
    /// same way for every entry.
    const POOLS: &[(i32, i32)] = &[(HEALTH, HEALTH_REGEN), (FOCUS, FOCUS_REGEN)];

    // Snapshot eligible player IDs first — we mutate stats inside the loop
    // and `send_entity_method` awaits, so we cannot hold a borrow on
    // `space_mgr` across iterations.
    let eligible: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.threatened_mobs.is_empty()
                    && e.state_field & BSF_DEAD == 0
                    && POOLS.iter().any(|&(pool_id, _)| {
                        e.stats
                            .get(pool_id)
                            .is_some_and(|s| s.cur < s.max && s.max > 0)
                    })
            })
        })
        .collect();

    for entity_id in eligible {
        let stat_payload = {
            let entity = match space_mgr.get_entity_mut(entity_id) {
                Some(e) => e,
                None => continue,
            };

            for &(pool_id, regen_id) in POOLS {
                let regen = entity.stats.get(regen_id).map_or(0, |s| s.cur).max(1);
                if let Some(pool) = entity.stats.get_mut(pool_id) {
                    if pool.cur < pool.max && pool.max > 0 {
                        pool.change(regen);
                    }
                }
            }

            let payload = entity.stats.serialize_dirty();
            entity.stats.clear_dirty();
            payload
        };

        // `serialize_dirty` always emits a 4-byte u32 count prefix, so
        // `is_empty()` would never fire. Gate on the encoded count to
        // suppress empty payloads when the eligibility filter raced with
        // a concurrent state change and nothing actually got dirtied.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn regen_tick_advances_health_when_ooc_and_damaged() {
        use cimmeria_entity::stats::HEALTH;

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
            // Damage: set health to 50/100
            if let Some(hp) = e.stats.get_mut(HEALTH) {
                hp.update(0, 50, 100);
                hp.dirty = false; // clear the dirty bit from setup
            }
            // HEALTH_REGEN defaults to 0/0/0 — the floor-of-1 logic applies
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        let entity = mgr.get_entity(1).unwrap();
        let hp = entity.stats.get(HEALTH).unwrap();
        assert_eq!(
            hp.cur, 51,
            "OOC regen must advance HP by floor-of-1 (regen stat is 0)"
        );

        // Must have sent an onStatUpdate
        let mut got_stat_update = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                    got_stat_update = true;
                }
            }
        }
        assert!(
            got_stat_update,
            "regen_tick must send onStatUpdate when HP changed"
        );
    }

    #[tokio::test]
    async fn regen_tick_skips_player_in_combat() {
        use cimmeria_entity::stats::HEALTH;

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
            if let Some(hp) = e.stats.get_mut(HEALTH) {
                hp.update(0, 50, 100);
                hp.dirty = false;
            }
            // In combat: threatened_mobs is non-empty
            e.threatened_mobs.insert(200);
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        let entity = mgr.get_entity(1).unwrap();
        let hp = entity.stats.get(HEALTH).unwrap();
        assert_eq!(
            hp.cur, 50,
            "in-combat player must NOT regen (threatened_mobs non-empty)"
        );
        assert!(
            rx.try_recv().is_err(),
            "no onStatUpdate when nothing changed"
        );
    }

    #[tokio::test]
    async fn regen_tick_skips_player_at_full_health() {
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
            // Health at max — default is 100/100
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        regen_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "full health player must not trigger onStatUpdate"
        );
    }
}
