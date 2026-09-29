//! Key names to the window-message form of a key press: virtual-key code,
//! set-1 scan code, and whether it is an extended key. Pure, so the
//! mapping and the typing plan are tested off Windows.

/// One key as `WM_KEYDOWN` / `WM_KEYUP` carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    pub vk: u8,
    pub scan: u8,
    pub extended: bool,
}

impl Key {
    const fn new(vk: u8, scan: u8, extended: bool) -> Self {
        Self { vk, scan, extended }
    }

    fn lparam(&self) -> u32 {
        1 | (u32::from(self.scan) << 16) | (u32::from(self.extended) << 24)
    }

    /// `lParam` of `WM_KEYDOWN`: repeat 1, scan code, extended flag.
    pub fn lparam_down(&self) -> isize {
        self.lparam() as isize
    }

    /// `lParam` of `WM_KEYUP`: also the previous-state and transition bits.
    pub fn lparam_up(&self) -> isize {
        (self.lparam() | (1 << 30) | (1 << 31)) as isize
    }
}

pub const SHIFT: Key = Key::new(0x10, 0x2A, false);

/// Letters `a`..`z` (VK `A`..`Z`) with their set-1 scan codes.
const LETTER_SCANS: [u8; 26] = [
    0x1E, 0x30, 0x2E, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18, 0x19,
    0x10, 0x13, 0x1F, 0x14, 0x16, 0x2F, 0x11, 0x2D, 0x15, 0x2C,
];

fn letter(c: char) -> Option<Key> {
    let lower = c.to_ascii_lowercase();
    if !lower.is_ascii_lowercase() {
        return None;
    }
    let i = (lower as u8 - b'a') as usize;
    Some(Key::new(b'A' + i as u8, LETTER_SCANS[i], false))
}

fn digit(c: char) -> Option<Key> {
    let d = c.to_digit(10)?;
    // Scan codes: 1..9 = 0x02..0x0A, 0 = 0x0B.
    let scan = if d == 0 { 0x0B } else { 0x01 + d as u8 };
    Some(Key::new(b'0' + d as u8, scan, false))
}

/// A key by name (`"W"`, `"Enter"`, `"Escape"`, `"F1"`, `"Space"`, `"1"`,
/// `"Up"`, ...), case-insensitive.
pub fn named(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if let Some(k) = letter(c).or_else(|| digit(c)) {
            return Some(k);
        }
    }
    let n = name.to_ascii_lowercase();
    if let Some(f) = n.strip_prefix('f').and_then(|s| s.parse::<u8>().ok()) {
        return match f {
            1..=10 => Some(Key::new(0x6F + f, 0x3A + f, false)),
            11 => Some(Key::new(0x7A, 0x57, false)),
            12 => Some(Key::new(0x7B, 0x58, false)),
            _ => None,
        };
    }
    Some(match n.as_str() {
        "escape" | "esc" => Key::new(0x1B, 0x01, false),
        "backspace" | "back" => Key::new(0x08, 0x0E, false),
        "tab" => Key::new(0x09, 0x0F, false),
        "enter" | "return" => Key::new(0x0D, 0x1C, false),
        "space" => Key::new(0x20, 0x39, false),
        "shift" | "lshift" => SHIFT,
        "ctrl" | "control" | "lctrl" => Key::new(0x11, 0x1D, false),
        "alt" | "lalt" => Key::new(0x12, 0x38, false),
        "minus" => Key::new(0xBD, 0x0C, false),
        "grave" | "tilde" => Key::new(0xC0, 0x29, false),
        "pageup" => Key::new(0x21, 0x49, true),
        "pagedown" => Key::new(0x22, 0x51, true),
        "end" => Key::new(0x23, 0x4F, true),
        "home" => Key::new(0x24, 0x47, true),
        "left" => Key::new(0x25, 0x4B, true),
        "up" => Key::new(0x26, 0x48, true),
        "right" => Key::new(0x27, 0x4D, true),
        "down" => Key::new(0x28, 0x50, true),
        "insert" => Key::new(0x2D, 0x52, true),
        "delete" | "del" => Key::new(0x2E, 0x53, true),
        _ => return None,
    })
}

/// One typed character: its key and whether Shift is held for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeStep {
    pub key: Key,
    pub shift: bool,
}

/// The key presses that type `text`. Letters, digits, space, `-` and `_`;
/// anything else is refused rather than typed as something different.
pub fn plan_text(text: &str) -> Result<Vec<TypeStep>, String> {
    text.chars()
        .map(|c| {
            let step = |key, shift| TypeStep { key, shift };
            if let Some(k) = letter(c) {
                return Ok(step(k, c.is_ascii_uppercase()));
            }
            if let Some(k) = digit(c) {
                return Ok(step(k, false));
            }
            match c {
                ' ' => Ok(step(named("space").expect("space"), false)),
                '-' => Ok(step(named("minus").expect("minus"), false)),
                '_' => Ok(step(named("minus").expect("minus"), true)),
                other => Err(format!("cannot type {other:?}")),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_names_map_to_vk_and_scan() {
        assert_eq!(named("W"), Some(Key::new(b'W', 0x11, false)));
        assert_eq!(named("l"), Some(Key::new(b'L', 0x26, false)));
        assert_eq!(named("1"), Some(Key::new(b'1', 0x02, false)));
        assert_eq!(named("0"), Some(Key::new(b'0', 0x0B, false)));
        assert_eq!(named("Enter"), Some(Key::new(0x0D, 0x1C, false)));
        assert_eq!(named("F1"), Some(Key::new(0x70, 0x3B, false)));
        assert_eq!(named("f12"), Some(Key::new(0x7B, 0x58, false)));
        assert_eq!(named("end"), Some(Key::new(0x23, 0x4F, true)));
        assert_eq!(named("bogus"), None);
    }

    /// The live-verified shape: End/Backspace/letters posted with these
    /// lParams cleared and retyped the login field.
    #[test]
    fn lparams_carry_scan_code_extended_and_up_bits() {
        let end = named("End").unwrap();
        assert_eq!(end.lparam_down(), (1 | (0x4F << 16) | (1 << 24)) as isize);
        let a = named("a").unwrap();
        assert_eq!(
            a.lparam_up() as u32,
            1 | (0x1E << 16) | (1 << 30) | (1 << 31)
        );
    }

    #[test]
    fn typing_plan_shifts_capitals_and_refuses_unknown_characters() {
        let plan = plan_text("Ab_1").unwrap();
        assert_eq!(
            plan.iter().map(|s| (s.key.vk, s.shift)).collect::<Vec<_>>(),
            vec![(b'A', true), (b'B', false), (0xBD, true), (b'1', false)]
        );
        assert!(plan_text("a!").unwrap_err().contains('!'));
    }
}
