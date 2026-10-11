use super::*;
use crate::uat::record::log::read_log;
use serde_json::json;

const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

/// An inbound tap row with `args` as INT32 fields after the 4-byte prefix.
fn inbound(name: &str, method: i32, args: &[i32]) -> Value {
    json!({
        "dir": "in",
        "msg_name": name,
        "method_index": method,
        "msg_id": null,
        "args_hex": args_hex(args),
    })
}

fn args_hex(fields: &[i32]) -> String {
    let mut hex = "00000000".to_string();
    for f in fields {
        for b in f.to_le_bytes() {
            hex.push_str(&format!("{b:02x}"));
        }
    }
    hex
}

/// The fixture's `interact` (tick 32) and `dialogButtonChoice` (tick 36)
/// decode to the target and dialog the ledger records. Found by name:
/// the fixture's own record numbers are not the ones the spec quotes.
#[test]
fn fs_p3_interact_and_close_decode() {
    let log = read_log(FIXTURE).unwrap();
    let rows = || log.ticks.iter().flat_map(|t| t.tap.iter());
    let interact = rows().find(|r| r["msg_name"] == "interact").unwrap();
    assert_eq!(
        decode_call(interact),
        Some(Call::Interact { target: 100751 })
    );
    let close = rows()
        .find(|r| r["msg_name"] == "dialogButtonChoice")
        .unwrap();
    assert_eq!(
        decode_call(close),
        Some(Call::DialogChoice {
            dialog: 3995,
            button: -1
        })
    );
    let region = rows()
        .find(|r| r["msg_name"] == "triggerClientHintedGenericRegion")
        .unwrap();
    assert_eq!(
        decode_call(region),
        Some(Call::Region {
            region: 14,
            entering: false
        })
    );
    let goto = rows().find(|r| r["msg_name"] == "gmGotoXYZ").unwrap();
    assert_eq!(
        decode_call(goto),
        Some(Call::GmGoto {
            pos: [-325.0, 73.6, -212.8]
        })
    );
    let out = rows().find(|r| r["dir"] == "out").unwrap();
    assert_eq!(decode_call(out), None);
}

/// A payload one field short, a bad hex string or a hostile WSTRING count
/// is `Other` for the call's name, never a panic.
#[test]
fn a_short_payload_is_other_not_a_panic() {
    let other = |name: &str, method: i32| Call::Other {
        name: name.to_string(),
        method_index: method,
    };
    let row = |name: &str, method: i32, hex: &str| json!({"dir": "in", "msg_name": name, "method_index": method, "msg_id": null, "args_hex": hex});
    assert_eq!(
        decode_call(&row("interact", 74, "0000")),
        Some(other("interact", 74))
    );
    // Three of the target's four bytes.
    assert_eq!(
        decode_call(&row("interact", 74, "00000000010000")),
        Some(other("interact", 74))
    );
    // The full target decodes, so the short cases above are the length.
    assert_eq!(
        decode_call(&row("interact", 74, "0000000001000000")),
        Some(Call::Interact { target: 1 })
    );
    assert_eq!(
        decode_call(&row("moveItem", 38, "00000000010000")),
        Some(other("moveItem", 38))
    );
    assert_eq!(
        decode_call(&row("useAbility", 68, "0000000001000000")),
        Some(other("useAbility", 68))
    );
    assert_eq!(
        decode_call(&row("useItem", 39, "zz")),
        Some(other("useItem", 39))
    );
    assert_eq!(
        decode_call(&row("useItem", 39, "000")),
        Some(other("useItem", 39))
    );
    // A text count of 0xffffffff units with nothing behind it is a cut-short
    // text: a chat with no units, not a refusal.
    assert_eq!(
        decode_call(&row("sendPlayerCommunication", 2, "0000000000ffffffff")),
        Some(Call::Chat {
            channel: 0,
            text: String::new()
        })
    );
    // A target count with its units missing is unreadable.
    assert_eq!(
        decode_call(&row("sendPlayerCommunication", 2, "0005000000")),
        Some(other("sendPlayerCommunication", 2))
    );
    // No channel byte at all.
    assert_eq!(
        decode_call(&row("sendPlayerCommunication", 2, "")),
        Some(other("sendPlayerCommunication", 2))
    );
    assert_eq!(
        decode_call(&row("someNewCall", 500, "00000000")),
        Some(other("someNewCall", 500))
    );
    // A row with no name or a non-string direction is skipped, not a panic.
    assert_eq!(decode_call(&json!({"dir": "in"})), None);
    assert_eq!(
        decode_call(&json!({"dir": 3, "msg_name": "interact"})),
        None
    );
}

