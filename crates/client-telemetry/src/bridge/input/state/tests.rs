use super::*;

#[test]
fn held_keys_overlay_the_immediate_keyboard_state() {
    let mut s = InputState::default();
    s.key(0x11, true); // W
    let mut kb = [0u8; 256];
    s.overlay_keyboard_state(&mut kb);
    assert_eq!(kb[0x11], 0x80);
    s.key(0x11, false);
    let mut kb = [0u8; 256];
    s.overlay_keyboard_state(&mut kb);
    assert_eq!(kb[0x11], 0);
}

/// A repeat press is not a transition: the buffered consumer must see one
/// down and one up, or a held key would type twice.
#[test]
fn key_transitions_are_queued_once_per_change() {
    let mut s = InputState::default();
    s.key(0x1C, true);
    s.key(0x1C, true);
    s.key(0x1C, false);
    s.key(0x1C, false);
    assert_eq!(
        s.take_buffered(DeviceKind::Keyboard, 16),
        vec![(0x1C, 0x80), (0x1C, 0)]
    );
    assert!(!s.is_active());
}

#[test]
fn mouse_state_gets_the_motion_once_and_the_held_buttons() {
    let mut s = InputState::default();
    s.mouse_move(10, -5);
    s.mouse_move(3, 0);
    s.mouse_button(0, true);
    let mut ms = [0u8; 16];
    ms[0..4].copy_from_slice(&7i32.to_le_bytes()); // real hardware moved 7
    s.overlay_mouse_state(&mut ms);
    assert_eq!(i32::from_le_bytes(ms[0..4].try_into().unwrap()), 20);
    assert_eq!(i32::from_le_bytes(ms[4..8].try_into().unwrap()), -5);
    assert_eq!(ms[12], 0x80);
    // Motion is consumed; the held button stays.
    let mut ms = [0u8; 16];
    s.overlay_mouse_state(&mut ms);
    assert_eq!(i32::from_le_bytes(ms[0..4].try_into().unwrap()), 0);
    assert_eq!(ms[12], 0x80);
}

/// A mouse read through the buffer must not also leave the same motion in
/// the immediate accumulator (double movement).
#[test]
fn buffered_mouse_reads_consume_the_immediate_motion_too() {
    let mut s = InputState::default();
    s.mouse_move(10, 4);
    s.mouse_button(1, true);
    let ev = s.take_buffered(DeviceKind::Mouse, 8);
    assert_eq!(
        ev,
        vec![(DIMOFS_X, 10), (DIMOFS_Y, 4), (DIMOFS_BUTTON0 + 1, 0x80)]
    );
    let mut ms = [0u8; 20];
    s.overlay_mouse_state(&mut ms);
    assert_eq!(i32::from_le_bytes(ms[0..4].try_into().unwrap()), 0);
    assert_eq!(ms[13], 0x80);
}

#[test]
fn take_buffered_respects_the_callers_room() {
    let mut s = InputState::default();
    for k in [0x10u8, 0x11, 0x12] {
        s.key(k, true);
    }
    assert_eq!(s.take_buffered(DeviceKind::Keyboard, 2).len(), 2);
    assert_eq!(s.take_buffered(DeviceKind::Keyboard, 2), vec![(0x12, 0x80)]);
}

#[test]
fn release_all_lets_go_of_everything() {
    let mut s = InputState::default();
    s.key(0x11, true);
    s.mouse_button(0, true);
    s.release_all();
    assert!(s.held_keys().is_empty());
    assert!(s.held_buttons().is_empty());
}

/// The GUIDs as `CreateDevice` receives them: `Data1` is little-endian,
/// so the keyboard starts `61 2B 1D 6F` (a first draft had the bytes of
/// `Data1` reversed and never matched a real device).
#[test]
fn guids_classify_keyboard_and_mouse() {
    assert_eq!(
        classify_guid(&GUID_SYS_KEYBOARD),
        Some(DeviceKind::Keyboard)
    );
    assert_eq!(classify_guid(&GUID_SYS_MOUSE), Some(DeviceKind::Mouse));
    assert_eq!(&GUID_SYS_KEYBOARD[..4], &0x6F1D_2B61u32.to_le_bytes());
    let mut other = GUID_SYS_KEYBOARD;
    other[15] = 1;
    assert_eq!(classify_guid(&other), None);
}

#[test]
fn dev_types_classify_keyboard_and_mouse() {
    assert_eq!(classify_dev_type(0x0000_0113), Some(DeviceKind::Keyboard));
    assert_eq!(classify_dev_type(0x0000_0212), Some(DeviceKind::Mouse));
    assert_eq!(classify_dev_type(0x0001_0114), None); // joystick
}

#[test]
fn key_names_map_to_dik_codes() {
    assert_eq!(dik_for("W"), Some(0x11));
    assert_eq!(dik_for("a"), Some(0x1E));
    assert_eq!(dik_for("1"), Some(0x02));
    assert_eq!(dik_for("0"), Some(0x0B));
    assert_eq!(dik_for("F1"), Some(0x3B));
    assert_eq!(dik_for("f12"), Some(0x58));
    assert_eq!(dik_for("Enter"), Some(0x1C));
    assert_eq!(dik_for("space"), Some(0x39));
    assert_eq!(dik_for("nope"), None);
}
