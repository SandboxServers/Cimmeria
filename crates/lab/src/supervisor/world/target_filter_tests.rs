//! Server-backed target filtering and native-action regression tests.

use super::click::{self, ClickRequest, Expect};
use super::find::{self, FindRequest};
use super::geometry::Vec3;
use super::sim::{Sim, UNITS};
use super::tool_tests::room;

#[tokio::test]
async fn template_and_life_filters_use_server_identity_not_client_mob_id() {
    let mut sim = room();
    sim.server_templates.insert(10, 4);
    sim.server_templates.insert(11, 4);
    sim.dead_ids.push(10);
    let alive = find::run(
        &mut sim,
        FindRequest {
            template_id: Some(4),
            alive: Some(true),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(alive["count"], 1);
    assert_eq!(alive["matches"][0]["id"], 11);
    assert_eq!(alive["matches"][0]["template_id"], 4);
    assert_eq!(alive["matches"][0]["mob_id"], 9011);
    let dead = find::run(
        &mut sim,
        FindRequest {
            template_id: Some(4),
            alive: Some(false),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(dead["matches"][0]["id"], 10);
}

#[tokio::test]
async fn template_only_finds_an_inanimate_prop_with_unknown_life_state() {
    let mut sim = room();
    sim.server_templates.insert(12, 13);
    sim.unknown_alive_ids.push(12);
    let prop = find::run(
        &mut sim,
        FindRequest {
            template_id: Some(13),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(prop["matches"][0]["id"], 12);
    assert_eq!(prop["matches"][0]["template_id"], 13);
    assert!(prop["matches"][0]["alive"].is_null());
    let dead = find::run(
        &mut sim,
        FindRequest {
            template_id: Some(13),
            alive: Some(false),
            ..FindRequest::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(dead["count"], 0, "unknown is not a corpse");
}

#[tokio::test]
async fn template_target_selects_filtered_match_and_still_clicks_natively() {
    let mut sim = Sim::new()
        .with_entity(10, "Guard", Vec3::new(1000.0, 300.0, 0.0))
        .with_entity(11, "Guard", Vec3::new(1200.0, -300.0, 0.0));
    sim.server_templates.insert(10, 24);
    sim.server_templates.insert(11, 24);
    sim.dead_ids.push(10);
    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            template_id: Some(24),
            alive: Some(true),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(sim.slots.get(&UNITS.target).copied().unwrap_or(0), 11);
    assert_eq!(out["native_level"], "real_input");
    assert_eq!(sim.buttons, vec![(0, true), (0, false)]);
}

#[tokio::test]
async fn template_index_selects_second_match_in_stable_id_order() {
    let mut sim = Sim::new()
        .with_entity(10, "Guard", Vec3::new(1200.0, 300.0, 0.0))
        .with_entity(11, "Guard", Vec3::new(1000.0, -300.0, 0.0));
    sim.server_templates.insert(10, 24);
    sim.server_templates.insert(11, 24);
    let out = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            template_id: Some(24),
            nearest: Some(false),
            index: Some(1),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap();
    assert_eq!(sim.slots.get(&UNITS.target).copied().unwrap_or(0), 11);
    assert_eq!(out["native_level"], "real_input");
}

#[tokio::test]
async fn unavailable_server_refuses_template_selection_without_clicking() {
    let mut sim = room();
    sim.server_unavailable = true;
    let err = click::run(
        &mut sim,
        "client_target",
        ClickRequest {
            template_id: Some(4),
            ..ClickRequest::default()
        },
        Some((0, Expect::Target)),
    )
    .await
    .unwrap_err();
    assert_eq!(err.step, "server_entity_query");
    assert!(sim.buttons.is_empty());
}
