use super::space_registry::resolve_space_id_fallback;
use crate::mercury::world_data::historical_cellblocks::historical_cellblocks;
use crate::mercury::DEFAULT_SPACE_ID;

#[test]
fn space_id_mapping_known_worlds() {
    // Resolve through the actual fallback table rather than hardcoded literals
    // so this test stays meaningful if `DEFAULT_SPACE_ID` or the world list
    // ever shifts. We only assert the structural invariants the rest of the
    // server relies on: each known world maps to a distinct id, and every id
    // lives in cell 1's id space (high 16 bits == 1).
    let castle_cellblock = resolve_space_id_fallback("Castle_CellBlock").unwrap();
    let sgc_w1 = resolve_space_id_fallback("SGC_W1").unwrap();
    let combat_sim = resolve_space_id_fallback("CombatSim").unwrap();

    assert_ne!(castle_cellblock, sgc_w1);
    assert_ne!(sgc_w1, combat_sim);
    assert_ne!(castle_cellblock, combat_sim);

    assert_eq!(castle_cellblock >> 16, 1);
    assert_eq!(sgc_w1 >> 16, 1);
    assert_eq!(combat_sim >> 16, 1);
}

/// The unknown-world default is the stock CellBlock startup space. The
/// historical CellBlock worlds are "unknown" to this table, so without the
/// explicit refusal each of them would resolve there.
#[test]
fn historical_cellblocks_never_fall_back_into_the_stock_cellblock_space() {
    assert_eq!(
        resolve_space_id_fallback("Castle_CellBlock"),
        Some(DEFAULT_SPACE_ID)
    );
    for world in historical_cellblocks() {
        assert_eq!(
            resolve_space_id_fallback(world.world),
            None,
            "{} must fail closed, not reuse a stock space id",
            world.world
        );
    }
}

/// The Debug Area (1300) is shared, not instanced, but it has no stand-in
/// space either: the default would put a player bound for `DebugArea` in the
/// stock CellBlock space. Every added world fails closed; a shipped world the
/// table does not list keeps the old default (see the function's docs for
/// why that is not widened yet).
#[test]
fn every_added_world_fails_closed_including_the_debug_area() {
    use crate::mercury::world_data::added_worlds::ADDED_WORLDS;

    assert_eq!(resolve_space_id_fallback("DebugArea"), None);
    for world in &ADDED_WORLDS {
        assert_eq!(
            resolve_space_id_fallback(world.world),
            None,
            "{}",
            world.world
        );
    }
    assert_eq!(
        resolve_space_id_fallback("Ihpet_Crater_Light"),
        Some(DEFAULT_SPACE_ID),
        "the shipped map the Debug Area plays on is not an added world"
    );
}
