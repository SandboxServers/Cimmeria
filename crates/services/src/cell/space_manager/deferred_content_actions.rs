//! Deferred content-engine actions — `content_actions.delay_ms > 0`.
//!
//! A content chain's action list can mix immediate actions (`delay_ms ==
//! 0`, executed inline by `content::executor::execute_actions`) with
//! delayed ones. Delayed actions are queued here, on `SpaceManager`, keyed
//! by the entity that triggered the chain, and drained by
//! `content::executor::deferred_content_action_tick` on the existing
//! 100ms cell tick — the same cadence `pending_attack_tick` /
//! `pending_reload_tick` already use for "resume this later without
//! blocking the tick" work (see `cell::service::ticks::pending_holster`).
//!
//! Storing the queue on `SpaceManager` rather than spawning a
//! `tokio::time::sleep` task was the deliberate choice for C08a: a spawned
//! task would need its own way to reach `&mut SpaceManager`, which is
//! owned exclusively by the single-threaded cell message loop — there's no
//! channel for "run this closure against space_mgr later" today, and
//! building one would be new surface area for a one-field feature. A
//! queue keyed by entity_id, drained on the tick that's already running,
//! needs none of that, and gets disconnect/leave-space cleanup for free:
//! `SpaceManager::destroy_entity` is already the single choke point that
//! scrubs entity-keyed session state (`authoring_changes`,
//! `autosave_spawns`) on disconnect, cross-world teleport, and every other
//! path that removes a `CellEntity` from its space. Adding one more
//! `.remove(&entity_id)` there means a deferred action can never fire
//! against a torn-down entity — there is no separate cancellation path to
//! forget.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_content_engine::actions::Action;

use super::SpaceManager;

/// One action deferred by `content_actions.delay_ms`, queued against the
/// entity whose chain resolution produced it.
pub(crate) struct PendingContentAction {
    /// Wall-clock deadline; `deferred_content_action_tick` fires this once
    /// `Instant::now() >= fire_at`.
    pub(crate) fire_at: Instant,
    /// The chain that produced this action — carried through purely for
    /// logging/tracing parity with the immediate-execution path.
    pub(crate) chain_id: i64,
    pub(crate) action: Action,
    /// Snapshotted at schedule time rather than re-looked-up at drain
    /// time — the entity's `player_id` shouldn't change between
    /// scheduling and firing, and looking it up fresh would be one more
    /// thing that has to handle "entity vanished" instead of relying on
    /// the queue-entry-removed-on-destroy invariant above.
    pub(crate) player_id: i32,
    /// Resolution-time `ExecutionContext.params` snapshot — same payload
    /// `ResolvedActions.params` carries for the immediate path, needed by
    /// actions like `RemoveItem` that read `instance_id`.
    pub(crate) params: HashMap<String, serde_json::Value>,
}

impl SpaceManager {
    /// Queue `action` to fire against `entity_id` once `delay_ms` has
    /// elapsed. Callers must not pass `delay_ms <= 0` — that's the
    /// immediate-execution case and belongs in
    /// `content::executor::execute_actions`'s inline branch, not here.
    pub(crate) fn schedule_content_action(
        &mut self,
        entity_id: u32,
        chain_id: i64,
        action: Action,
        player_id: i32,
        delay_ms: i32,
        params: HashMap<String, serde_json::Value>,
    ) {
        let fire_at = Instant::now() + std::time::Duration::from_millis(delay_ms.max(0) as u64);
        self.pending_content_actions
            .entry(entity_id)
            .or_default()
            .push(PendingContentAction {
                fire_at,
                chain_id,
                action,
                player_id,
                params,
            });
    }

