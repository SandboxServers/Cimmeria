//! Combat flows against a fake bridge. Only the paths that post no window
//! messages can run here (input needs the game's HWND): the Lua fallback
//! of `client_use_ability`, its refusals, the hotbar read, and
//! `client_die_and_respawn` with `respawn: none | auto`. The key press, the
//! button and Ability-window clicks, `setup_health` typing and the Release
//! click are live-only.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::defeat::{DieRequest, Respawn};
use super::use_ability::resolve::Query;
use super::use_ability::{Fallback, Press, UseAbilityRequest};
use crate::supervisor::events::fake_bridge::{
    self, empty_rings, events, is_ring_pump, lua_ok, Responder,
};

fn chunk(params: &Value) -> &str {
    params["chunk"].as_str().unwrap_or_default()
}

// ---- client_use_ability --------------------------------------------------

/// A client whose hotbar is empty, whose trees know ability 1100, and
/// whose server answers a `useAbility` with a cast and an effect.
fn ability_fake(used: Arc<Mutex<Vec<String>>>) -> Responder {
    Arc::new(move |method, params| {
        let c = chunk(params);
        match method {
            "events_read" => {
                if used.lock().unwrap().is_empty() {
                    Ok(events(vec![]))
                } else {
                    let now = crate::supervisor::now_ms();
                    Ok(events(vec![
                        (
                            "net.out",
                            now,
                            json!({ "method": "useAbility", "entity_id": 1 }),
                        ),
                        (
                            "cme.event",
                            now,
                            json!({ "event": "Event_NetIn_onSequence" }),
                        ),
                        (
                            "cme.event",
                            now,
                            json!({ "event": "Event_NetIn_onEffectResults" }),
                        ),
                        (
                            "cme.event",
                            now,
                            json!({ "event": "Event_NetIn_onTimerUpdate" }),
                        ),
                    ]))
                }
            }
            "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
            "lua_eval" if c.contains("ActionButtonMod.buttons[i]") => {
                Ok(lua_ok(&["profile\t1\t1"]))
            }
            "lua_eval" if c.contains("getTrainingTreeCount") => Ok(lua_ok(&[
                "known\t1100\t1\t1\tPistol Shot",
                "info\t4000\tGM Blast\tfalse\tfalse",
            ])),
            "lua_eval" if c.contains("put(\"world_id\"") => {
                Ok(lua_ok(&["world_id\t701", "health\t100", "target\tGuard"]))
            }
            "lua_eval" if c.contains("useAbility(") => {
                used.lock().unwrap().push(c.to_string());
                Ok(lua_ok(&["ok"]))
            }
            _ => Err(format!("unexpected {method}: {c:.80}")),
        }
    })
}

fn use_req(query: Query, fallback: Fallback, place: bool) -> UseAbilityRequest {
    UseAbilityRequest {
        query,
        press: Press::Auto,
        place,
        fallback,
        observe: Duration::from_millis(2000),
    }
}

#[tokio::test]
async fn lua_fallback_is_labelled_n3_and_reads_the_effect() {
    let used = Arc::new(Mutex::new(Vec::new()));
    let sup = fake_bridge::supervisor(ability_fake(used.clone())).await;
    let out = sup
        .use_ability_flow(use_req(Query::Id(4000), Fallback::Lua, false))
        .await
        .map_err(|e| e.summary())
        .unwrap();
    assert_eq!(out["path"], "lua");
    assert_eq!(out["native_level"]["tier"], "N3");
    assert_eq!(out["ability"]["name"], "GM Blast");
    assert_eq!(out["result"]["verdict"], "effect_applied");
    assert_eq!(out["result"]["sent"], true);
    assert_eq!(out["result"]["cast_started"], true);
    assert_eq!(out["target_before"]["name"], "Guard");
    assert!(used.lock().unwrap()[0].contains("useAbility(4000, Unit.Target)"));
    // Settled early: well under the 2 s observe budget.
    assert!(out["elapsed_ms"].as_u64().unwrap() < 1900, "{out}");
}

#[tokio::test]
async fn an_ability_outside_the_bar_and_trees_needs_place_or_lua() {
    let sup = fake_bridge::supervisor(ability_fake(Arc::default())).await;
    let e = sup
        .use_ability_flow(use_req(Query::Id(4000), Fallback::Window, false))
        .await
        .unwrap_err();
    assert_eq!(e.step, "no_native_path");
    assert!(e.message.contains("place=true"));
    let none = sup
        .use_ability_flow(use_req(Query::Name("pistol".into()), Fallback::None, false))
        .await
        .unwrap_err();
    assert_eq!(none.step, "not_on_hotbar");
    // No visible empty button to place it on (the fake bar lists none).
    let place = sup
        .use_ability_flow(use_req(Query::Id(1100), Fallback::None, true))
        .await
        .unwrap_err();
    assert_eq!(place.step, "place");
}

#[tokio::test]
async fn an_unknown_name_fails_at_resolve_with_the_known_list() {
    let sup = fake_bridge::supervisor(ability_fake(Arc::default())).await;
    let e = sup
        .use_ability_flow(use_req(Query::Name("zat".into()), Fallback::Lua, false))
        .await
        .unwrap_err();
    assert_eq!(e.step, "resolve");
    assert!(e.message.contains("Pistol Shot"));
}

