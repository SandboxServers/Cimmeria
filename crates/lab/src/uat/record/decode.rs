//! LR-02: the inbound decoder, which turns recorded client calls into `Call` values.

use serde_json::Value;

use crate::uat::golden::key_args::{decode_inbound, InArg};
use crate::uat::golden::normalise::{glob_match, DEFAULT_IGNORE};

/// One decoded inbound client call.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    /// `setTargetID`: the target the next `interact(0)` uses.
    SetTarget { target: u32 },
    /// `moveItem`: the item instance id is read but not kept.
    MoveItem {
        item: i32,
        bag: i32,
        slot: i32,
        qty: i32,
    },
    /// `useItem`.
    UseItem { item: i32, target: u32 },
    /// `useAbility`.
    UseAbility { ability: i32, target: u32 },
    /// `interact`; a target of 0 means the current target.
    Interact { target: u32 },
    /// `dialogButtonChoice`; a button of -1 is the close.
    DialogChoice { dialog: i32, button: i32 },
    /// `triggerClientHintedGenericRegion`.
    Region { region: i32, entering: bool },
    /// `gmGotoXYZ`: a teleport, replaced by the pose.
    GmGoto { pos: [f32; 3] },
    /// A chat line the person typed (`sendPlayerCommunication`).
    Chat { channel: u8, text: String },
    /// A message that only means the client is alive (`DEFAULT_IGNORE`).
    Noise,
    /// Any other inbound call, for a TODO line.
    Other { name: String, method_index: i32 },
}

/// One tap row (`dir = "in"` only; None for "out").
pub fn decode_call(row: &Value) -> Option<Call> {
    if row["dir"].as_str()? != "in" {
        return None;
    }
    let name = row["msg_name"].as_str()?;
    let method = row["method_index"]
        .as_i64()
        .and_then(|m| i32::try_from(m).ok())
        .unwrap_or(-1);
    let msg_id = row["msg_id"].as_u64().and_then(|id| u8::try_from(id).ok());
    let args = row["args_hex"]
        .as_str()
        .and_then(hex_bytes)
        .unwrap_or_default();
    let other = || Call::Other {
        name: name.to_string(),
        method_index: method,
    };
    if DEFAULT_IGNORE.iter().any(|glob| glob_match(glob, name)) {
        return Some(Call::Noise);
    }
    if name == "sendPlayerCommunication" || (name == "unknown" && msg_id == Some(0xC2)) {
        return Some(chat(&args).unwrap_or_else(other));
    }
    // The tap numbers base methods (0xC0 and up) from 0 as well, so
    // `chatJoin` (0xC0) has method_index 0 like `setTargetID`. Only a cell
    // method's index means anything to the table below.
    if msg_id.is_some_and(|id| id >= 0xC0) {
        return Some(other());
    }
    Some(method_call(method, msg_id, &args).unwrap_or_else(other))
}

/// The call a listed method index makes; None for an unlisted method or a
/// payload too short for its fields.
fn method_call(method: i32, msg_id: Option<u8>, args: &[u8]) -> Option<Call> {
    let mut r = Reader::fields(args, msg_id);
    Some(match method {
        0 => Call::SetTarget {
            target: r.i32()? as u32,
        },
        38 => {
            // The item instance id comes first; decode_inbound skips it.
            let item = r.i32()?;
            match decode_inbound(38, msg_id, args)?.as_slice() {
                [(_, InArg::Int(bag)), (_, InArg::Int(slot)), (_, InArg::Int(qty))] => {
                    Call::MoveItem {
                        item,
                        bag: *bag as i32,
                        slot: *slot as i32,
                        qty: *qty as i32,
                    }
                }
                _ => return None,
            }
        }
        39 => Call::UseItem {
            item: r.i32()?,
            target: r.i32()? as u32,
        },
        68 => Call::UseAbility {
            ability: r.i32()?,
            target: r.i32()? as u32,
        },
        74 => match decode_inbound(74, msg_id, args)?.as_slice() {
            [(_, InArg::Entity(target))] => Call::Interact { target: *target },
            _ => return None,
        },
        75 => match decode_inbound(75, msg_id, args)?.as_slice() {
            [(_, InArg::Int(dialog)), (_, InArg::Int(button))] => Call::DialogChoice {
                dialog: *dialog as i32,
                button: *button as i32,
            },
            _ => return None,
        },
        85 => match decode_inbound(85, msg_id, args)?.as_slice() {
            [(_, InArg::Int(region)), (_, InArg::Int(entering))] => Call::Region {
                region: *region as i32,
                entering: *entering != 0,
            },
            _ => return None,
        },
        163 => match decode_inbound(163, msg_id, args)?.as_slice() {
            [(_, InArg::Pos(pos))] => Call::GmGoto { pos: *pos },
            _ => return None,
        },
        _ => return None,
    })
}

/// `sendPlayerCommunication`: channel UINT8, target WSTRING (not kept), text
/// WSTRING. A base method, so unlike the cell calls there is no entity-id
/// prefix: the base handler reads the channel from byte 0
/// (`crates/base/src/base/dispatch/chat.rs`, `handle_send_player_communication`).
fn chat(args: &[u8]) -> Option<Call> {
    let mut r = Reader { buf: args, at: 0 };
    let channel = r.u8()?;
    r.wstring()?;
    let text = r.wstring()?;
    Some(Call::Chat { channel, text })
}

/// Reads little-endian fields in order. Every read is bounds-checked, so a
/// short or hostile capture gives None instead of a panic.
struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// A cell call's fields: after the entity-id prefix, and the sub-slot
    /// byte too on 0xBD (README F5).
    fn fields(buf: &'a [u8], msg_id: Option<u8>) -> Self {
        let at = if msg_id == Some(0xBD) { 5 } else { 4 };
        Reader { buf, at }
    }

    fn take_slice(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let bytes = self.buf.get(self.at..end)?;
        self.at = end;
        Some(bytes)
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take_slice(N)?.try_into().ok()
    }

    fn i32(&mut self) -> Option<i32> {
        self.take().map(i32::from_le_bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take().map(|[b]: [u8; 1]| b)
    }

    /// A WSTRING: a u32 count of UTF-16 code units, then the units.
    fn wstring(&mut self) -> Option<String> {
        let count = u32::from_le_bytes(self.take()?) as usize;
        let bytes = self.take_slice(count.checked_mul(2)?)?;
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect();
        Some(String::from_utf16_lossy(&units))
    }
}

/// The bytes of a hex string; None when the length is odd or a digit is not hex.
fn hex_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
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
        // A text count of 0xffffffff units with nothing behind it.
        assert_eq!(
            decode_call(&row("sendPlayerCommunication", 2, "0000000000ffffffff")),
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
}
