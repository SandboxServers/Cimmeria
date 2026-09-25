//! `decision_outcome=no_cover` reasons and `cover.coverage event=space_summary`.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, add_threat_player, ai_tick, castle_mgr, cellblock_mgr, rows, NPC};
use crate::cell::cover::{Cover, CoverHeight, CoverNode, CoverQuality};
use crate::test_support::LogCapture;

/// Parse `db/resources/AI/Seed/cover_nodes.sql` into the nodes the loader
/// would build (`pos = (pos_x, pos_y, pos_z)`, exactly as `cover/loader.rs`
/// maps the columns).
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
            CoverNode {
                chunk_id: f[0].parse().unwrap(),
                node_id: f[1].parse().unwrap(),
                pos: Vector3::new(
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                    f[4].parse().unwrap(),
                ),
                orient: f[5].parse().unwrap(),
                height: CoverHeight::Mid,
                quality: CoverQuality::Good,
                tail: [0; 4],
            }
        })
        .collect();
    Some(nodes)
}

/// **Acceptance: the `cover.coverage` WARN on Cellblock with today's
/// seed.** Note what it proves about the plan's own criterion: thousands
/// of seed nodes land on the mesh's `y ≈ 0.2` ground plane by coincidence,
/// so "WARN when nothing is on the mesh" would have stayed silent. The
/// origin-centred-set test is what fires. Revert-proof: removing the
/// `PrefabLocalCoordinates` arm of `SpaceCoverage::gap` turns this row INFO.
#[test]
fn cellblock_with_todays_cover_seed_warns() {
    let Some((mut mgr, space_id)) = cellblock_mgr() else {
        return;
    };
    let Some(nodes) = seed_nodes() else {
        return;
    };
    assert!(nodes.len() > 9_000, "seed parse: {}", nodes.len());
    mgr.cover = Cover::from_loaded(Vec::new(), nodes);
    let logs = LogCapture::install();
    mgr.cover_loaded();
    let found = rows(&logs, "cover.coverage", "space_summary");
    assert_eq!(found.len(), 1, "{found:#?}");
    let row = &found[0];
    assert_eq!(row.level, Level::WARN, "{row:?}");
    assert!(
        row.has_field("reason", "prefab_local_coordinates"),
        "{row:?}"
    );
    assert!(row.has_field("world", "Castle_CellBlock"));
    assert!(row.has_field("space_id", &space_id.to_string()));
    let on_mesh = &row.fields["nodes_on_mesh"];
    assert_ne!(
        on_mesh, "0",
        "documented: the seed's nodes DO hit the ground plane by coincidence"
    );
}

/// Cover authored in world space — two sets standing on the mess-hall
/// floor, far from the origin — is usable: INFO, no reason.
#[test]
fn placed_cover_on_the_mesh_is_info() {
    let Some((mut mgr, _)) = cellblock_mgr() else {
        return;
    };
    let node = |chunk_id, node_id, dx: f32| CoverNode {
        chunk_id,
        node_id,
        pos: Vector3::new(-96.25 + dx, 34.6, -91.59),
        orient: 0.0,
        height: CoverHeight::Mid,
        quality: CoverQuality::Good,
        tail: [0; 4],
    };
    mgr.cover = Cover::from_loaded(
        Vec::new(),
        vec![node(1, 0, 0.0), node(1, 1, 1.0), node(2, 0, 2.0)],
    );
    let logs = LogCapture::install();
    mgr.cover_loaded();
    let found = rows(&logs, "cover.coverage", "space_summary");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].level, Level::INFO, "{:?}", found[0]);
    assert!(found[0].has_field("nodes_on_mesh", "3"), "{:?}", found[0]);
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
