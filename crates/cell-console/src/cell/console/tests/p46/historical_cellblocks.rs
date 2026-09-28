//! `.gotolocation` into the historical CellBlock worlds (1201–1207), through
//! the real `entities/spaces.xml`: a typed name reaches its world under the
//! declared spelling, as a transfer into a new instance of that world, and
//! `Castle_CellBlock` still names the stock world on the way back.
//!
//! The base half (the world id and client map the transfer turns into) is
//! pinned by `cimmeria-wire`'s `world_data::tests::historical_cellblocks`.

use super::*;

/// The comparison spot from the historical CellBlock handoff.
const UAT_ARGS: [&str; 3] = ["-334.231", "73.472", "-228.026"];
const UAT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

/// `setup_worlds()` plus every world the shipped `spaces.xml` declares.
fn setup_with_shipped_worlds() -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup_worlds();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../entities/spaces.xml");
    let xml = std::fs::read_to_string(path).expect("entities/spaces.xml readable");
    mgr.parse_spaces_xml(&xml)
        .expect("shipped spaces.xml parses");
    (mgr, gm)
}

async fn goto_location(mgr: &mut SpaceManager, caller: u32, world: &str) -> Traffic {
    let args = [world, UAT_ARGS[0], UAT_ARGS[1], UAT_ARGS[2]];
    run("gotolocation", caller, &args, None, mgr).await
}

#[tokio::test]
async fn gotolocation_reaches_each_historical_cellblock_under_its_declared_name() {
    for world in [
        "CellBlock43",
        "CellBlock55",
        "CellBlock57",
        "CellBlock58",
        "CellBlock60",
        "CellBlock62",
        "CellBlock63",
    ] {
        let (mut mgr, gm) = setup_with_shipped_worlds();
        // Typed the way a GM might type it; the transfer must carry the
        // spaces.xml spelling, which is what the base looks the world up by.
        let t = goto_location(&mut mgr, gm, &world.to_lowercase()).await;
        assert_eq!(
            t.only_gate_travel(),
            &(gm, world.to_string(), None, UAT_POS),
            "{world}: no instance is loaded yet, so the create path allocates a new one"
        );
    }
}

/// From inside one historical world, another historical world and the stock
/// CellBlock are both real transfers, each naming its own world. Nothing
/// collapses onto the stock CellBlock or onto the world already occupied.
#[tokio::test]
async fn gotolocation_moves_between_historical_worlds_and_back_to_stock() {
    let (mut mgr, _gm) = setup_with_shipped_worlds();
    let archaeologist = 700;
    let cellblock43 = spawn_named_player(
        &mut mgr,
        archaeologist,
        "CellBlock43",
        UAT_POS,
        "Archaeologist",
    );
    mgr.get_entity_mut(archaeologist).unwrap().is_player = true;
    assert_eq!(mgr.world_name_for_space(cellblock43), Some("CellBlock43"));

    let t = goto_location(&mut mgr, archaeologist, "CellBlock55").await;
    assert_eq!(
        t.only_gate_travel(),
        &(archaeologist, "CellBlock55".to_string(), None, UAT_POS)
    );

    let (mut mgr, _gm) = setup_with_shipped_worlds();
    spawn_named_player(
        &mut mgr,
        archaeologist,
        "CellBlock43",
        UAT_POS,
        "Archaeologist",
    );
    mgr.get_entity_mut(archaeologist).unwrap().is_player = true;
    let t = goto_location(&mut mgr, archaeologist, "Castle_CellBlock").await;
    assert_eq!(
        t.only_gate_travel(),
        &(archaeologist, "Castle_CellBlock".to_string(), None, UAT_POS)
    );
}

/// `.gotolocation CellBlockNN` with no coordinates: every historical world
/// shares the stock Cellblock's new-character start (owner, 2026-09-27).
/// Reverting the historical arm refuses each one with "no known entry point".
#[tokio::test]
async fn gotolocation_historical_cellblock_alone_lands_on_the_cellblock_start() {
    for world in [
        "CellBlock43",
        "CellBlock55",
        "CellBlock57",
        "CellBlock58",
        "CellBlock60",
        "CellBlock62",
        "CellBlock63",
    ] {
        let (mut mgr, gm) = setup_with_shipped_worlds();
        let t = run("gotolocation", gm, &[&world.to_lowercase()], None, &mut mgr).await;
        assert_eq!(
            t.only_gate_travel(),
            &(gm, world.to_string(), None, UAT_POS),
            "{world}"
        );
        assert!(
            t.mentions("[new-character start]"),
            "{world}: {:?}",
            t.feedback
        );
    }
}
