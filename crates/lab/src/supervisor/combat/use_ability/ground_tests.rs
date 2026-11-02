//! Ground-targeted abilities: the press opens the reticle, a left click on
//! the ground makes the client send `useAbilityOnGroundTarget`. The fake
//! client below sends nothing until the click arrives, as the live client
//! does (canary.md 2026-11-02), so dropping the click fails these tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use super::{Fallback, Press, UseAbilityRequest};
use crate::supervisor::combat::use_ability::resolve::Query;
use crate::supervisor::events::fake_bridge::{
    self, empty_rings, events, is_ring_pump, lua_ok, Responder,
};
use crate::supervisor::world::{PointArg, Space};

/// What the fake client saw: the press (Lua `useAbility`) and the click.
#[derive(Debug, Default)]
struct Seen {
    pressed: bool,
    clicked: Option<PointArg>,
}

fn ground_fake(seen: Arc<Mutex<Seen>>) -> Responder {
    Arc::new(move |method, params| {
        let c = params["chunk"].as_str().unwrap_or_default();
        match method {
            "events_read" => {
                let s = seen.lock().unwrap();
                if s.pressed && s.clicked.is_some() {
                    let now = crate::supervisor::now_ms();
                    Ok(events(vec![
                        (
                            "net.out",
                            now,
                            json!({ "method": "useAbilityOnGroundTarget", "entity_id": 1 }),
                        ),
                        (
                            "cme.event",
                            now,
                            json!({ "event": "Event_NetIn_onSequence" }),
                        ),
                        (
                            "cme.event",
                            now,
                            json!({ "event": "Event_NetIn_onEffectResults", "ability_id": 4000 }),
                        ),
                    ]))
                } else {
                    Ok(events(vec![]))
                }
            }
            "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
            "lua_eval" if c.contains("ActionButtonMod.buttons[i]") => {
                Ok(lua_ok(&["profile\t1\t1"]))
            }
            "lua_eval" if c.contains("getTrainingTreeCount") => {
                Ok(lua_ok(&["info\t4000\tGround Blast\tfalse\tfalse"]))
            }
            "lua_eval" if c.contains("put(\"world_id\"") => {
                Ok(lua_ok(&["world_id\t1300", "health\t100"]))
            }
            "lua_eval" if c.contains("useAbility(") => {
                seen.lock().unwrap().pressed = true;
                Ok(lua_ok(&["ok"]))
            }
            _ => Err(format!("unexpected {method}: {c:.80}")),
        }
    })
}

fn plaza() -> PointArg {
    PointArg {
        x: -87000.0,
        y: 24600.0,
        z: 663.0,
        space: Some(Space::Client),
    }
}

fn req(ground: Option<PointArg>) -> UseAbilityRequest {
    UseAbilityRequest {
        query: Query::Id(4000),
        press: Press::Auto,
        place: false,
        fallback: Fallback::Lua,
        observe: Duration::from_millis(1500),
        ground,
    }
}

#[tokio::test]
async fn a_ground_point_is_clicked_after_the_press_and_the_send_follows() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let sup = fake_bridge::supervisor(ground_fake(seen.clone())).await;
    let clicks = seen.clone();
    let out = sup
        .use_ability_flow_with(req(Some(plaza())), move |p| async move {
            let mut s = clicks.lock().unwrap();
            assert!(s.pressed, "the click comes after the press");
            s.clicked = Some(p);
            Ok(json!({ "native_level": "real_input" }))
        })
        .await
        .map_err(|e| e.summary())
        .unwrap();
    assert_eq!(seen.lock().unwrap().clicked, Some(plaza()));
    assert_eq!(out["result"]["sent"], true, "{out}");
    assert_eq!(
        out["result"]["net_out"][0]["method"],
        "useAbilityOnGroundTarget"
    );
    assert_eq!(out["ground"]["point"]["space"], "client");
    assert_eq!(out["ground"]["point"]["x"], -87000.0);
    let steps: Vec<&str> = out["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["step"].as_str())
        .collect();
    let at = |name: &str| steps.iter().position(|s| *s == name);
    assert!(at("use_ability_lua") < at("ground_confirm"), "{steps:?}");
    assert!(at("ground_confirm") < at("observe"), "{steps:?}");
}

#[tokio::test]
async fn without_a_ground_point_the_press_is_ignored() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let sup = fake_bridge::supervisor(ground_fake(seen.clone())).await;
    let out = sup
        .use_ability_flow_with(req(None), |_| async {
            panic!("no ground point, no click");
            #[allow(unreachable_code)]
            Ok(json!({}))
        })
        .await
        .map_err(|e| e.summary())
        .unwrap();
    assert_eq!(out["result"]["verdict"], "nothing_observed", "{out}");
    assert_eq!(out["result"]["sent"], false);
    assert!(out["ground"].is_null());
}

#[tokio::test]
async fn a_failed_ground_click_is_a_named_flow_failure() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let sup = fake_bridge::supervisor(ground_fake(seen)).await;
    let e = sup
        .use_ability_flow_with(req(Some(plaza())), |_| async {
            Err("on_screen: off screen".to_string())
        })
        .await
        .unwrap_err();
    assert_eq!(e.step, "ground_confirm");
    assert!(e.message.contains("off screen"), "{}", e.message);
}