/// The tap numbers base methods from 0 too: `chatJoin` (0xC0) is
/// method 0, which must not read as `setTargetID`.
#[test]
fn a_base_method_is_not_read_as_a_cell_method() {
    let row = json!({"dir": "in", "msg_name": "chatJoin", "method_index": 0, "msg_id": 0xC0, "args_hex": "0000000001000000"});
    assert_eq!(
        decode_call(&row),
        Some(Call::Other {
            name: "chatJoin".into(),
            method_index: 0
        })
    );
    let row = json!({"dir": "in", "msg_name": "setTargetID", "method_index": 0, "msg_id": 0x80, "args_hex": "000000008f890100"});
    assert_eq!(decode_call(&row), Some(Call::SetTarget { target: 100751 }));
}

/// Ambient messages are noise, and a chat line decodes its text.
#[test]
fn noise_and_chat_decode() {
    assert_eq!(
        decode_call(&inbound("avatarUpdateExplicit", -1, &[])),
        Some(Call::Noise)
    );
    assert_eq!(
        decode_call(&inbound("requestEntityUpdate", -1, &[])),
        Some(Call::Noise)
    );
    let text: Vec<u16> = ".stand".encode_utf16().collect();
    // A base method: no entity-id prefix.
    let mut args = vec![0]; // channel
    args.extend([0; 4]); // empty target
    args.extend((text.len() as u32).to_le_bytes());
    for unit in text {
        args.extend(unit.to_le_bytes());
    }
    let hex: String = args.iter().map(|b| format!("{b:02x}")).collect();
    let row = json!({"dir": "in", "msg_name": "unknown", "msg_id": 0xC2, "method_index": 2, "args_hex": hex});
    assert_eq!(
        decode_call(&row),
        Some(Call::Chat {
            channel: 0,
            text: ".stand".into()
        })
    );
}

/// An inbound cell call as the tap writes it: methods from 61 ride 0xBD with
/// a sub-slot byte after the entity id, the rest 0x80 + index.
fn cell(name: &str, method: u8, fields: &[i32]) -> Value {
    let (msg_id, mut args) = if method >= 61 {
        (0xBD, vec![0, 0, 0, 0, method - 61])
    } else {
        (0x80 + method, vec![0, 0, 0, 0])
    };
    for f in fields {
        args.extend(f.to_le_bytes());
    }
    let hex: String = args.iter().map(|b| format!("{b:02x}")).collect();
    json!({"dir": "in", "msg_name": name, "method_index": method, "msg_id": msg_id, "args_hex": hex})
}

/// `useAbility` is cell method 68, 0xBD sub-slot 7 (cell-method dispatch
/// table): the sub-slot byte is skipped, then ability, then target.
#[test]
fn use_ability_decodes_through_the_bd_sub_slot() {
    assert_eq!(
        decode_call(&cell("useAbility", 68, &[1234, 100751])),
        Some(Call::UseAbility {
            ability: 1234,
            target: 100751
        })
    );
}

/// `moveItem` keeps the instance id that `decode_inbound` skips, and every
/// field lands in its own place.
#[test]
fn move_item_keeps_its_item_id() {
    assert_eq!(
        decode_call(&cell("moveItem", 38, &[123456, 1, 7, 3])),
        Some(Call::MoveItem {
            item: 123456,
            bag: 1,
            slot: 7,
            qty: 3
        })
    );
}

#[test]
fn use_item_reads_item_then_target() {
    assert_eq!(
        decode_call(&cell("useItem", 39, &[55, 100751])),
        Some(Call::UseItem {
            item: 55,
            target: 100751
        })
    );
}

/// A `.bug` line longer than the tap's 256-byte cap is still a chat line:
/// its text is the whole units the cap kept.
#[test]
fn a_chat_cut_short_by_the_tap_cap_still_decodes() {
    let text = format!(".bug {}", "x".repeat(300));
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut args = vec![3]; // channel
    args.extend([0; 4]); // empty target
    args.extend((units.len() as u32).to_le_bytes());
    for unit in &units {
        args.extend(unit.to_le_bytes());
    }
    // The tap keeps 256 bytes: 9 of header, then 123 whole units and half of
    // the 124th.
    args.truncate(256);
    let hex: String = args.iter().map(|b| format!("{b:02x}")).collect();
    let row = json!({"dir": "in", "msg_name": "sendPlayerCommunication", "method_index": 2, "msg_id": 0xC2, "args_hex": hex});
    assert_eq!(
        decode_call(&row),
        Some(Call::Chat {
            channel: 3,
            text: text[..123].to_string()
        })
    );
}
