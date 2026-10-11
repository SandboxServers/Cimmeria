//! The inventory family: `onActiveSlotUpdate` (70), `onRemoveItem` (71) and
//! `onUpdateItem` (72), all player-only.

use super::primitives::Reader;
use super::{InvItem, S2CEvent, SemanticError};

/// The fewest bytes one `InvItem` can take: five INT32s, a UINT8, then the
/// durability, the ammo count (zero ammo types), the current ammo type and
/// the charges.
const MIN_ITEM_BYTES: usize = 37;

/// `onActiveSlotUpdate`: the bag and slot the player has active.
pub(super) fn active_slot_update(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onActiveSlotUpdate", args);
    let event = S2CEvent::ActiveSlotUpdate {
        bag_id: r.i32()?,
        slot_id: r.i32()?,
    };
    r.finish()?;
    Ok(event)
}

/// `onRemoveItem`: the ids of items that left the inventory.
pub(super) fn remove_item(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onRemoveItem", args);
    let item_ids = r.array_i32()?;
    r.finish()?;
    Ok(S2CEvent::RemoveItem { item_ids })
}

/// `onUpdateItem`: a batch of items whose state changed.
pub(super) fn update_item(args: &[u8]) -> Result<S2CEvent, SemanticError> {
    let mut r = Reader::new("onUpdateItem", args);
    let count = r.u32()? as usize;
    // The count is checked against the bytes that follow before anything is
    // reserved, so a forged count cannot allocate more than the body holds.
    // The items start after the 4-byte count, which is why `at` is 4.
    let need = count.saturating_mul(MIN_ITEM_BYTES);
    if need > args.len() - 4 {
        return Err(SemanticError::Short {
            method: "onUpdateItem",
            at: 4,
            need,
            len: args.len(),
        });
    }
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(inv_item(&mut r)?);
    }
    r.finish()?;
    Ok(S2CEvent::UpdateItem { items })
}

/// One `InvItem` in wire order. `is_bound` travels as a byte, so any non-zero
/// value counts as bound rather than only the value 1.
fn inv_item(r: &mut Reader<'_>) -> Result<InvItem, SemanticError> {
    Ok(InvItem {
        id: r.i32()?,
        dbid: r.i32()?,
        stack_size: r.i32()?,
        slot_id: r.i32()?,
        container_id: r.i32()?,
        is_bound: r.u8()? != 0,
        durability: r.i32()?,
        ammo_types: r.array_i32()?,
        cur_ammo_type: r.i32()?,
        charges: r.i32()?,
    })
}

#[cfg(test)]
mod tests {
    use super::{active_slot_update, remove_item, update_item};
    use crate::semantic::{InvItem, S2CEvent, SemanticError};

    /// Little-endian bytes for a run of INT32 values.
    fn i32s(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    // The fixture has no inventory message (F11), so these hand-built bytes
    // are the only check here. WC-14 checks the same layout against the real
    // server's bytes.

    /// Each field is a distinct value, so a swapped width or order cannot
    /// decode equal. Item one has two ammo types; item two has none.
    #[test]
    fn update_item_decodes_two_items() {
        let mut args = 2u32.to_le_bytes().to_vec();
        args.extend(i32s(&[1, 2, 3, 4, 5]));
        args.push(1);
        args.extend(i32s(&[6, 2, 1, 2, 7, 8]));
        args.extend(i32s(&[9, 10, 11, 12, 13]));
        args.push(0);
        args.extend(i32s(&[14, 0, 15, 16]));
        assert_eq!(
            update_item(&args),
            Ok(S2CEvent::UpdateItem {
                items: vec![
                    InvItem {
                        id: 1,
                        dbid: 2,
                        stack_size: 3,
                        slot_id: 4,
                        container_id: 5,
                        is_bound: true,
                        durability: 6,
                        ammo_types: vec![1, 2],
                        cur_ammo_type: 7,
                        charges: 8,
                    },
                    InvItem {
                        id: 9,
                        dbid: 10,
                        stack_size: 11,
                        slot_id: 12,
                        container_id: 13,
                        is_bound: false,
                        durability: 14,
                        ammo_types: vec![],
                        cur_ammo_type: 15,
                        charges: 16,
                    },
                ],
            })
        );
    }

    #[test]
    fn update_item_rejects_a_forged_count() {
        // The count says 100,000 items; the body holds one 37-byte item.
        let mut args = 100_000u32.to_le_bytes().to_vec();
        args.extend(i32s(&[1, 2, 3, 4, 5]));
        args.push(0);
        args.extend(i32s(&[6, 0, 7, 8]));
        assert_eq!(
            update_item(&args),
            Err(SemanticError::Short {
                method: "onUpdateItem",
                at: 4,
                need: 3_700_000,
                len: 41,
            })
        );
    }

    /// `ItemIdList` is a count of three, then the ids; `BagId` 3 and `SlotId`
    /// 9 are distinct so the two INT32s cannot be read in the other order.
    #[test]
    fn remove_item_and_active_slot_round_trip() {
        let mut remove = 3u32.to_le_bytes().to_vec();
        remove.extend(i32s(&[101, 102, 103]));
        assert_eq!(
            remove_item(&remove),
            Ok(S2CEvent::RemoveItem {
                item_ids: vec![101, 102, 103],
            })
        );
        assert_eq!(
            active_slot_update(&i32s(&[3, 9])),
            Ok(S2CEvent::ActiveSlotUpdate {
                bag_id: 3,
                slot_id: 9,
            })
        );
    }

    #[test]
    fn active_slot_update_rejects_a_trailing_byte() {
        let mut args = i32s(&[3, 9]);
        args.push(0);
        assert_eq!(
            active_slot_update(&args),
            Err(SemanticError::Trailing {
                method: "onActiveSlotUpdate",
                extra: 1,
            })
        );
    }
}
