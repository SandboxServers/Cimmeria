//! Key bindings as the client reports them (`getBindingKey(action, n)`),
//! and the lab key that presses one.
//!
//! The stock action-bar code reads `keyInfo.key` (a number, bound when
//! `> 0`) and shows `keyInfo.vkeyShortText` / `vkeyText`, so `key` is a
//! Windows virtual-key code. Any other fields the table carries (modifier
//! flags, if the client has them) are kept verbatim and honoured when their
//! name says shift/ctrl/alt.

use serde_json::{json, Map, Value};

/// One binding slot (`n` = 1 or 2) of an action.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Binding {
    pub slot: u8,
    /// Virtual-key code; `None` or 0 = unbound.
    pub vk: Option<u32>,
    pub text: Option<String>,
    pub short_text: Option<String>,
    /// Every field of the Lua table, as text.
    pub raw: Map<String, Value>,
}

impl Binding {
    pub fn is_bound(&self) -> bool {
        self.vk.is_some_and(|k| k > 0)
    }

    /// Modifier keys the binding needs held (lab key names), from any field
    /// named like a modifier whose value is true.
    pub fn modifiers(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (k, v) in &self.raw {
            let on = matches!(v.as_str(), Some("true") | Some("1"));
            if !on {
                continue;
            }
            let k = k.to_ascii_lowercase();
            let m = if k.contains("shift") {
                "Shift"
            } else if k.contains("ctrl") || k.contains("control") {
                "Ctrl"
            } else if k.contains("alt") {
                "Alt"
            } else {
                continue;
            };
            if !out.contains(&m) {
                out.push(m);
            }
        }
        out
    }

    pub fn to_json(&self) -> Value {
        json!({
            "slot": self.slot,
            "vk": self.vk,
            "text": self.text,
            "short_text": self.short_text,
            "lab_key": self.vk.and_then(lab_key_for_vk),
            "modifiers": self.modifiers(),
            "raw": self.raw,
        })
    }
}

/// Parse one binding: `name=value` pairs joined by U+001F (the hotbar
/// chunk's encoding). An empty string is "no table".
pub fn parse_binding(slot: u8, s: &str) -> Binding {
    let mut b = Binding {
        slot,
        ..Default::default()
    };
    for pair in s.split('\u{1f}').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        b.raw.insert(k.to_string(), json!(v));
        match k {
            "key" => b.vk = v.parse::<f64>().ok().map(|f| f as u32),
            "vkeyText" => b.text = Some(v.to_string()).filter(|t| !t.is_empty()),
            "vkeyShortText" => b.short_text = Some(v.to_string()).filter(|t| !t.is_empty()),
            _ => {}
        }
    }
    b
}

/// The lab key name (see `supervisor::keys::named`) that posts this
/// virtual-key code, or `None` when the lab cannot press it.
pub fn lab_key_for_vk(vk: u32) -> Option<String> {
    let name = match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from(vk as u8).to_string(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x1B => "Escape".into(),
        0x20 => "Space".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0xBD => "Minus".into(),
        0xBE => "Period".into(),
        0xBF => "Slash".into(),
        0xC0 => "Grave".into(),
        _ => return None,
    };
    // Only hand out names the key table really knows.
    crate::supervisor::keys::named(&name).map(|_| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const US: char = '\u{1f}';

    #[test]
    fn a_bound_digit_parses_and_maps_to_its_lab_key() {
        let s = format!("key=49{US}vkeyShortText=1{US}vkeyText=1");
        let b = parse_binding(1, &s);
        assert!(b.is_bound());
        assert_eq!(b.vk, Some(0x31));
        assert_eq!(b.short_text.as_deref(), Some("1"));
        assert_eq!(b.to_json()["lab_key"], "1");
        assert!(b.modifiers().is_empty());
    }

    #[test]
    fn unbound_and_empty_tables_are_not_bound() {
        assert!(!parse_binding(2, "").is_bound());
        assert!(!parse_binding(2, &format!("key=0{US}vkeyText=")).is_bound());
        assert_eq!(parse_binding(2, &format!("key=0{US}vkeyText=")).text, None);
    }

    #[test]
    fn modifier_fields_are_honoured() {
        let b = parse_binding(
            1,
            &format!("key=49{US}shift=true{US}ctrlDown=1{US}alt=false"),
        );
        let m = b.modifiers();
        assert_eq!(m.len(), 2);
        assert!(m.contains(&"Shift") && m.contains(&"Ctrl") && !m.contains(&"Alt"));
    }

    #[test]
    fn vk_codes_map_to_known_lab_keys_only() {
        assert_eq!(lab_key_for_vk(0x41).as_deref(), Some("A"));
        assert_eq!(lab_key_for_vk(0x30).as_deref(), Some("0"));
        assert_eq!(lab_key_for_vk(0x70).as_deref(), Some("F1"));
        assert_eq!(lab_key_for_vk(0x7B).as_deref(), Some("F12"));
        assert_eq!(lab_key_for_vk(0xBD).as_deref(), Some("Minus"));
        // Numpad 1 and an OEM key the lab cannot post.
        assert_eq!(lab_key_for_vk(0x61), None);
        assert_eq!(lab_key_for_vk(0xBB), None);
    }

    #[test]
    fn a_value_with_an_equals_sign_survives() {
        let b = parse_binding(1, &format!("key=187{US}vkeyText=="));
        assert_eq!(b.text.as_deref(), Some("="));
    }
}
