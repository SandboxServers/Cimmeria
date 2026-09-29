//! Synthetic input the lab queues for the game to read through its own
//! DirectInput devices, and the pure logic that merges it into what a
//! device call returns. No Win32 here, so all of it is tested on the host.
//!
//! Two consumers read the same state, because a game may poll a device
//! either way:
//!
//! - **Immediate** (`GetDeviceState`): the keyboard sees every held key
//!   as `0x80`; the mouse gets the accumulated motion since the last read
//!   and the held buttons.
//! - **Buffered** (`GetDeviceData`): the queued transitions, one
//!   `DIDEVICEOBJECTDATA` per key/button change or motion step.
//!
//! Whichever consumer reads first drains the motion and the transition
//! queue; a device is polled one way in practice.

use std::collections::VecDeque;

/// `DIMOFS_*` offsets into `DIMOUSESTATE(2)`: the `dwOfs` of a buffered
/// mouse event.
pub const DIMOFS_X: u32 = 0;
pub const DIMOFS_Y: u32 = 4;
pub const DIMOFS_Z: u32 = 8;
pub const DIMOFS_BUTTON0: u32 = 12;

/// Which DirectInput device a call is on, from the GUID it was created with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Keyboard,
    Mouse,
}

/// One buffered event: `(dwOfs, dwData)` of a `DIDEVICEOBJECTDATA`.
pub type BufferedEvent = (u32, u32);

/// Queued synthetic input.
#[derive(Debug, Default)]
pub struct InputState {
    held_keys: Vec<u8>,
    key_events: VecDeque<BufferedEvent>,
    mouse_dx: i32,
    mouse_dy: i32,
    mouse_wheel: i32,
    buttons: [bool; 8],
    mouse_events: VecDeque<BufferedEvent>,
    /// Report the game window as the foreground window so the game keeps
    /// reading input while it is in the background.
    pub virtual_focus: bool,
}

impl InputState {
    /// Press or release a key by DirectInput scan code (`DIK_*`).
    pub fn key(&mut self, dik: u8, down: bool) {
        let held = self.held_keys.contains(&dik);
        if down && !held {
            self.held_keys.push(dik);
        } else if !down && held {
            self.held_keys.retain(|&k| k != dik);
        } else {
            return;
        }
        self.key_events
            .push_back((u32::from(dik), if down { 0x80 } else { 0 }));
    }

    /// Move the mouse by a relative amount (DirectInput mice are relative).
    pub fn mouse_move(&mut self, dx: i32, dy: i32) {
        self.mouse_dx = self.mouse_dx.saturating_add(dx);
        self.mouse_dy = self.mouse_dy.saturating_add(dy);
        if dx != 0 {
            self.mouse_events.push_back((DIMOFS_X, dx as u32));
        }
        if dy != 0 {
            self.mouse_events.push_back((DIMOFS_Y, dy as u32));
        }
    }

    /// Scroll the wheel (120 per notch).
    pub fn mouse_wheel(&mut self, delta: i32) {
        self.mouse_wheel = self.mouse_wheel.saturating_add(delta);
        self.mouse_events.push_back((DIMOFS_Z, delta as u32));
    }

    /// Press or release mouse button `index` (0 = left, 1 = right, 2 = middle).
    pub fn mouse_button(&mut self, index: usize, down: bool) {
        let Some(slot) = self.buttons.get_mut(index) else {
            return;
        };
        if *slot == down {
            return;
        }
        *slot = down;
        self.mouse_events
            .push_back((DIMOFS_BUTTON0 + index as u32, if down { 0x80 } else { 0 }));
    }

    /// Release every held key and button (the lab's "hands off").
    pub fn release_all(&mut self) {
        for k in self.held_keys.clone() {
            self.key(k, false);
        }
        for b in 0..self.buttons.len() {
            self.mouse_button(b, false);
        }
    }

    pub fn held_keys(&self) -> &[u8] {
        &self.held_keys
    }

    pub fn held_buttons(&self) -> Vec<usize> {
        (0..self.buttons.len())
            .filter(|&i| self.buttons[i])
            .collect()
    }

    /// Anything to merge into a device read.
    pub fn is_active(&self) -> bool {
        !self.held_keys.is_empty()
            || !self.key_events.is_empty()
            || !self.mouse_events.is_empty()
            || self.buttons.iter().any(|&b| b)
            || self.mouse_dx != 0
            || self.mouse_dy != 0
            || self.mouse_wheel != 0
    }

    /// Merge into a `GetDeviceState` keyboard buffer (256 bytes, `0x80`
    /// = down). The keyboard transition queue is left for a buffered
    /// consumer.
    pub fn overlay_keyboard_state(&self, state: &mut [u8]) {
        for &k in &self.held_keys {
            if let Some(b) = state.get_mut(usize::from(k)) {
                *b = 0x80;
            }
        }
    }

    /// Merge into a `GetDeviceState` mouse buffer: `DIMOUSESTATE` (16
    /// bytes, 4 buttons) or `DIMOUSESTATE2` (20 bytes, 8 buttons). Adds
    /// and consumes the accumulated motion, sets the held buttons, and
    /// drops the buffered mouse events this motion already covered.
    pub fn overlay_mouse_state(&mut self, state: &mut [u8]) {
        if state.len() < 16 {
            return;
        }
        add_i32(state, 0, self.mouse_dx);
        add_i32(state, 4, self.mouse_dy);
        add_i32(state, 8, self.mouse_wheel);
        let n_buttons = (state.len() - 12).min(self.buttons.len());
        for i in 0..n_buttons {
            if self.buttons[i] {
                state[12 + i] = 0x80;
            }
        }
        self.mouse_dx = 0;
        self.mouse_dy = 0;
        self.mouse_wheel = 0;
        self.mouse_events.clear();
    }

