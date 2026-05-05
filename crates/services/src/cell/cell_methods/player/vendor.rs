use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

use super::constants::*;

fn read_i32_array(args: &[u8], offset: &mut usize) -> Option<Vec<(i32, i32)>> {
    if args.len() < *offset + 4 {
        return None;
    }
    let count = u32::from_le_bytes([
        args[*offset],
        args[*offset + 1],
        args[*offset + 2],
        args[*offset + 3],
    ]) as usize;
    *offset += 4;

    // Bound count against remaining bytes BEFORE allocating: a malformed packet
    // with count=u32::MAX would otherwise reserve ~32 GiB up-front and OOM.
    let remaining = args.len().saturating_sub(*offset);
    if count.saturating_mul(8) > remaining {
        return None;
    }

    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        if args.len() < *offset + 8 {
            return None;
        }
        let item_id = i32::from_le_bytes([
            args[*offset],
            args[*offset + 1],
            args[*offset + 2],
            args[*offset + 3],
        ]);
        let quantity = i32::from_le_bytes([
            args[*offset + 4],
            args[*offset + 5],
            args[*offset + 6],
            args[*offset + 7],
        ]);
        *offset += 8;
        items.push((item_id, quantity));
    }
    Some(items)
}

fn read_trailing_template_id(args: &[u8], offset: usize) -> Option<i32> {
    if args.len() >= offset + 4 {
        Some(i32::from_le_bytes([
            args[offset],
            args[offset + 1],
            args[offset + 2],
            args[offset + 3],
        ]))
    } else {
        None
    }
}

/// Resolved vendor session for the player, looked up from server-side state.
///
/// `vendor_template_id` is read from the vendor entity itself rather than trusted
/// from the client's request — this prevents a client from opening vendor A
/// then submitting a purchase against vendor B's item lists.
struct VendorSession {
    player_id: i32,
    vendor_entity_id: i32,
    /// Server-side authoritative template id for the currently-open vendor.
    /// May be `None` if the vendor entity has no template attached.
    server_template_id: Option<i32>,
}

fn vendor_context(entity_id: u32, space_mgr: &SpaceManager) -> Option<VendorSession> {
    let player = space_mgr.get_entity(entity_id)?;
    let player_id = player.player_id?;
    let vendor_entity_id_u32 = player.vendor_entity?;
    let vendor_entity_id = vendor_entity_id_u32 as i32;
    let server_template_id = space_mgr
        .get_entity(vendor_entity_id_u32)
        .and_then(|v| v.template_id);
    Some(VendorSession {
        player_id,
        vendor_entity_id,
        server_template_id,
    })
}