#[tokio::test]
async fn hotbar_tool_reports_the_read_level() {
    let r: Responder = Arc::new(|_, _| {
        Ok(lua_ok(&[
            "profile\t1\t1",
            "button\t1\tActionButtons_1Button\t1\t7\t1\tAbility\t1100\tPistol Shot\t-1\t0\t4\tkey=49\u{1f}vkeyShortText=1\t",
        ]))
    });
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.hotbar(false).await.unwrap();
    assert_eq!(out["native_level"]["tier"], "read");
    assert_eq!(out["buttons"][0]["ability_id"], 1100);
    assert_eq!(out["buttons"][0]["keys"][0]["lab_key"], "1");
}

// ---- client_die_and_respawn ------------------------------------------------

/// A defeat that the fake plays out: the window shows after two polls,
/// hides again after two more (auto release), then the player is alive at
/// the aid station.
fn defeat_fake() -> Responder {
    let polls = Arc::new(Mutex::new((0u32, 0u32)));
    Arc::new(move |method, params| {
        let c = chunk(params);
        let poll = |met: bool| Ok(lua_ok(&[if met { "true" } else { "false" }, "", ""]));
        match method {
            "events_read" => Ok(events(vec![])),
            "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
            "lua_eval" if c.contains("put(\"world_id\"") => {
                let (shown, hidden) = *polls.lock().unwrap();
                Ok(if hidden >= 2 {
                    lua_ok(&[
                        "world_id\t701",
                        "x\t50",
                        "y\t0",
                        "z\t0",
                        "health\t850",
                        "health_max\t850",
                    ])
                } else if shown >= 2 {
                    lua_ok(&[
                        "world_id\t701",
                        "x\t10",
                        "y\t0",
                        "z\t0",
                        "health\t0",
                        "defeat_visible\ttrue",
                    ])
                } else {
                    lua_ok(&[
                        "world_id\t701",
                        "x\t0",
                        "y\t0",
                        "z\t0",
                        "health\t1",
                        "health_max\t850",
                    ])
                })
            }
            "lua_eval" if c.contains("PlayerDefeat_RespawnerList") => Ok(lua_ok(&[
                "respawner\t0\tCellblock Aid Station\t12\ttrue",
                "countdown\tAutomatically respawn in 0:09...",
                "total_s\t9",
            ])),
            "lua_eval" if c.contains("not not (not (") => {
                let mut p = polls.lock().unwrap();
                p.1 += 1;
                poll(p.1 >= 2)
            }
            "lua_eval" if c.contains("PlayerDefeatWin") => {
                let mut p = polls.lock().unwrap();
                p.0 += 1;
                poll(p.0 >= 2)
            }
            "lua_eval" if c.contains("Stat.Health).current > 0") => poll(true),
            _ => Err(format!("unexpected {method}: {c:.80}")),
        }
    })
}

fn die_req(respawn: Respawn) -> DieRequest {
    DieRequest {
        setup_health: None,
        respawn,
        respawner: None,
        defeat_timeout: Duration::from_secs(5),
        respawn_timeout: Duration::from_secs(5),
    }
}

#[tokio::test]
async fn auto_release_is_verified_without_any_input() {
    let sup = fake_bridge::supervisor(defeat_fake()).await;
    let out = sup
        .die_and_respawn_flow(die_req(Respawn::Auto))
        .await
        .map_err(|e| e.summary())
        .unwrap();
    assert_eq!(out["defeated"], true);
    assert_eq!(
        out["defeat_window"]["respawners"][0]["name"],
        "Cellblock Aid Station"
    );
    assert_eq!(out["defeat_window"]["auto_respawn_s"], 9.0);
    assert_eq!(out["respawner"]["id"], 12);
    assert_eq!(out["native_level"]["tier"], "none");
    assert_eq!(out["verified"], true);
    assert_eq!(out["checks"]["alive"], true);
    assert_eq!(out["checks"]["moved_from_death_point_m"], 40.0);
    assert_eq!(out["checks"]["world_changed"], false);
    assert!(out["setup"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn respawn_none_stops_at_the_window() {
    let sup = fake_bridge::supervisor(defeat_fake()).await;
    let out = sup
        .die_and_respawn_flow(die_req(Respawn::None))
        .await
        .map_err(|e| e.summary())
        .unwrap();
    assert_eq!(out["respawned"], false);
    assert_eq!(out["at_death"]["defeat_window"], true);
}

#[tokio::test]
async fn no_defeat_window_is_a_named_failure() {
    let r: Responder = Arc::new(|method, params| {
        let c = chunk(params);
        match method {
            "events_read" => Ok(events(vec![])),
            "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
            "lua_eval" if c.contains("put(\"world_id\"") => Ok(lua_ok(&["health\t0"])),
            "lua_eval" if c.contains("PlayerDefeatWin") => Ok(lua_ok(&["false", "", ""])),
            // The failure's screen summary.
            "lua_eval" => Ok(lua_ok(&["", ""])),
            other => Err(format!("unexpected {other}")),
        }
    });
    let sup = fake_bridge::supervisor(r).await;
    let mut req = die_req(Respawn::Release);
    req.defeat_timeout = Duration::from_millis(600);
    let e = sup.die_and_respawn_flow(req).await.unwrap_err();
    assert_eq!(e.step, "wait_defeat");
    assert!(
        e.message.contains("does not kill by itself"),
        "{}",
        e.message
    );
}

#[tokio::test]
async fn a_negative_setup_health_is_refused_before_typing() {
    let sup = fake_bridge::supervisor(defeat_fake()).await;
    let mut req = die_req(Respawn::None);
    req.setup_health = Some(-1);
    let e = sup.die_and_respawn_flow(req).await.unwrap_err();
    assert_eq!(e.step, "setup");
}
