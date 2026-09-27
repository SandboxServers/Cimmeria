use super::space_registry::resolve_space_id_fallback;
use crate::mercury::world_data::historical_cellblocks::HISTORICAL_CELLBLOCKS;
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
    for world in &HISTORICAL_CELLBLOCKS {
        assert_eq!(
            resolve_space_id_fallback(world.world),
            None,
            "{} must fail closed, not reuse a stock space id",
            world.world
        );
    }
}
