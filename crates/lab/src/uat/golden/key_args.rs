//! Which decoded fields of a message a golden keeps, and how each value is
//! written. Anything not listed here is compared by name and target only,
//! so a per-run id that is not selected cannot make two runs differ.

use serde_json::Value;

use super::labels::EntityLabels;

/// Outbound messages and the decoded fields a golden keeps for each.
pub const OUTBOUND_KEYS: &[(&str, &[&str])] = &[
    ("onDialogDisplay", &["dialog_id", "entity_id", "mission_id"]),
    ("InteractionType", &["TypeId"]),
    ("onMissionUpdate", &["MissionID", "Status"]),
    ("onStepUpdate", &["StepID", "Status"]),
    (
        "onObjectiveUpdate",
        &["ObjectiveID", "Status", "Hidden", "Optional"],
    ),
    ("onKnownAbilitiesUpdate", &["AbilityData.items"]),
    ("onPlayerCommunication", &["Channel", "Text"]),
    (
        "onSequence",
        &["kismet_event_set_seq_id", "source_id", "target_id"],
    ),
    ("onEntityProperty", &["type", "value"]),
];

/// Decoded fields holding an entity id, rendered as labels.
pub const ENTITY_FIELDS: &[&str] = &["entity_id", "source_id", "target_id"];

/// One decoded inbound argument, before it is rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum InArg {
    Int(i64),
    Entity(u32),
    Pos([f32; 3]),
}

/// Key arguments of an inbound cell call from its raw args (the tap's
/// args_hex, decoded). Skips the 4-byte entity-id prefix, and the sub-slot
/// byte too when msg_id is 0xBD (README F5). None for a method not listed
/// or args too short.
///   74  interact                          target: Entity (INT32)
///   75  dialogButtonChoice                dialog: Int, button: Int (INT32 each)
///   38  moveItem                          (INT32 item id skipped) bag, slot, qty: Int
///   163 gmGotoXYZ                         pos: Pos (3 x FLOAT)
///   85  triggerClientHintedGenericRegion  region: Int (INT32), entering: Int (UINT8)
pub fn decode_inbound(
    method_index: i32,
    msg_id: Option<u8>,
    args: &[u8],
) -> Option<Vec<(&'static str, InArg)>> {
    let start = if msg_id == Some(0xBD) { 5 } else { 4 };
    let mut c = Cursor {
        buf: args,
        at: start,
    };
    let out = match method_index {
        74 => vec![("target", InArg::Entity(c.i32()? as u32))],
        75 => vec![
            ("dialog", InArg::Int(c.i32()?.into())),
            ("button", InArg::Int(c.i32()?.into())),
        ],
        38 => {
            c.i32()?;
            vec![
                ("bag", InArg::Int(c.i32()?.into())),
                ("slot", InArg::Int(c.i32()?.into())),
                ("qty", InArg::Int(c.i32()?.into())),
            ]
        }
        163 => vec![("pos", InArg::Pos([c.f32()?, c.f32()?, c.f32()?]))],
        85 => vec![
            ("region", InArg::Int(c.i32()?.into())),
            ("entering", InArg::Int(c.u8()?.into())),
        ],
        _ => return None,
    };
    Some(out)
}

/// Reads little-endian fields in order. Every read is bounds-checked, so a
/// short or hostile capture gives None instead of a panic.
struct Cursor<'a> {
    buf: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.at.checked_add(N)?;
        let bytes = self.buf.get(self.at..end)?.try_into().ok()?;
        self.at = end;
        Some(bytes)
    }

    fn i32(&mut self) -> Option<i32> {
        self.take().map(i32::from_le_bytes)
    }

    fn f32(&mut self) -> Option<f32> {
        self.take().map(f32::from_le_bytes)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take().map(|[b]: [u8; 1]| b)
    }
}

/// A JSON value as the golden writes it: integers in decimal, floats to one
/// decimal, arrays as `[a,b]`, strings with their whitespace collapsed and
/// cut to 80 characters.
pub fn render_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() => format!("{f:.1}"),
            _ => n.to_string(),
        },
        Value::String(s) => clip(s),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(render_json).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(_) => clip(&v.to_string()),
    }
}

/// A decoded inbound argument as the golden writes it. An entity id is a
/// label, never the raw id (README D-GD3).
pub fn render_in(a: &InArg, labels: &mut EntityLabels) -> String {
    match a {
        InArg::Int(n) => n.to_string(),
        InArg::Entity(id) => labels.label(*id),
        InArg::Pos([x, y, z]) => format!("{x:.1},{y:.1},{z:.1}"),
    }
}

