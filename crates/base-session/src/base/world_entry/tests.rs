use super::space_registry::resolve_space_id_fallback;

#[test]
fn space_id_mapping_known_worlds() {
    // Resolve through the actual fallback table rather than hardcoded literals
    // so this test stays meaningful if `DEFAULT_SPACE_ID` or the world list
    // ever shifts. We only assert the structural invariants the rest of the
    // server relies on: each known world maps to a distinct id, and every id
    // lives in cell 1's id space (high 16 bits == 1).
    let castle_cellblock = resolve_space_id_fallback("Castle_CellBlock");
    let sgc_w1 = resolve_space_id_fallback("SGC_W1");
    let combat_sim = resolve_space_id_fallback("CombatSim");

    assert_ne!(castle_cellblock, sgc_w1);
    assert_ne!(sgc_w1, combat_sim);
    assert_ne!(castle_cellblock, combat_sim);

    assert_eq!(castle_cellblock >> 16, 1);
    assert_eq!(sgc_w1 >> 16, 1);
    assert_eq!(combat_sim >> 16, 1);
}