    /// Take up to `room` queued events for a buffered read of `kind`.
    /// Mouse motion taken this way is also removed from the immediate
    /// accumulators, so a device read both ways never double-counts.
    pub fn take_buffered(&mut self, kind: DeviceKind, room: usize) -> Vec<BufferedEvent> {
        let queue = match kind {
            DeviceKind::Keyboard => &mut self.key_events,
            DeviceKind::Mouse => &mut self.mouse_events,
        };
        let n = room.min(queue.len());
        let taken: Vec<BufferedEvent> = queue.drain(..n).collect();
        if kind == DeviceKind::Mouse {
            for &(ofs, data) in &taken {
                match ofs {
                    DIMOFS_X => self.mouse_dx = self.mouse_dx.wrapping_sub(data as i32),
                    DIMOFS_Y => self.mouse_dy = self.mouse_dy.wrapping_sub(data as i32),
                    DIMOFS_Z => self.mouse_wheel = self.mouse_wheel.wrapping_sub(data as i32),
                    _ => {}
                }
            }
        }
        taken
    }
}

fn add_i32(buf: &mut [u8], at: usize, delta: i32) {
    let cur = i32::from_le_bytes(buf[at..at + 4].try_into().expect("4 bytes"));
    buf[at..at + 4].copy_from_slice(&cur.wrapping_add(delta).to_le_bytes());
}

/// `GUID_SysKeyboard` {6F1D2B61-D5A0-11CF-BFC7-444553540000} and
/// `GUID_SysMouse` {6F1D2B60-...}, in memory order (`Data1..3` little-endian).
pub const GUID_SYS_KEYBOARD: [u8; 16] = [
    0x61, 0x2B, 0x1D, 0x6F, 0xA0, 0xD5, 0xCF, 0x11, 0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00,
];
pub const GUID_SYS_MOUSE: [u8; 16] = [
    0x60, 0x2B, 0x1D, 0x6F, 0xA0, 0xD5, 0xCF, 0x11, 0xBF, 0xC7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00,
];

/// Which system device a `CreateDevice` GUID names.
pub fn classify_guid(guid: &[u8; 16]) -> Option<DeviceKind> {
    if *guid == GUID_SYS_KEYBOARD {
        Some(DeviceKind::Keyboard)
    } else if *guid == GUID_SYS_MOUSE {
        Some(DeviceKind::Mouse)
    } else {
        None
    }
}

/// `DIDEVCAPS::dwDevType` low byte: `DI8DEVTYPE_MOUSE` / `_KEYBOARD`.
pub fn classify_dev_type(dev_type: u32) -> Option<DeviceKind> {
    match dev_type & 0xFF {
        0x12 => Some(DeviceKind::Mouse),
        0x13 => Some(DeviceKind::Keyboard),
        _ => None,
    }
}

/// `DIK_*` scan code for a key name (`"W"`, `"Enter"`, `"F1"`, `"Space"`,
/// ...), case-insensitive. `None` for an unknown name.
pub fn dik_for(name: &str) -> Option<u8> {
    let n = name.to_ascii_lowercase();
    let letters: [(char, u8); 26] = [
        ('a', 0x1E),
        ('b', 0x30),
        ('c', 0x2E),
        ('d', 0x20),
        ('e', 0x12),
        ('f', 0x21),
        ('g', 0x22),
        ('h', 0x23),
        ('i', 0x17),
        ('j', 0x24),
        ('k', 0x25),
        ('l', 0x26),
        ('m', 0x32),
        ('n', 0x31),
        ('o', 0x18),
        ('p', 0x19),
        ('q', 0x10),
        ('r', 0x13),
        ('s', 0x1F),
        ('t', 0x14),
        ('u', 0x16),
        ('v', 0x2F),
        ('w', 0x11),
        ('x', 0x2D),
        ('y', 0x15),
        ('z', 0x2C),
    ];
    if n.len() == 1 {
        let c = n.chars().next()?;
        if let Some(&(_, code)) = letters.iter().find(|(l, _)| *l == c) {
            return Some(code);
        }
        if let Some(d) = c.to_digit(10) {
            // DIK_1..DIK_9 = 0x02..0x0A, DIK_0 = 0x0B.
            return Some(if d == 0 { 0x0B } else { 0x01 + d as u8 });
        }
    }
    if let Some(f) = n.strip_prefix('f').and_then(|s| s.parse::<u8>().ok()) {
        return match f {
            1..=10 => Some(0x3A + f),
            11 => Some(0x57),
            12 => Some(0x58),
            _ => None,
        };
    }
    Some(match n.as_str() {
        "escape" | "esc" => 0x01,
        "minus" => 0x0C,
        "equals" => 0x0D,
        "backspace" | "back" => 0x0E,
        "tab" => 0x0F,
        "enter" | "return" => 0x1C,
        "lctrl" | "ctrl" | "control" => 0x1D,
        "lshift" | "shift" => 0x2A,
        "rshift" => 0x36,
        "lalt" | "alt" => 0x38,
        "space" => 0x39,
        "capslock" => 0x3A,
        "slash" => 0x35,
        "grave" | "tilde" => 0x29,
        "home" => 0xC7,
        "up" => 0xC8,
        "pageup" => 0xC9,
        "left" => 0xCB,
        "right" => 0xCD,
        "end" => 0xCF,
        "down" => 0xD0,
        "pagedown" => 0xD1,
        "insert" => 0xD2,
        "delete" | "del" => 0xD3,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
