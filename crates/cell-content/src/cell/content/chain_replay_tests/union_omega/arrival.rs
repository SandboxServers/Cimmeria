//! UO-04: the beam into the Command Center (63801) and Modi (63810, 63811).
//!
//! Resolve-only for the three chains, plus seed-shape guards over Omega's
//! whole id block. F21 is the load-bearing rule here: no Omega chain completes
//! an objective of 1655: S4940 has required O5647 and O5644 beside optional
//! O5645 and O5646, and completing the required pair would end 1655 before the
//! Accept. Each bind runs `remove_dialog_set` before `add_dialog_set`, and only
//! the hub services (63850-63869) key on `interact_tag`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;

use super::{
    ctx_for, fire_dialog_choice, fire_player_loaded, fire_teleport_in, labels, with_mission,
    with_objective, with_step, ARCH_HUMAN, ARCH_JAFFA, CMD_NAME, DLG_MODI, HUB_NAME, M_1655,
    O_5645, REGION_CMD_PAD, S_4940, S_4942, WORLD_CMD, WORLD_HUB,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

/// Just landed from the SGC: 1655 active at S4940, Modi's objective open.
fn arrived(archetype: i64, world: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, S_4940, "active");
    with_objective(ctx, M_1655, O_5645, "active")
}

/// Past the beam: S4940 done, S4942 active.
fn beamed(archetype: i64, world: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, S_4940, "completed");
    with_step(ctx, M_1655, S_4942, "active")
}

/// The rendered labels from one chain id, so a test reads one chain at a time.
fn only(chain: i64, all: Vec<String>) -> Vec<String> {
    let prefix = format!("{chain}:");
    all.into_iter().filter(|l| l.starts_with(&prefix)).collect()
}

/// **Hub entry at S4940 binds Modi, once per fire.** Chain 63810 removes and
/// then adds Modi's bind (map 7015, slot 65). Past S4940, or in the Command
/// Center, the chain resolves nothing.
#[tokio::test]
async fn live_db_entering_the_hub_at_4940_binds_modi_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_HUMAN, ARCH_JAFFA] {
        let ctx = arrived(archetype, WORLD_HUB);
        assert_eq!(
            labels(&fire_player_loaded(&engine, &ctx, HUB_NAME)),
            vec![
                "63810:remove_dialog_set(7015@65)".to_string(),
                "63810:add_dialog_set(7015@65)".to_string(),
            ],
            "archetype {archetype} binds Modi on hub entry at S4940"
        );
        let past = beamed(archetype, WORLD_HUB);
        assert!(
            only(63810, labels(&fire_player_loaded(&engine, &past, HUB_NAME))).is_empty(),
            "archetype {archetype} past S4940 binds nothing"
        );
        let cmd = arrived(archetype, WORLD_CMD);
        assert!(
            labels(&fire_player_loaded(&engine, &cmd, CMD_NAME)).is_empty(),
            "archetype {archetype} in the Command Center resolves no Omega chain"
        );
    }
}

/// **Closing Modi's dialog clears his bind and completes nothing (F21).**
/// Chain 63811 removes the bind; no action names 1655 as a mission to
/// complete, to advance or to complete an objective of.
#[tokio::test]
async fn live_db_closing_modi_clears_his_bind_and_completes_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_HUMAN, ARCH_JAFFA] {
        let ctx = arrived(archetype, WORLD_HUB);
        let resolved = fire_dialog_choice(&engine, &ctx, DLG_MODI);
        assert_eq!(
            labels(&resolved),
            vec!["63811:remove_dialog_set(7015@65)".to_string()],
            "archetype {archetype} closing Modi's dialog clears his bind"
        );
        let completes_1655 = resolved.actions.iter().any(|(_, a)| {
            matches!(
                a,
                Action::CompleteObjective { mission_id, .. }
                    | Action::CompleteMission { mission_id }
                    | Action::AdvanceStep { mission_id, .. }
                    if *mission_id == M_1655
            )
        });
        assert!(
            !completes_1655,
            "archetype {archetype}: Modi's dialog must complete or advance nothing on 1655"
        );
        let past = beamed(archetype, WORLD_HUB);
        assert!(
            only(63811, labels(&fire_dialog_choice(&engine, &past, DLG_MODI))).is_empty(),
            "archetype {archetype} past S4940 resolves no 63811"
        );
        let cmd = arrived(archetype, WORLD_CMD);
        assert!(
            only(63811, labels(&fire_dialog_choice(&engine, &cmd, DLG_MODI))).is_empty(),
            "archetype {archetype} in the Command Center resolves no 63811"
        );
    }
}

