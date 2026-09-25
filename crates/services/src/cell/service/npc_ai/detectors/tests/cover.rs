//! `decision_outcome=no_cover` reasons and `cover.coverage event=space_summary`.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, add_threat_player, ai_tick, castle_mgr, cellblock_mgr, rows, NPC};
use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality};
use crate::test_support::LogCapture;

/// Parse `db/resources/AI/Seed/cover_nodes.sql` into the nodes the loader
/// builds. Columns: `chunk_id, node_id, pos_x, pos_y, pos_z, orient,
/// height, quality, width, tail`; set ids are `world_id * 100000 + n`.
fn seed_nodes() -> Option<Vec<CoverNode>> {
    let sql = std::fs::read_to_string("../../db/resources/AI/Seed/cover_nodes.sql").ok()?;
    let nodes: Vec<CoverNode> = sql
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('('))
        .map(|l| {
            let f: Vec<&str> = l
                .trim_start_matches('(')
                .split(',')
                .map(str::trim)
                .collect();
            let chunk_id: i32 = f[0].parse().unwrap();
            CoverNode {
                chunk_id,
                node_id: f[1].parse().unwrap(),
                world_id: chunk_id / 100_000,
                pos: Vector3::new(
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                ),
                orient: f[5].parse().unwrap(),
                height: CoverHeight::Mid,
                quality: CoverQuality::Good,
                width: f[8].parse().unwrap(),
                tail: [0; 4],
            }
        })
        .collect();
    Some(nodes)
}

/// Mark the fixture NPC as one that would look for cover.
fn make_cover_npc(mgr: &mut crate::cell::space_manager::SpaceManager, pos: [f32; 3]) {
    add_npc(mgr, "Castle_CellBlock", pos, None, AiState::Idle);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.use_cover = true;
    npc.is_stationary = false;
}

fn field_usize(row: &crate::test_support::Captured, k: &str) -> usize {
    row.fields
        .get(k)
        .unwrap_or_else(|| panic!("{k} missing: {row:?}"))
        .parse()
        .unwrap()
}

/// **The real world-12 seed (NA21) covers Cellblock.** About 236 extracted
/// world-space nodes in world 12, most of them standing on the rebuilt
/// mesh, and a cover-seeking NPC present: INFO, no WARN. Before NA21 this
/// world had no usable cover (audit C1).
#[test]
fn the_cellblock_seed_is_on_the_mesh_and_does_not_warn() {
    let Some((mut mgr, space_id)) = cellblock_mgr() else {
        return;
    };
    let Some(nodes) = seed_nodes() else {
        return;
    };
    mgr.cover = Cover::from_loaded(Vec::new(), nodes);
    make_cover_npc(&mut mgr, [-96.25, 34.591, -91.59]);
    let logs = LogCapture::install();
    mgr.cover_loaded();
    let found = rows(&logs, "cover.coverage", "space_summary");
    assert_eq!(found.len(), 1, "{found:#?}");
    let row = &found[0];
    assert_eq!(row.level, Level::INFO, "{row:?}");
    assert!(row.has_field("space_id", &space_id.to_string()));
    assert!(row.has_field("world_id", "12"), "{row:?}");
    assert!(row.has_field("cover_npcs", "1"), "{row:?}");
    let in_world = field_usize(row, "nodes_in_world");
    let on_mesh = field_usize(row, "nodes_on_mesh");
    assert!(
        (200..=280).contains(&in_world),
        "about 236 world-12 nodes expected, got {in_world}"
    );
    assert!(
        on_mesh * 2 > in_world,
        "most extracted nodes stand on the mesh: {on_mesh} of {in_world}"
    );
    assert!(field_usize(row, "sets_in_world") > 0);
}

/// **Acceptance: the WARN.** A meshed space whose NPCs use cover, in a
/// world with no cover nodes (the index holds another world's only), gets
/// `reason = no_usable_cover`. Revert-proof: making `SpaceCoverage::warns`
/// return `false` turns this row INFO.
#[test]
fn a_meshed_world_with_cover_npcs_and_no_nodes_warns() {
    let Some((mut mgr, _)) = cellblock_mgr() else {
        return;
    };
    let elsewhere = CoverNode {
        chunk_id: 800_001,
        node_id: 0,
        world_id: 8,
        pos: Vector3::new(-96.25, 34.6, -91.59),
        orient: 0.0,
        height: CoverHeight::Mid,
        quality: CoverQuality::Good,
        width: 1.0,
        tail: [0; 4],
    };
    mgr.cover = Cover::from_loaded(Vec::new(), vec![elsewhere]);
    make_cover_npc(&mut mgr, [-96.25, 34.591, -91.59]);
    let logs = LogCapture::install();
    mgr.cover_loaded();
    let found = rows(&logs, "cover.coverage", "space_summary");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].level, Level::WARN, "{:?}", found[0]);
    assert!(found[0].has_field("reason", "no_usable_cover"));
    assert!(found[0].has_field("nodes_in_world", "0"));
    assert!(found[0].has_field("cover_npcs", "1"));
}

/// No cover-seeking NPC, nothing to warn about.
#[test]
fn a_world_without_cover_npcs_does_not_warn() {
    let Some((mut mgr, _)) = cellblock_mgr() else {
        return;
    };
    let logs = LogCapture::install();
    mgr.cover_loaded();
    let found = rows(&logs, "cover.coverage", "space_summary");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].level, Level::INFO, "{:?}", found[0]);
    assert!(found[0].has_field("cover_npcs", "0"));
}

fn no_cover_rows(
    logs: &crate::test_support::LogCaptureGuard,
) -> Vec<crate::test_support::Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == "npc_ai" && c.has_field("decision_outcome", "no_cover"))
        .collect()
}

/// The silent `NoCover => {}` arm now says why (audit C7). Revert-proof:
/// restoring the silent arm in `fight_cover::route_via_cover` leaves no row.
#[tokio::test]
async fn an_out_of_range_chase_with_no_cover_says_no_candidate_in_radius() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    mgr.get_entity_mut(NPC).unwrap().use_cover = true;
    add_threat_player(&mut mgr, "Castle", [45.0, 0.0, 0.0]);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    let found = no_cover_rows(&logs);
    assert_eq!(found.len(), 1, "{:#?}", logs.all());
    assert!(
        found[0].has_field("reason", "no_candidate_in_radius"),
        "{:?}",
        found[0]
    );
    assert!(found[0].has_field("candidates_scanned", "0"));
}

#[tokio::test]
async fn in_range_and_use_cover_false_have_their_own_reasons() {
    for (use_cover, target_x, reason) in [
        (true, 5.0, "in_range_no_better_slot"),
        (false, 45.0, "use_cover_false"),
    ] {
        let mut mgr = castle_mgr();
        add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
        mgr.get_entity_mut(NPC).unwrap().use_cover = use_cover;
        add_threat_player(&mut mgr, "Castle", [target_x, 0.0, 0.0]);
        let logs = LogCapture::install();
        ai_tick(&mut mgr).await;
        let found = no_cover_rows(&logs);
        assert_eq!(found.len(), 1, "{reason}: {:#?}", logs.all());
        assert!(found[0].has_field("reason", reason), "{:?}", found[0]);
    }
}
