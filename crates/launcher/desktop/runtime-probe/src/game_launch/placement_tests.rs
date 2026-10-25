//! Window placement: the monitor choice, the centring, the unit rule and the
//! `place_on`/`window_placed` wire shape.
use super::*;
use crate::game_launch::{Event, Message, Request};
use uuid::Uuid;

const MAIN: [i32; 4] = [0, 0, 1920, 1080];
const SIDE: [i32; 4] = [1920, 0, 2560, 1440];

#[test]
fn choose_takes_the_exact_rectangle_then_the_same_size() {
    let side_same_size_left = [-2560, 0, 2560, 1440];
    for (name, monitors, target, expected) in [
        ("exact", vec![MAIN, SIDE], SIDE, Some(1)),
        (
            "exact beats size",
            vec![side_same_size_left, SIDE],
            SIDE,
            Some(1),
        ),
        // The side monitor was moved to the left of the main one.
        (
            "moved monitor",
            vec![MAIN, side_same_size_left],
            SIDE,
            Some(1),
        ),
        ("unplugged", vec![MAIN], SIDE, None),
        ("no monitors", vec![], SIDE, None),
    ] {
        assert_eq!(choose(&monitors, target), expected, "{name}");
    }
}

#[test]
fn centre_keeps_the_window_inside_the_work_area() {
    for (name, work, window, expected) in [
        (
            "centred",
            [1920, 0, 2560, 1400],
            [0, 0, 1280, 800],
            (2560, 300),
        ),
        (
            "main",
            [0, 25, 1920, 1055],
            [100, 100, 1024, 768],
            (448, 168),
        ),
        (
            "negative origin",
            [-1920, 0, 1920, 1040],
            [0, 0, 1280, 720],
            (-1600, 160),
        ),
        ("exact fit", [0, 0, 1280, 720], [0, 0, 1280, 720], (0, 0)),
        // Larger than the work area: pinned to its top-left.
        (
            "too large",
            [1920, 30, 1280, 770],
            [0, 0, 1920, 1080],
            (1920, 30),
        ),
        (
            "too wide only",
            [0, 0, 1024, 768],
            [0, 0, 1280, 600],
            (0, 84),
        ),
    ] {
        assert_eq!(centre(work, window), expected, "{name}");
    }
}

#[test]
fn the_unit_rule_matches_the_engine_display_list() {
    for (dpi, pct) in [(96, 100), (120, 125), (144, 150), (168, 175), (192, 200)] {
        assert_eq!(scale_percent_from_dpi(dpi), pct, "dpi {dpi}");
    }
    assert_eq!(scale_percent_from_dpi(0), 100);
    // PD-31's vectors.
    assert_eq!(to_logical(2560, 150), 1707);
    assert_eq!(to_logical(1440, 150), 960);
    assert_eq!(to_logical(-2560, 150), -1707);
    assert_eq!(to_logical(1920, 125), 1536);
    assert_eq!(to_logical(1920, 100), 1920);
    assert_eq!(game_units(SIDE, 150), [1280, 0, 1707, 960]);
    // Under Wine the DPI reads 96: the identity.
    assert_eq!(game_units(SIDE, 100), SIDE);
}

#[test]
fn choose_matches_a_150_percent_monitor_in_game_units() {
    let monitors = [game_units(MAIN, 100), game_units(SIDE, 150)];
    assert_eq!(choose(&monitors, [1280, 0, 1707, 960]), Some(1));
    // Unconverted pixels would not match the engine's rectangle.
    assert_eq!(choose(&[MAIN, SIDE], [1280, 0, 1707, 960]), None);
}

fn request(place_on: Option<[i32; 4]>) -> Request {
    Request {
        schema_version: 1,
        operation_id: Uuid::from_u128(1),
        exe: "C:\\Games\\SGW\\Working\\Binaries\\SGW.exe".into(),
        directory: "C:\\Games\\SGW\\Working\\Binaries".into(),
        dlls: Vec::new(),
        args: Vec::new(),
        place_on,
    }
}

#[test]
fn place_on_is_omitted_when_none_and_old_requests_decode() {
    let json = serde_json::to_value(request(None)).unwrap();
    assert!(json.get("place_on").is_none(), "{json}");
    // A request from a launcher without placement decodes with no target.
    let decoded = Request::decode(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(decoded.place_on, None);
    let mut explicit_null = json.clone();
    explicit_null["place_on"] = serde_json::Value::Null;
    let decoded = Request::decode(&serde_json::to_vec(&explicit_null).unwrap()).unwrap();
    assert_eq!(decoded.place_on, None);

    let json = serde_json::to_value(request(Some([1280, 0, 1707, 960]))).unwrap();
    assert_eq!(json["place_on"], serde_json::json!([1280, 0, 1707, 960]));
    let decoded = Request::decode(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(decoded.place_on, Some([1280, 0, 1707, 960]));
}

#[test]
fn window_placed_round_trips() {
    let id = Uuid::from_u128(1);
    for outcome in [
        PlacementOutcome::Placed,
        PlacementOutcome::AlreadyThere,
        PlacementOutcome::NoWindow,
        PlacementOutcome::NoMonitor,
    ] {
        let message = Message {
            schema_version: 1,
            operation_id: id,
            observation: Event::WindowPlaced { outcome },
        };
        let bytes = serde_json::to_vec(&message).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["observation"]["event"], "window_placed");
        assert_eq!(json["observation"]["outcome"], outcome.code());
        assert_eq!(
            Message::decode(&bytes, id).unwrap(),
            Event::WindowPlaced { outcome }
        );
    }
}