/// **The ring beam advances 1655 from S4940 to S4942 in the Command Center.**
/// Chain 63801 clears Modi's bind if it is still there, then advances the
/// step (D-UO3). Another pad, or the hub, resolves nothing from 63801.
#[tokio::test]
async fn live_db_the_beam_advances_1655_to_4942_in_the_command_center() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_HUMAN, ARCH_JAFFA] {
        let cmd = arrived(archetype, WORLD_CMD);
        assert_eq!(
            only(
                63801,
                labels(&fire_teleport_in(&engine, &cmd, REGION_CMD_PAD))
            ),
            vec![
                "63801:remove_dialog_set(7015@65)".to_string(),
                "63801:advance_step(1655->4942)".to_string(),
            ],
            "archetype {archetype} beams to S4942"
        );
        let past = beamed(archetype, WORLD_CMD);
        assert!(
            only(
                63801,
                labels(&fire_teleport_in(&engine, &past, REGION_CMD_PAD))
            )
            .is_empty(),
            "archetype {archetype} past S4940 resolves no 63801"
        );
        let hub = arrived(archetype, WORLD_HUB);
        assert!(
            only(
                63801,
                labels(&fire_teleport_in(&engine, &hub, REGION_CMD_PAD))
            )
            .is_empty(),
            "archetype {archetype} beaming in on the hub resolves no 63801"
        );
        assert!(
            only(63801, labels(&fire_teleport_in(&engine, &cmd, 14))).is_empty(),
            "archetype {archetype} on pad 14 resolves no 63801"
        );
    }
}

/// **No Omega chain completes an objective of 1655 (F21, seed guard).** Type 2
/// guard over the whole Omega block, so UO-07 to UO-09 are covered too.
#[tokio::test]
async fn live_db_no_omega_chain_completes_a_1655_objective() {
    let pool = require_db_or_skip!();
    let rows: Vec<i32> = sqlx::query_scalar(
        "SELECT chain_id FROM resources.content_actions \
         WHERE chain_id BETWEEN 63800 AND 63999 \
           AND action_type::text = 'complete_objective' AND target_id = 1655",
    )
    .fetch_all(&pool)
    .await
    .expect("content_actions");
    assert!(
        rows.is_empty(),
        "Omega chains completing 1655 objectives: {rows:?}"
    );
}

/// **Every Omega bind removes before it adds (seed guard).** `add_dialog_set`
/// has no dedupe, so each add needs an earlier remove of the same slot on the
/// same map. The count of Omega adds keeps this from passing on an empty set.
#[tokio::test]
async fn live_db_every_omega_bind_removes_before_it_adds() {
    let pool = require_db_or_skip!();
    let unpaired: Vec<i32> = sqlx::query_scalar(
        "SELECT a.chain_id FROM resources.content_actions a \
         WHERE a.chain_id BETWEEN 63800 AND 63999 \
           AND a.action_type::text = 'add_dialog_set' \
           AND NOT EXISTS ( \
             SELECT 1 FROM resources.content_actions r \
             WHERE r.chain_id = a.chain_id \
               AND r.action_type::text = 'remove_dialog_set' \
               AND r.target_id = a.target_id \
               AND r.params->>'slot' = a.params->>'slot' \
               AND r.sort_order < a.sort_order)",
    )
    .fetch_all(&pool)
    .await
    .expect("content_actions");
    assert!(
        unpaired.is_empty(),
        "adds with no earlier remove: {unpaired:?}"
    );
    let adds: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_actions \
         WHERE chain_id BETWEEN 63800 AND 63999 AND action_type::text = 'add_dialog_set'",
    )
    .fetch_one(&pool)
    .await
    .expect("content_actions count");
    assert!(adds >= 1, "Omega binds at least one dialog set");
}