/// Whitespace runs collapsed to one space, then cut to 80 characters.
fn clip(s: &str) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The Praxis capture's FS-P3 window (`fixtures/fs_p3_tap.json`); each
    /// record is a message's position in that window, from 0.
    fn fixture() -> Value {
        serde_json::from_str(include_str!("fixtures/fs_p3_tap.json")).unwrap()
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// The record's name and its decoded key arguments.
    fn decode_record(tap: &Value, record: usize) -> (String, Option<Vec<(&'static str, InArg)>>) {
        let m = &tap["tap"]["messages"][record];
        let method = m["method_index"].as_i64().unwrap() as i32;
        let msg_id = m["msg_id"].as_u64().map(|id| id as u8);
        let args = hex(m["args_hex"].as_str().unwrap());
        (
            m["msg_name"].as_str().unwrap().to_string(),
            decode_inbound(method, msg_id, &args),
        )
    }

    /// The Praxis capture decodes to the values the ledger records (F5).
    /// The 0xBD sub-slot byte is skipped, so the entity id is read whole.
    #[test]
    fn inbound_calls_decode_from_the_praxis_capture() {
        let tap = fixture();
        let (name, interact) = decode_record(&tap, 15);
        assert_eq!(name, "interact");
        assert_eq!(interact, Some(vec![("target", InArg::Entity(100751))]));

        let first_choice = tap["tap"]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .position(|m| m["msg_name"] == "dialogButtonChoice")
            .unwrap();
        let (_, choice) = decode_record(&tap, first_choice);
        assert_eq!(
            choice,
            Some(vec![
                ("dialog", InArg::Int(3995)),
                ("button", InArg::Int(-1))
            ])
        );

        let (name, goto) = decode_record(&tap, 0);
        assert_eq!(name, "gmGotoXYZ");
        let goto = goto.unwrap();
        let mut labels = EntityLabels::new(8, None, None);
        assert_eq!(render_in(&goto[0].1, &mut labels), "-325.0,73.6,-212.8");

        let (name, region) = decode_record(&tap, 4);
        assert_eq!(name, "triggerClientHintedGenericRegion");
        assert_eq!(
            region,
            Some(vec![
                ("region", InArg::Int(14)),
                ("entering", InArg::Int(0))
            ])
        );
    }

    /// Each listed method needs exactly its prefix plus its fields: None one
    /// byte short (and for every shorter length, without a panic), Some at
    /// the minimum. The 0xBD sub-slot adds one byte. An unlisted method is
    /// always None.
    #[test]
    fn inbound_decode_never_panics_on_short_args() {
        // Entity-id prefix (4) plus the fields the method reads.
        let minimums = [(74, 8), (75, 12), (38, 20), (163, 16), (85, 9)];
        for (method, base) in minimums {
            for (msg_id, extra) in [(None, 0), (Some(0xA6), 0), (Some(0xBD), 1)] {
                let min = base + extra;
                for len in 0..min {
                    assert!(
                        decode_inbound(method, msg_id, &vec![0xFF; len]).is_none(),
                        "method {method} {msg_id:?} with {len} bytes"
                    );
                }
                assert!(
                    decode_inbound(method, msg_id, &vec![0; min]).is_some(),
                    "method {method} {msg_id:?} with {min} bytes"
                );
            }
        }
        assert!(decode_inbound(1, None, &[0; 32]).is_none());
    }

    fn le(fields: &[i32]) -> Vec<u8> {
        let mut out = vec![0xEE; 4]; // the entity-id prefix, skipped
        for f in fields {
            out.extend_from_slice(&f.to_le_bytes());
        }
        out
    }

    /// Without the 0xBD sub-slot the fields start at byte 4, and moveItem
    /// drops its item instance id (README D-GD4).
    #[test]
    fn inbound_fields_start_after_the_prefix_without_a_sub_slot() {
        assert_eq!(
            decode_inbound(38, Some(0xA6), &le(&[123_456, 1, 7, 3])),
            Some(vec![
                ("bag", InArg::Int(1)),
                ("slot", InArg::Int(7)),
                ("qty", InArg::Int(3))
            ])
        );
        assert_eq!(
            decode_inbound(74, None, &le(&[100_751])),
            Some(vec![("target", InArg::Entity(100_751))])
        );
        assert_eq!(
            decode_inbound(75, Some(0xA6), &le(&[3995, -1])),
            Some(vec![
                ("dialog", InArg::Int(3995)),
                ("button", InArg::Int(-1))
            ])
        );
    }

    #[test]
    fn render_json_follows_the_value_rules() {
        assert_eq!(render_json(&json!(3995)), "3995");
        assert_eq!(render_json(&json!(-325.0)), "-325.0");
        assert_eq!(render_json(&json!([579, 597])), "[579,597]");
        assert_eq!(render_json(&json!("  two\n  words ")), "two words");
        assert_eq!(render_json(&json!("x".repeat(100))).len(), 80);
    }
}
