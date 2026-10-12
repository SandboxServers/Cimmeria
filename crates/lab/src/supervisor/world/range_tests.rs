//! `client_world_click` reports the server's interaction range (#1660 item 9).

use super::click::{self, ClickRequest, Expect};
use super::geometry::Vec3;
use super::sim::Sim;

/// A medic `x` UE3 units straight ahead; the sim only opens its window
/// within 5 m, like the server.
fn medic_at(x: f64) -> Sim {
    let mut s = Sim::new().with_entity(11, "Medic Ogilvie", Vec3::new(x, 0.0, 0.0));
    s.entities[0].window = Some("DialogWin".into());
    s.interact_range_m = Some(5.0);
    s
}

fn window_click() -> ClickRequest {
    ClickRequest {
        entity_id: Some(11),
        expect: Some(Expect::Window),
        settle_ms: Some(300),
        ..ClickRequest::default()
    }
}

#[tokio::test]
async fn a_right_click_from_beyond_the_range_says_why_it_opened_nothing() {
    let mut sim = medic_at(2000.0);
    let e = click::run(&mut sim, "client_world_click", window_click(), None)
        .await
        .unwrap_err();
    assert_eq!(e.step, "observe");
    assert!(e.message.contains("20.0 m away"), "{}", e.message);
    assert!(e.message.contains("5 m interact range"), "{}", e.message);
    let state = e.to_json()["state"].clone();
    assert_eq!(state["range"]["in_interact_range"], false, "{state}");
    assert_eq!(state["range"]["distance_m"], 20.0, "{state}");
}

#[tokio::test]
async fn a_right_click_inside_the_range_reports_the_distance() {
    let mut sim = medic_at(400.0);
    let out = click::run(&mut sim, "client_world_click", window_click(), None)
        .await
        .unwrap();
    assert_eq!(out["result"]["windows_opened"][0], "DialogWin", "{out}");
    assert_eq!(out["result"]["range"]["distance_m"], 4.0, "{out}");
    assert_eq!(out["result"]["range"]["in_interact_range"], true, "{out}");
}
