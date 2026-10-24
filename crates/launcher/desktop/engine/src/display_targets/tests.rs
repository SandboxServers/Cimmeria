use super::*;

fn display(x: i32, y: i32, width: u32, height: u32) -> DisplayInfo {
    DisplayInfo {
        index: 0,
        main: false,
        x,
        y,
        width,
        height,
        scale_percent: 100,
        refresh_hz: 0,
    }
}

#[test]
fn sizes_for_keeps_the_frame_margin_on_a_laptop_panel() {
    // A 14" MacBook Pro at its default 1710x1107 points.
    let sizes = sizes_for(&display(0, 0, 1710, 1107));
    assert_eq!(
        sizes,
        vec![
            (1024, 768),
            (1280, 720),
            (1280, 800),
            (1440, 900),
            (1600, 900)
        ]
    );
    // 1050 + 80 > 1107: the title bar would fall off the screen.
    assert!(!sizes.contains(&(1680, 1050)));
}

#[test]
fn sizes_for_never_returns_an_empty_list() {
    assert_eq!(sizes_for(&display(0, 0, 800, 600)), vec![(1024, 768)]);
}

#[test]
fn sizes_for_offers_every_size_on_a_large_display() {
    assert_eq!(sizes_for(&display(0, 0, 2560, 1600)), SIZES.to_vec());
}

#[test]
fn setup_key_ignores_list_order_and_changes_when_a_display_moves() {
    let laptop = display(0, 0, 1710, 1107);
    let external = display(1710, 0, 2560, 1440);
    let forward = setup_key(&[laptop, external]);
    assert_eq!(forward, setup_key(&[external, laptop]));
    let moved = display(-2560, 0, 2560, 1440);
    assert_ne!(forward, setup_key(&[laptop, moved]));
    assert_eq!(forward.len(), 16);
    assert!(forward
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
}

#[test]
fn setup_key_ignores_everything_but_the_rectangle() {
    let mut a = display(0, 0, 1710, 1107);
    let b = a;
    a.index = 3;
    a.main = true;
    a.scale_percent = 200;
    a.refresh_hz = 120;
    assert_eq!(setup_key(&[a]), setup_key(&[b]));
}

#[test]
fn fnv1a64_matches_the_published_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn setup_key_is_pinned_to_known_values() {
    // Little-endian (x, y, width, height) per display, sorted; a stored
    // placement's key must never change between launcher builds.
    assert_eq!(setup_key(&[display(0, 0, 1710, 1107)]), "0585e56207bf585a");
    assert_eq!(
        setup_key(&[display(1710, 0, 2560, 1440), display(0, 0, 1710, 1107)]),
        "9ba06192f52f5fa7"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn list_finds_the_displays_of_this_mac_with_exactly_one_main() {
    if super::mac::main_display_asleep() {
        eprintln!("skipped: the main display is asleep, so CoreGraphics lists no active display");
        return;
    }
    let displays = list();
    assert!(!displays.is_empty());
    assert_eq!(displays.iter().filter(|d| d.main).count(), 1);
    for (i, d) in displays.iter().enumerate() {
        assert_eq!(d.index, i as u32);
        assert!(d.width > 0 && d.height > 0);
        assert!(d.scale_percent >= 100);
    }
}

#[test]
fn display_info_and_size_pairs_serialize_in_the_snapshot_wire_shape() {
    let d = DisplayInfo {
        index: 1,
        main: true,
        x: -1710,
        y: 0,
        width: 1710,
        height: 1107,
        scale_percent: 200,
        refresh_hz: 120,
    };
    assert_eq!(
        serde_json::to_value(d).unwrap(),
        serde_json::json!({"index":1,"main":true,"x":-1710,"y":0,"width":1710,"height":1107,
            "scale_percent":200,"refresh_hz":120})
    );
    assert_eq!(
        serde_json::to_value(size_pairs(&display(0, 0, 800, 600))).unwrap(),
        serde_json::json!([[1024, 768]])
    );
}

#[test]
fn scale_percent_from_dpi_maps_the_windows_scale_steps() {
    for (dpi, pct) in [
        (96, 100),
        (120, 125),
        (144, 150),
        (168, 175),
        (192, 200),
        (288, 300),
    ] {
        assert_eq!(scale_percent_from_dpi(dpi), pct, "dpi {dpi}");
    }
    assert_eq!(scale_percent_from_dpi(0), 100);
}

#[test]
fn to_logical_rounds_to_the_nearest_unit_on_both_sides_of_zero() {
    assert_eq!(to_logical(2560, 150), 1707);
    assert_eq!(to_logical(1440, 150), 960);
    assert_eq!(to_logical(-2560, 150), -1707);
    assert_eq!(to_logical(-1920, 150), -1280);
    assert_eq!(to_logical(1920, 125), 1536);
    assert_eq!(to_logical(3840, 200), 1920);
    assert_eq!(to_logical(1920, 100), 1920);
    assert_eq!(to_logical(0, 175), 0);
}

#[test]
fn a_150_percent_monitor_offers_only_the_sizes_the_scaled_game_fits() {
    // 2560x1440 pixels at 150%, right of a 1920-wide primary.
    let (x, y, width, height) = logical_rect(1920, 0, 4480, 1440, 150);
    assert_eq!((x, y, width, height), (1280, 0, 1707, 960));
    let d = DisplayInfo {
        index: 1,
        main: false,
        x,
        y,
        width,
        height,
        scale_percent: 150,
        refresh_hz: 60,
    };
    assert_eq!(sizes_for(&d), vec![(1024, 768), (1280, 720), (1280, 800)]);
    // Revert proof: the raw pixels (scale 100) would offer 1920x1080 and 2560x1440.
    let raw = logical_rect(1920, 0, 4480, 1440, 100);
    assert_eq!(raw, (1920, 0, 2560, 1440));
}

#[test]
fn first_empty_logs_once_per_run_of_empty_lists() {
    let latch = std::sync::atomic::AtomicBool::new(false);
    let seen: Vec<bool> = [false, true, true, true, false, true]
        .into_iter()
        .map(|empty| first_empty(&latch, empty))
        .collect();
    assert_eq!(seen, vec![false, true, false, false, false, true]);
}

#[test]
fn setup_key_of_an_empty_list_is_the_fnv_offset_basis() {
    // PD-32 must never store a placement under this key (ruling 5).
    assert_eq!(setup_key(&[]), "cbf29ce484222325");
}
