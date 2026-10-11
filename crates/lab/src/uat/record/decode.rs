//! LR-02: the inbound decoder, which turns recorded client calls into `Call` values.

use serde_json::Value;

use crate::uat::golden::key_args::{decode_inbound, InArg};
use crate::uat::golden::normalise::{glob_match, DEFAULT_IGNORE};

/// One decoded inbound client call.
#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    /// `setTargetID`: the target the next `interact(0)` uses.
    SetTarget { target: u32 },
    /// `moveItem`: `item` is the item instance id.
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
/// None only when the channel or the target is unreadable.
fn chat(args: &[u8]) -> Option<Call> {
    let mut r = Reader { buf: args, at: 0 };
    let channel = r.u8()?;
    r.wstring()?;
    // The tap keeps only the first 256 payload bytes (wire-log's
    // HEX_DUMP_CAP), so a long line's text is cut short. Keeping the units
    // that are there lets a long `.bug` line still vanish and a long dot
    // command still reach the row's setup.
    let text = r.wstring_truncated();
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
        Some(utf16_le(bytes))
    }

    /// A WSTRING that may be cut short: the whole units present, up to its
    /// count. Empty when even the count is missing.
    fn wstring_truncated(&mut self) -> String {
        let Some(count) = self.take().map(u32::from_le_bytes) else {
            return String::new();
        };
        let rest = self.buf.len().saturating_sub(self.at);
        let n = (count as usize).saturating_mul(2).min(rest);
        // A dangling half unit is dropped by utf16_le.
        utf16_le(self.take_slice(n).unwrap_or_default())
    }
}

/// UTF-16LE units to a string; an odd trailing byte is ignored.
fn utf16_le(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    String::from_utf16_lossy(&units)
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
#[path = "decode_tests.rs"]
mod tests;