/// Service tags of the hub services (chains 63850-63869, `interact_tag`). Each
/// names spawns of a template that carries its interaction bit as a default.
/// Each crafting guide has its own tag (`spawnlist.tag` is unique), so guide
/// chains 63852-63855 carry two triggers each, plain and Adv. (UO-35).
const SERVICE_TAGS: [&str; 12] = [
    "UnionOmega_Auctioneer",
    "UnionOmega_Medic",
    "UnionOmega_Guide_Biology",
    "UnionOmega_Guide_AdvBiology",
    "UnionOmega_Guide_Electronics",
    "UnionOmega_Guide_AdvElectronics",
    "UnionOmega_Guide_Engineering",
    "UnionOmega_Guide_AdvEngineering",
    "UnionOmega_Guide_Physics",
    "UnionOmega_Guide_AdvPhysics",
    "UnionOmega_Inn",
    "UnionOmega_Battlemaster",
];

/// **Omega's `interact_tag` chains are hub services only (F4, seed guard).** An
/// `interact_tag` trigger short-circuits `handle_interact` and would open a second
/// dialog, so only the hub services (63850-63869) may key on it. They carry no
/// dialog set: `live_db_omega_service_tags_name_npcs_with_no_dialog` proves it.
#[tokio::test]
async fn live_db_omega_interact_tag_chains_are_services_only() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, event_key FROM resources.content_triggers \
         WHERE chain_id BETWEEN 63800 AND 63999 AND event_type::text = 'interact_tag'",
    )
    .fetch_all(&pool)
    .await
    .expect("content_triggers");
    for (chain_id, tag) in &rows {
        assert!(
            (63850..=63869).contains(chain_id),
            "Omega interact_tag outside the hub services: chain {chain_id}"
        );
        let tag = tag.as_deref().unwrap_or_default();
        assert!(
            SERVICE_TAGS.contains(&tag),
            "Omega interact_tag on a non-service tag {tag:?} (chain {chain_id})"
        );
    }
}

/// **Each service tag names NPCs with no dialog set (F4, seed guard).** A spawn
/// that carries a service tag must use a template with no static interaction set
/// and no interaction set, and no `add_dialog_set` may bind its template id. With
/// no service chain seeded yet, the trigger query returns no rows and this passes.
#[tokio::test]
async fn live_db_omega_service_tags_name_npcs_with_no_dialog() {
    let pool = require_db_or_skip!();
    let triggers: Vec<(Option<String>,)> = sqlx::query_as(
        "SELECT DISTINCT event_key FROM resources.content_triggers \
         WHERE chain_id BETWEEN 63800 AND 63999 AND event_type::text = 'interact_tag'",
    )
    .fetch_all(&pool)
    .await
    .expect("content_triggers");
    for (tag,) in triggers {
        let tag = tag.unwrap_or_default();
        let spawns: Vec<(i32, i32, bool, bool)> = sqlx::query_as(
            "SELECT s.spawn_id, t.template_id, t.static_interaction_sets = '{}' AS no_static, \
                    t.interaction_set_id IS NULL AS no_set \
             FROM resources.spawnlist s \
             JOIN resources.entity_templates t ON t.template_id = s.template_id \
             WHERE s.tag = $1",
        )
        .bind(&tag)
        .fetch_all(&pool)
        .await
        .expect("spawnlist join");
        assert!(!spawns.is_empty(), "no spawn carries service tag {tag}");
        for &(spawn_id, template_id, no_static, no_set) in &spawns {
            assert!(
                no_static && no_set,
                "spawn {spawn_id} of {tag} (template {template_id}) has a dialog set"
            );
            let adds: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM resources.content_actions \
                 WHERE action_type::text = 'add_dialog_set' AND (params->>'slot')::int = $1",
            )
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .expect("content_actions count");
            assert_eq!(
                adds, 0,
                "add_dialog_set binds template {template_id} of {tag}"
            );
        }
    }
}
