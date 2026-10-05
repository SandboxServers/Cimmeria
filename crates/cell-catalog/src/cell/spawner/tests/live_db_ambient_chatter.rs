//! Live-DB guards for the ambient chatter seed (Debug Area DA-09, the System
//! Lords' summit): `resources.ambient_chatter_groups` / `_lines`, read
//! through the startup loader, and the NPCs they name.
//!
//! Each guard is about seed content that loads without error and still
//! fails in game:
//!
//! * a line whose speaker tag names no NPC in the group's world, or more than
//!   one, which the tick skips (or speaks from whichever it finds first);
//! * a speaker with no display name, whose line the tick skips because a say
//!   line with a blank speaker renders as garbage;
//! * a speaker players can attack, which would turn the summit into a fight;
//! * an exchange with a gap in its line indices or a first line that waits,
//!   which reads as a stalled scene.
mod live_db {
    use std::collections::BTreeSet;
    use std::time::Duration;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const SUMMIT: i32 = 1;
    const DEBUG_AREA: i32 = 1300;
    /// The faction players may attack (`player_may_attack_pve`).
    const HOSTILE_FACTION: i32 = 10;

    /// The summit loads through the startup loader as one world-1300 group
    /// whose exchanges each start at once and run in index order with no
    /// gap. Revert proof: drop the group row (the loader drops its lines) or
    /// delete a line, and this fails.
    #[tokio::test]
    async fn the_summit_loads_as_complete_scenes() {
        let pool = require_db_or_skip!();
        let catalog = load_ambient_chatter(&pool).await.expect("chatter loads");
        let summit = catalog
            .groups
            .iter()
            .find(|g| g.group_id == SUMMIT)
            .expect("group 1, the System Lords' summit");
        assert_eq!(summit.world_id, DEBUG_AREA);
        assert!(summit.exchanges.len() >= 10, "a summit worth listening to");

        let indices: Vec<(i32, Vec<i32>)> = sqlx::query_as(
            "SELECT exchange_id, array_agg(line_index ORDER BY line_index) \
             FROM resources.ambient_chatter_lines WHERE group_id = $1 \
             GROUP BY exchange_id ORDER BY exchange_id",
        )
        .bind(SUMMIT)
        .fetch_all(&pool)
        .await
        .expect("line indices");
        for (exchange_id, idx) in &indices {
            assert_eq!(
                idx,
                &(0..idx.len() as i32).collect::<Vec<_>>(),
                "exchange {exchange_id} has a gap in its line indices"
            );
        }
        for ex in &summit.exchanges {
            assert!(
                ex.lines.len() >= 2,
                "exchange {} is a scene, not a line",
                ex.exchange_id
            );
            assert_eq!(
                ex.lines[0].delay,
                Duration::ZERO,
                "exchange {} opens with a pause",
                ex.exchange_id
            );
        }
    }

    /// Every speaker tag names exactly one NPC in the group's world, with a
    /// display name, that players cannot attack. Revert proof: misspell a
    /// line's tag, clear a lord's `name_id`, or set one to faction 10.
    #[tokio::test]
    async fn every_speaker_is_one_named_friendly_npc_in_the_world() {
        let pool = require_db_or_skip!();
        let tags: Vec<(i32, String)> = sqlx::query_as(
            "SELECT DISTINCT g.world_id, l.speaker_tag \
             FROM resources.ambient_chatter_lines l \
             JOIN resources.ambient_chatter_groups g USING (group_id)",
        )
        .fetch_all(&pool)
        .await
        .expect("speaker tags");
        assert!(!tags.is_empty());
        let mut bad = BTreeSet::new();
        for (world_id, tag) in &tags {
            let rows: Vec<(i32, Option<String>)> = sqlx::query_as(
                "SELECT t.faction, x.text \
                 FROM resources.spawnlist s \
                 JOIN resources.entity_templates t USING (template_id) \
                 LEFT JOIN resources.texts x ON x.moniker_id = t.name_id AND x.language = 1033 \
                 WHERE s.world_id = $1 AND s.tag = $2",
            )
            .bind(world_id)
            .bind(tag)
            .fetch_all(&pool)
            .await
            .expect("speaker rows");
            match rows.as_slice() {
                [(faction, name)] => {
                    if *faction == HOSTILE_FACTION {
                        bad.insert(format!("{tag}: faction 10, so players can attack it"));
                    }
                    if name.as_deref().is_none_or(|n| n.trim().is_empty()) {
                        bad.insert(format!("{tag}: no display name"));
                    }
                }
                other => {
                    bad.insert(format!(
                        "{tag}: {} spawn rows in world {world_id}",
                        other.len()
                    ));
                }
            }
        }
        assert!(bad.is_empty(), "bad chatter speakers: {bad:#?}");
    }
}
