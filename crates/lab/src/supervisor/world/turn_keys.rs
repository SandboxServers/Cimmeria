//! The pawn's turn keys, as the client binds them.
//!
//! The pawn's heading answers only to these keys (live 2026-10-12: the
//! camera's mouse-look handler moved the camera yaw offset 65.9 degrees
//! and the pawn heading not at all; Q/E turn it at about 110 degrees a
//! second, standing or walking). `getBindingKey(action, 1|2)` returns a
//! table `{key = <virtual-key code>, ...}`; an unknown or unbound action
//! returns an empty table, not an error.

use crate::supervisor::combat::binding::lab_key_for_vk;

pub const DEFAULT_LEFT: &str = "Q";
pub const DEFAULT_RIGHT: &str = "E";

/// Lua: the first and second binding of `TurnLeft` and `TurnRight` as
/// virtual-key codes (`'0'` = unbound, or needs a modifier). Returns four
/// values: left 1, right 1, left 2, right 2.
pub const TURN_KEYS_CHUNK: &str = "local function k(a, n) \
     local ok, t = pcall(getBindingKey, a, n) \
     if ok and type(t) == 'table' then \
       local v = tonumber(t.key) \
       if v ~= nil and v > 0 and not t.shift and not t.control and not t.alt then \
         return tostring(math.floor(v)) end end \
     return '0' end \
     return k('TurnLeft', 1), k('TurnRight', 1), k('TurnLeft', 2), k('TurnRight', 2)";

/// `(left, right)` lab key names from [`TURN_KEYS_CHUNK`]'s four results:
/// the first bound slot the lab can press, else the stock Q / E.
pub fn parse_turn_keys(r: &[String]) -> (String, String) {
    let pick = |a: usize, b: usize, default: &str| {
        [a, b]
            .iter()
            .filter_map(|i| r.get(*i)?.parse::<u32>().ok())
            .filter(|v| *v > 0)
            .find_map(lab_key_for_vk)
            .unwrap_or_else(|| default.to_string())
    };
    (pick(0, 2, DEFAULT_LEFT), pick(1, 3, DEFAULT_RIGHT))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn the_stock_bindings_are_q_and_e() {
        assert_eq!(
            parse_turn_keys(&s(&["81", "69", "46", "34"])),
            ("Q".into(), "E".into())
        );
    }

    #[test]
    fn an_unbound_first_slot_falls_to_the_second() {
        assert_eq!(
            parse_turn_keys(&s(&["0", "69", "46", "0"])),
            ("Delete".into(), "E".into())
        );
    }

    #[test]
    fn nothing_readable_means_q_and_e() {
        assert_eq!(parse_turn_keys(&[]), ("Q".into(), "E".into()));
        // A code the lab cannot press (999) is ignored.
        assert_eq!(
            parse_turn_keys(&s(&["999", "0", "0", "0"])),
            ("Q".into(), "E".into())
        );
    }

    #[test]
    fn the_chunk_asks_for_both_actions_and_both_slots() {
        for needle in [
            "'TurnLeft', 1",
            "'TurnRight', 1",
            "'TurnLeft', 2",
            "'TurnRight', 2",
        ] {
            assert!(TURN_KEYS_CHUNK.contains(needle), "{needle}");
        }
    }
}