/// Validate that the client-supplied template id matches the vendor that was
/// opened. Returns the authoritative server-side id on success, or `None` after
/// logging the mismatch.
fn validate_template_id(
    entity_id: u32,
    op: &str,
    session: &VendorSession,
    client_template_id: i32,
) -> Option<i32> {
    match session.server_template_id {
        Some(server_id) if server_id == client_template_id => Some(server_id),
        Some(server_id) => {
            tracing::warn!(
                entity_id,
                op,
                server_template_id = server_id,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: client supplied template id does not match opened vendor"
            );
            None
        }
        None => {
            tracing::warn!(
                entity_id,
                op,
                client_template_id,
                vendor_entity_id = session.vendor_entity_id,
                "vendor op rejected: opened vendor has no template id (server cannot validate)"
            );
            None
        }
    }
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        TRAIN_ABILITY => {
            if args.len() >= 4 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::debug!(entity_id, ability_id, "trainAbility (not yet implemented)");
            }
            true
        }

        PURCHASE_ITEMS | SELL_ITEMS | BUYBACK_ITEMS | REPAIR_ITEMS | RECHARGE_ITEMS => {
            let op_name = match method_index {
                PURCHASE_ITEMS => "purchaseItems",
                SELL_ITEMS => "sellItems",
                BUYBACK_ITEMS => "buybackItems",
                REPAIR_ITEMS => "repairItems",
                RECHARGE_ITEMS => "rechargeItems",
                _ => unreachable!(),
            };

            let session = match vendor_context(entity_id, space_mgr) {
                Some(s) => s,
                None => {
                    tracing::warn!(
                        entity_id,
                        op = op_name,
                        "vendor op: no active vendor context (player_id or vendor_entity unset)"
                    );
                    return true;
                }
            };

            let mut offset = 0;
            let items = match read_i32_array(args, &mut offset) {
                Some(items) => items,
                None => {
                    tracing::warn!(
                        entity_id,
                        op = op_name,
                        args_len = args.len(),
                        "vendor op: malformed item array in args"
                    );
                    return true;
                }
            };

            let trailing_template_id = read_trailing_template_id(args, offset);

            // For paid Repair/Recharge, `trailing_template_id` is optional (None
            // signals "free repair"). For Purchase/Sell/Buyback, it's required;
            // and in all cases where the client supplied one, it must match the
            // vendor that was actually opened so a client can't spoof it.
            let validated_template_id = match trailing_template_id {
                Some(client_id) => {
                    match validate_template_id(entity_id, op_name, &session, client_id) {
                        Some(server_id) => Some(server_id),
                        None => return true,
                    }
                }
                None => None,
            };

            let msg = match method_index {
                PURCHASE_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::PurchaseVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
                        return true;
                    }
                },
                SELL_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::SellVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
                        return true;
                    }
                },
                BUYBACK_ITEMS => match validated_template_id {
                    Some(vendor_template_id) => CellToBaseMsg::BuybackVendorItems {
                        entity_id,
                        player_id: session.player_id,
                        vendor_entity_id: session.vendor_entity_id,
                        vendor_template_id,
                        items,
                    },
                    None => {
                        tracing::warn!(
                            entity_id,
                            op = op_name,
                            "vendor op: missing vendor_template_id"
                        );
                        return true;
                    }
                },
                REPAIR_ITEMS => CellToBaseMsg::RepairInventoryItems {
                    entity_id,
                    player_id: session.player_id,
                    item_ids: items.iter().map(|(id, _)| *id).collect(),
                    vendor_template_id: validated_template_id,
                },
                RECHARGE_ITEMS => CellToBaseMsg::RechargeInventoryItems {
                    entity_id,
                    player_id: session.player_id,
                    item_ids: items.iter().map(|(id, _)| *id).collect(),
                    vendor_template_id: validated_template_id,
                },
                _ => unreachable!(),
            };

            if let Err(e) = tx.send(msg).await {
                tracing::warn!(
                    entity_id,
                    op = op_name,
                    "vendor op: cell->base channel closed: {e}"
                );
            }
            true
        }

        _ => false,
    }
}

#[cfg(test)]
mod read_i32_array_tests {
    use super::read_i32_array;

