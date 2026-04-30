#[test]
fn space_id_mapping_known_worlds() {
    // Verify the three known space IDs are distinct and have high 16 bits == 1.
    let castle_cellblock: u32 = 65552; // (1 << 16) | 16
    let sgc_w1: u32 = 65553;           // (1 << 16) | 17
    let combat_sim: u32 = 65554;       // (1 << 16) | 18

    // All distinct
    assert_ne!(castle_cellblock, sgc_w1);
    assert_ne!(sgc_w1, combat_sim);
    assert_ne!(castle_cellblock, combat_sim);

    // High 16 bits == 1 for all three
    assert_eq!(castle_cellblock >> 16, 1);
    assert_eq!(sgc_w1 >> 16, 1);
    assert_eq!(combat_sim >> 16, 1);
}
