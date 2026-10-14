use super::{update_item_args, InventoryRow};

#[test]
fn committed_partial_stack_encodes_exactly_one_update_row() {
    let row = InventoryRow {
        item_id: 1234,
        type_id: 3730,
        stack_size: 1,
        slot_id: 0,
        container_id: 2,
        bound: false,
        durability: 100,
        charges: 0,
        ammo_type_ids: vec![],
        cur_ammo_type_id: 0,
    };
    let args = update_item_args(&[row]);
    assert_eq!(&args[..4], &1u32.to_le_bytes());
    assert_eq!(&args[4..8], &1234i32.to_le_bytes());
    assert_eq!(&args[8..12], &3730i32.to_le_bytes());
}