    #[test]
    fn empty_array_returns_empty_vec() {
        let buf = 0u32.to_le_bytes();
        let mut off = 0;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![]));
        assert_eq!(off, 4, "offset must advance past the count prefix");
    }

    #[test]
    fn single_pair_parses_in_order() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&12345i32.to_le_bytes());
        buf.extend_from_slice(&67i32.to_le_bytes());
        let mut off = 0;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![(12345, 67)]));
        assert_eq!(off, 12);
    }

    #[test]
    fn multiple_pairs_preserve_input_order() {
        let pairs: [(i32, i32); 3] = [(10, 1), (20, 2), (30, 3)];
        let mut buf = Vec::new();
        buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        for (id, q) in &pairs {
            buf.extend_from_slice(&id.to_le_bytes());
            buf.extend_from_slice(&q.to_le_bytes());
        }
        let mut off = 0;
        let parsed = read_i32_array(&buf, &mut off).expect("parse must succeed");
        assert_eq!(parsed, pairs.to_vec());
        assert_eq!(off, 4 + pairs.len() * 8);
    }

    #[test]
    fn returns_none_when_too_short_for_count_prefix() {
        let mut off = 0;
        assert!(read_i32_array(&[0, 1, 2], &mut off).is_none());
    }

    #[test]
    fn returns_none_when_payload_truncated_mid_pair() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&[0u8, 0u8]); // truncated 2nd pair
        let mut off = 0;
        assert!(read_i32_array(&buf, &mut off).is_none());
    }

    #[test]
    fn rejects_oversized_count_without_allocating() {
        // count = u32::MAX with only 4 bytes after the prefix would imply
        // ~32 GiB of payload. The function must reject up-front so a
        // malicious packet can't OOM the cell process via Vec::with_capacity.
        let mut buf = Vec::new();
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]); // far less than u32::MAX * 8 bytes
        let mut off = 0;
        assert!(read_i32_array(&buf, &mut off).is_none());
    }

    #[test]
    fn parses_at_nonzero_offset() {
        // The function uses `*offset` for both bounds checks and indexing, so
        // a non-zero starting offset must work the same as a fresh slice.
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // junk prefix
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&7i32.to_le_bytes());
        buf.extend_from_slice(&8i32.to_le_bytes());
        let mut off = 4;
        assert_eq!(read_i32_array(&buf, &mut off), Some(vec![(7, 8)]));
        assert_eq!(off, 16);
    }
}

#[cfg(test)]
mod vendor_context_tests {
    use super::{vendor_context, VendorSession};
    use crate::test_support::make_space_manager;

    fn assert_session(
        session: Option<VendorSession>,
        expected_player_id: i32,
        expected_vendor_entity_id: i32,
        expected_template_id: Option<i32>,
    ) {
        let s = session.expect("vendor_context should return Some");
        assert_eq!(s.player_id, expected_player_id);
        assert_eq!(s.vendor_entity_id, expected_vendor_entity_id);
        assert_eq!(s.server_template_id, expected_template_id);
    }

    #[test]
    fn returns_none_when_entity_does_not_exist() {
        let mgr = make_space_manager();
        assert!(vendor_context(99999, &mgr).is_none());
    }

    #[test]
    fn returns_none_when_player_id_missing() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // player.player_id deliberately left None.
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.vendor_entity = Some(vendor);
        }
        assert!(vendor_context(1, &mgr).is_none());
    }

    #[test]
    fn returns_none_when_vendor_entity_unset() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            // p.vendor_entity deliberately left None — no vendor opened.
        }
        assert!(vendor_context(1, &mgr).is_none());
    }

    #[test]
    fn happy_path_returns_session_with_template_id() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(v) = mgr.get_entity_mut(vendor) {
            v.template_id = Some(9001);
        }
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(vendor);
        }
        assert_session(vendor_context(1, &mgr), 4242, vendor as i32, Some(9001));
    }

    #[test]
    fn happy_path_returns_session_when_vendor_lacks_template_id() {
        // The function reads template_id authoritatively from the vendor
        // entity, falling back to None rather than fabricating a value. A
        // missing template_id surfaces as `server_template_id: None` —
        // validate_template_id at the caller treats that as a rejection
        // (so a client can't open a templateless vendor and submit ops).
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let vendor = mgr.allocate_npc_id();
        mgr.spawn_npc(vendor, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Deliberately not setting template_id on the vendor.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(vendor);
        }
        assert_session(vendor_context(1, &mgr), 4242, vendor as i32, None);
    }

    #[test]
    fn returns_session_with_no_template_when_vendor_entity_id_is_stale() {
        // The player has a stale vendor_entity pointing at an id that doesn't
        // exist (vendor despawned or never spawned). vendor_context still
        // returns Some with server_template_id: None — the caller's
        // validate_template_id arm logs and rejects the op.
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(4242);
            p.vendor_entity = Some(123456); // no entity at this id
        }
        assert_session(vendor_context(1, &mgr), 4242, 123456, None);
    }
}
