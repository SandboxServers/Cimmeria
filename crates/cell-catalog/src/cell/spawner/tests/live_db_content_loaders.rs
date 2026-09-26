//! Live-DB sanity tests for the mission, objective and dialog loaders.
//!
//! Split out of `live_db_loaders.rs` (which crossed the 700-line hard cap when
//! CA02 replaced the NULL-dialog guard) along the theme the four tests already
//! shared: everything the content/dialog side of the spawner loads. The bodies
//! are unchanged.
//!
//! `load_dialog_set_maps_retains_rows_with_null_dialog_id` is the CA02 defect-B3
//! guard: the loader used to drop `dialog_id IS NULL` rows, which made an
//! interaction-only bind a no-op.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn load_mission_defs_only_includes_missions_with_a_step() {
        let pool = require_db_or_skip!();
        let map = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        assert!(!map.is_empty(), "seeded mission_steps has rows");
        for (mission_id, entry) in &map {
            assert!(
                entry.step_id > 0,
                "mission {mission_id} has non-positive step_id {}",
                entry.step_id
            );
        }
    }

    #[tokio::test]
    async fn load_step_objectives_groups_by_step_id() {
        let pool = require_db_or_skip!();
        let map = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        assert!(!map.is_empty());
        for (step_id, objs) in &map {
            assert!(
                !objs.is_empty(),
                "step {step_id} present in map with no objectives — should have been filtered out"
            );
        }
    }

    /// **Regression guard (CA02 / defect B3):** rows with a NULL `dialog_id`
    /// must survive the load, carrying `dialog_id: None`.
    ///
    /// These are the interaction-only rows — an indicator bit (`!`, `?`, quest
    /// glow) with no dialog behind the click. Seven of them are bound by the
    /// Castle content (3062, 3071, 3073, 5828, 5829, 5846, 5863); the loader
    /// used to drop every one, so `add_dialog_set` was a cache miss and the
    /// indicator never reached the client.
    ///
    /// The inverse of this test (`..._drops_rows_with_null_dialog_id`) pinned
    /// the old behaviour and is deliberately replaced rather than kept.
    #[tokio::test]
    async fn load_dialog_set_maps_retains_rows_with_null_dialog_id() {
        let pool = require_db_or_skip!();
        let map = load_dialog_set_maps(&pool)
            .await
            .expect("load_dialog_set_maps must succeed");
        assert!(!map.is_empty(), "seeded dialog_set_maps must load rows");

        // A cached dialog id, when present, is still a real positive id —
        // pins that widening to Option didn't open a "0 means no dialog" hole.
        for (set_map_id, entry) in &map {
            if let Some(dialog_id) = entry.dialog_id {
                assert!(
                    dialog_id > 0,
                    "dialog_set_map_id {set_map_id} surfaced with non-positive dialog_id {dialog_id}"
                );
            }
        }

        // Every NULL-dialog row in the seed must be in the cache with
        // `dialog_id: None`. Reverting the loader to the `if let Some(..)`
        // insert fails here on the first id.
        let null_ids: Vec<i32> = sqlx::query_scalar(
            "SELECT dialog_set_map_id FROM resources.dialog_set_maps WHERE dialog_id IS NULL",
        )
        .fetch_all(&pool)
        .await
        .expect("query must succeed");
        assert!(
            !null_ids.is_empty(),
            "seed must still contain interaction-only rows for this guard to mean anything"
        );
        for id in &null_ids {
            let entry = map.get(id).unwrap_or_else(|| {
                panic!("dialog_set_map_id {id} has NULL dialog_id in DB but was dropped from the cache")
            });
            assert_eq!(
                entry.dialog_id, None,
                "dialog_set_map_id {id} must cache as None, not a substituted id"
            );
        }

        // Castle row 3062 specifically: `Castle.py` binds it on Sgt. Gerschon
        // to raise `!` (INT_AStoryMissionActive, bit 24). Both halves matter —
        // presence AND the flag value, because a present-but-zero-flag entry
        // would bind silently and show nothing.
        let gerschon = map
            .get(&3062)
            .expect("Castle dialog_set_map 3062 must be cached");
        assert_eq!(gerschon.dialog_id, None);
        assert_eq!(
            gerschon.interaction_flags, 16_777_216,
            "row 3062 must carry INT_AStoryMissionActive (bit 24)"
        );
    }

    /// **Regression guard for the monologue dialog cache:** dialog 2982
    /// is the Castle Cellblock wake-up monologue, two screens both
    /// `speaker_id = 0`. The cache must include it OR the
    /// `DisplayDialog` monologue fallback never triggers and the
    /// cellblock opening narration silently never shows.
    ///
    /// Also asserts the predicate is correctly exclusive — a dialog
    /// with any non-zero speaker_id row must NOT surface in the
    /// monologue set (else the executor would bind the player as the
    /// NPC for that dialog and blank the NPC portrait — the bug the
    /// original abort gate was designed to prevent).
    #[tokio::test]
    async fn load_monologue_dialog_ids_includes_cellblock_wakeup() {
        let pool = require_db_or_skip!();
        let ids = load_monologue_dialog_ids(&pool)
            .await
            .expect("load_monologue_dialog_ids must succeed");

        // Dialog 2982 = Castle Cellblock wake-up monologue.
        assert!(
            ids.contains(&2982),
            "dialog 2982 (cellblock wake-up monologue) must be in the monologue set; \
             without it the chain-1001 DisplayDialog continues to silently never fire"
        );

        // Pick any dialog id from the seed that has at least one
        // non-zero speaker_id screen — it must NOT be in the cache.
        // Fetch a representative dialog and assert.
        let mixed_dialog: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_id FROM resources.dialog_screens \
             WHERE speaker_id <> 0 \
             GROUP BY dialog_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(d) = mixed_dialog {
            assert!(
                !ids.contains(&d),
                "dialog {d} has at least one non-zero speaker_id screen — it must NOT be \
                 in the monologue set, or the executor's monologue fallback would \
                 incorrectly bind the player for an NPC dialog"
            );
        }

        // A dialog with at least one NULL `speaker_id` screen must NOT
        // be classified as a monologue. The original predicate
        // (`COUNT(*) FILTER (WHERE speaker_id <> 0) = 0`) treated NULL
        // as "not non-zero" and wrongly admitted such dialogs — the
        // exact mis-binding the cache exists to prevent.
        let null_speaker_dialog: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_id FROM resources.dialog_screens \
             WHERE speaker_id IS NULL \
             GROUP BY dialog_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(d) = null_speaker_dialog {
            assert!(
                !ids.contains(&d),
                "dialog {d} has at least one NULL speaker_id screen — it must NOT be \
                 in the monologue set; treating NULL as monologue would mis-bind the \
                 player as the dialog context entity for unknown-speaker dialogs"
            );
        }
    }
}