    /// Remove and return every deferred action whose `fire_at` has
    /// elapsed as of `now`, paired with the owning entity id. Actions
    /// still in the future are left queued, in their original order.
    ///
    /// Entries for the same entity are returned in queue (= original
    /// `sort_order`) order — the "same delay_ms fires in sort_order"
    /// acceptance criterion falls out of this for free, since actions
    /// scheduled together in one `execute_actions` call are pushed to
    /// the same entity's queue in that call's iteration order and this
    /// scan never reorders the queue, only filters it.
    pub(crate) fn take_ready_content_actions(
        &mut self,
        now: Instant,
    ) -> Vec<(u32, PendingContentAction)> {
        let mut ready = Vec::new();
        self.pending_content_actions.retain(|&entity_id, queue| {
            let mut i = 0;
            while i < queue.len() {
                if queue[i].fire_at <= now {
                    ready.push((entity_id, queue.remove(i)));
                } else {
                    i += 1;
                }
            }
            !queue.is_empty()
        });
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> SpaceManager {
        SpaceManager::new(1)
    }

    #[test]
    fn schedule_then_take_ready_returns_nothing_before_deadline() {
        let mut mgr = mgr();
        mgr.schedule_content_action(
            1,
            100,
            Action::GrantXP { amount: 1 },
            50,
            60_000,
            HashMap::new(),
        );

        let ready = mgr.take_ready_content_actions(Instant::now());
        assert!(
            ready.is_empty(),
            "a 60s delay must not be ready immediately"
        );
    }

    #[test]
    fn take_ready_content_actions_returns_elapsed_entries() {
        let mut mgr = mgr();
        mgr.schedule_content_action(1, 100, Action::GrantXP { amount: 1 }, 50, 0, HashMap::new());

        // delay_ms=0 still goes through the scheduler in this unit test
        // (execute_actions itself short-circuits delay==0 to the inline
        // path in production) — fire_at is "now", so a check against
        // "now + 1ms" must find it ready.
        let ready =
            mgr.take_ready_content_actions(Instant::now() + std::time::Duration::from_millis(1));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, 1);
        assert_eq!(ready[0].1.chain_id, 100);
        assert_eq!(ready[0].1.player_id, 50);
    }

    #[test]
    fn take_ready_content_actions_preserves_queue_order_for_same_entity() {
        let mut mgr = mgr();
        let now = Instant::now();
        for i in 0..5 {
            mgr.pending_content_actions
                .entry(1)
                .or_default()
                .push(PendingContentAction {
                    fire_at: now,
                    chain_id: 1000 + i,
                    action: Action::GrantXP { amount: i as u64 },
                    player_id: 50,
                    params: HashMap::new(),
                });
        }

        let ready = mgr.take_ready_content_actions(now);
        assert_eq!(ready.len(), 5);
        let chain_ids: Vec<i64> = ready.iter().map(|(_, p)| p.chain_id).collect();
        assert_eq!(
            chain_ids,
            vec![1000, 1001, 1002, 1003, 1004],
            "entries sharing the same fire_at must drain in original push order"
        );
    }

    #[tokio::test]
    async fn disconnect_entity_drops_pending_content_actions() {
        let mut mgr = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.connect_entity(1);

        mgr.schedule_content_action(
            1,
            100,
            Action::GrantXP { amount: 1 },
            50,
            60_000,
            HashMap::new(),
        );
        assert!(mgr.pending_content_actions.contains_key(&1));

        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        mgr.disconnect_entity(1, &tx).await;

        assert!(
            !mgr.pending_content_actions.contains_key(&1),
            "disconnect_entity must scrub any pending deferred content \
             actions for the entity"
        );
    }

    #[test]
    fn destroy_entity_drops_pending_content_actions() {
        let mut mgr = mgr();
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();

        mgr.schedule_content_action(
            1,
            100,
            Action::GrantXP { amount: 1 },
            50,
            60_000,
            HashMap::new(),
        );
        assert!(mgr.pending_content_actions.contains_key(&1));

        mgr.destroy_entity(1);

        assert!(
            !mgr.pending_content_actions.contains_key(&1),
            "destroy_entity must scrub any pending deferred content actions \
             for the entity — a disconnect or leave-space before the delay \
             elapses must drop the action, not fire it later"
        );
    }
}
