//! Ability training: cell-side validation guards before the base-side
//! `training_points` debit + DB persist.

use crate::ability_tree::{evaluate_train, TrainContext, TrainPlan, TrainReject};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Train an ability — full validation + base-side persistence + debit.
///
/// Cell-side validation is [`evaluate_train`], the same predicate the
/// trainer window uses for its `trainable` byte, so a node the window
/// enables is a node this handler forwards. Its gates, in order:
/// 1. Ability id exists in `space_mgr.ability_defs` (rejects typos)
/// 2. Player has a `player_id` (no orphan-entity grants)
/// 3. Player not already known the ability (no-op duplicate)
/// 4. Ability is in player's archetype tree (no cross-class training)
/// 5. Player level meets the tree entry's `level` requirement
/// 6. Every prerequisite is in `entity.abilities`
/// 7. Archetype-wide `tree_points_spent` meets `required_branch_points`
/// 8. `training_points` covers the node's `skill_point_cost`
///
/// On all checks passing, sends `CellToBaseMsg::TrainAbility` (with the
/// node's cost and branch) to the base. The base does the cost debit,
/// the spend increment + DB UPDATE +
/// responds with `BaseToCellMsg::AbilityGranted`, which the cell's
/// dispatcher handles (in `service/base_messages/mod.rs`) by adding
/// the ability to `entity.abilities` and sending
/// `onKnownAbilitiesUpdate` for the hotbar refresh.
///
/// For archetypes without a tree (Scientist/Asgard/Goa'uld/etc. — see
/// Phase 7 content gap), step 4 fails immediately. This is the
/// correct behavior: training is gated on content existing, and the
/// player can still get starter abilities via `char_creation_abilities`
/// (Phase 2).
pub(super) async fn handle_train_ability(
    entity_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (verdict, player_id, archetype_id, player_level) = {
        let entity = match space_mgr.get_entity(entity_id) {
            Some(e) => e,
            None => return,
        };
        let ctx = TrainContext {
            catalog: &space_mgr.ability_tree_catalog,
            ability_id,
            ability_exists: space_mgr.ability_defs.contains_key(&ability_id),
            player_id: entity.player_id,
            archetype_id: entity.archetype_id,
            level: entity.level as i32,
            known: &entity.abilities,
            tree_points_spent: entity.tree_progress.tree_points_spent,
            training_points: entity.tree_progress.training_points,
        };
        (
            evaluate_train(&ctx),
            entity.player_id,
            entity.archetype_id,
            entity.level as i32,
        )
    };

    let plan = match verdict {
        Ok(plan) => plan,
        Err(reject) => {
            log_rejection(
                &reject,
                entity_id,
                player_id,
                archetype_id,
                player_level,
                ability_id,
            );
            return;
        }
    };
    let player_id = plan.player_id;

    // All validation passed. Hand off to base for the training_points
    // debit + DB persist. The cell-side state change happens when base
    // responds with AbilityGranted (see service/base_messages/mod.rs).
    tracing::info!(
        target: "abilities",
        event = "train_requested",
        entity_id,
        player_id,
        ability_id,
        archetype_id = plan.archetype_id,
        tree_index = plan.tree_index,
        cost = plan.cost,
        "trainAbility: validation passed, requesting base persist + debit"
    );
    warn_if_raw_cost_zero(entity_id, player_id, &plan);
    if let Err(e) = tx
        .send(CellToBaseMsg::TrainAbility {
            entity_id,
            player_id,
            ability_id,
            cost: plan.cost,
            tree_index: plan.tree_index,
        })
        .await
    {
        // Cell→base channel closed mid-train. The player saw their
        // request validated client-side (training UI accepted the click)
        // but the persistence step never ran. Log at error so operators
        // can correlate against the "Train clicked but TP didn't drop"
        // player report and investigate the channel-shutdown root cause.
        tracing::error!(
            target: "abilities",
            event = "train_ability_send_failed",
            entity_id,
            player_id,
            ability_id,
            error = %e,
            "TrainAbility cell→base send failed — training point not debited"
        );
    }
}

/// Flag a purchase whose ability has no authored `training_cost`.
///
/// 59 of the 439 FINAL v2 nodes have a raw cost of 0 in
/// `resources.abilities`. The debit uses the tree's `skill_point_cost`, so
/// the purchase goes ahead; the WARN lets UAT see which nodes still carry
/// an unsourced cost. The source value is never rewritten.
fn warn_if_raw_cost_zero(entity_id: u32, player_id: i32, plan: &TrainPlan) {
    if plan.raw_training_cost == 0 {
        tracing::warn!(
            target: "abilities",
            event = "train_raw_cost_zero",
            entity_id,
            player_id,
            ability_id = plan.ability_id,
            archetype_id = plan.archetype_id,
            tree_index = plan.tree_index,
            cost = plan.cost,
            "trainAbility: purchased node has raw training_cost 0 — debiting skill_point_cost"
        );
    }
}

/// The log line each rejection has always produced. Levels, targets and
/// `reason=` values are unchanged from the pre-predicate handler; operators
/// and log-based tests key on them.
fn log_rejection(
    reject: &TrainReject,
    entity_id: u32,
    player_id: Option<i32>,
    archetype_id: Option<i32>,
    player_level: i32,
    ability_id: i32,
) {
    // Only reachable after the player-id gate passed, so the fallback is
    // never logged.
    let pid = player_id.unwrap_or_default();
    match reject {
        TrainReject::UnknownAbility => tracing::warn!(
            entity_id,
            ability_id,
            "trainAbility: ability_id not found in ability_defs — rejecting"
        ),
        TrainReject::NoPlayerId => tracing::warn!(
            entity_id,
            ability_id,
            "trainAbility: entity has no player_id — rejecting"
        ),
        // Replayed packet or UI double-click: a silent no-op.
        TrainReject::AlreadyKnown => tracing::debug!(
            entity_id,
            player_id = pid,
            ability_id,
            "trainAbility: ability already known — no-op"
        ),
        TrainReject::NoArchetype => tracing::warn!(
            entity_id,
            player_id = pid,
            ability_id,
            "trainAbility: entity has no archetype_id — rejecting"
        ),
        TrainReject::NotInArchetypeTree => tracing::info!(
            target: "abilities",
            event = "train_rejected",
            reason = reject.reason(),
            entity_id,
            player_id = pid,
            archetype_id = archetype_id.unwrap_or_default(),
            ability_id,
            "trainAbility: ability not in player's archetype tree — rejecting \
             (likely an unsupported archetype until Phase 7 content lands)"
        ),
        TrainReject::LevelTooLow { required, .. } => tracing::info!(
            target: "abilities",
            event = "train_rejected",
            reason = reject.reason(),
            entity_id,
            player_id = pid,
            ability_id,
            player_level,
            required_level = *required,
            "trainAbility: player level below required — rejecting"
        ),
        TrainReject::MissingPrerequisite { missing } => tracing::info!(
            target: "abilities",
            event = "train_rejected",
            reason = reject.reason(),
            entity_id,
            player_id = pid,
            ability_id,
            missing_prereq = *missing,
            "trainAbility: prerequisite ability not known — rejecting"
        ),
        TrainReject::SpendGate { required, spent } => tracing::info!(
            target: "abilities",
            event = "train_rejected",
            reason = reject.reason(),
            entity_id,
            player_id = pid,
            ability_id,
            required_branch_points = *required,
            tree_points_spent = *spent,
            "trainAbility: archetype-wide spend below the node's gate — rejecting"
        ),
        TrainReject::NotEnoughPoints { cost, available } => tracing::info!(
            target: "abilities",
            event = "train_rejected",
            reason = reject.reason(),
            entity_id,
            player_id = pid,
            ability_id,
            cost = *cost,
            training_points = *available,
            "trainAbility: not enough training points — rejecting"
        ),
    }
}

#[cfg(test)]
mod handle_train_ability_tests {
    //! Validation guards on `handle_train_ability` (cell side). The
    //! base-side persist + debit is exercised via the live-DB path; these
    //! unit tests pin the rejection cases so a refactor can't silently
    //! drop a guard.
    use super::*;
    use crate::ability_tree::TreeNode;
    use crate::cell::messages::CellToBaseMsg;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::abilities::AbilityDef;
    use tokio::sync::mpsc;

    const TEST_ABILITY: i32 = 12345;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-50" MaxX="50" MinY="-50" MaxY="50" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        mgr
    }

    fn seed_ability(mgr: &mut SpaceManager, ability_id: i32) {
        mgr.ability_defs.insert(
            ability_id,
            AbilityDef {
                ability_id,
                name: "TestAbility".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
    }

    fn seed_tree(mgr: &mut SpaceManager, node: TreeNode) {
        mgr.ability_tree_catalog.push(node);
    }

    fn drain_train_msgs(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<i32> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::TrainAbility { ability_id, .. } = msg {
                out.push(ability_id);
            }
        }
        out
    }

    #[tokio::test]
    async fn rejects_unknown_ability_id() {
        // No seeded ability_def → step 1 fails.
        let mut mgr = make_mgr();
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, 99999, &tx, &mut mgr).await;
        assert!(
            drain_train_msgs(&mut rx).is_empty(),
            "must not send TrainAbility"
        );
    }

    #[tokio::test]
    async fn rejects_entity_without_player_id() {
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        // Entity 1 has no player_id by default.
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        assert!(
            drain_train_msgs(&mut rx).is_empty(),
            "no player_id = no train"
        );
    }

    #[tokio::test]
    async fn already_known_ability_is_silent_noop() {
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
            e.archetype_id = Some(1); // Soldier
            e.level = 1;
            e.abilities.add_ability(TEST_ABILITY); // already known
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        assert!(
            drain_train_msgs(&mut rx).is_empty(),
            "duplicate train is no-op"
        );
    }

    #[tokio::test]
    async fn rejects_ability_not_in_archetype_tree() {
        // Player is a Soldier; ability is in a tree but only for Commando.
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        seed_tree(
            &mut mgr,
            TreeNode::with_defaults(2, 0, TEST_ABILITY, 1, vec![]),
        );
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
            e.archetype_id = Some(1); // Soldier, NOT Commando
            e.level = 1;
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        assert!(
            drain_train_msgs(&mut rx).is_empty(),
            "cross-class training rejected"
        );
    }

    #[tokio::test]
    async fn rejects_below_required_level() {
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        // Requires level 10.
        seed_tree(
            &mut mgr,
            TreeNode::with_defaults(1, 0, TEST_ABILITY, 10, vec![]),
        );
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.level = 5; // below requirement
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        assert!(drain_train_msgs(&mut rx).is_empty(), "level-gated reject");
    }

    #[tokio::test]
    async fn rejects_missing_prerequisite_ability() {
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        // Requires ability 1000.
        seed_tree(
            &mut mgr,
            TreeNode::with_defaults(1, 0, TEST_ABILITY, 1, vec![1000]),
        );
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.level = 1;
            // Doesn't know prerequisite 1000.
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        assert!(drain_train_msgs(&mut rx).is_empty(), "prereq-gated reject");
    }

    #[tokio::test]
    async fn passes_all_guards_sends_train_ability_to_base() {
        let mut mgr = make_mgr();
        seed_ability(&mut mgr, TEST_ABILITY);
        seed_tree(
            &mut mgr,
            TreeNode::with_defaults(1, 0, TEST_ABILITY, 1, vec![]),
        );
        if let Some(e) = mgr.get_entity_mut(1) {
            e.player_id = Some(100);
            e.archetype_id = Some(1);
            e.level = 5;
            e.tree_progress.training_points = 1;
        }
        let (tx, mut rx) = mpsc::channel(8);
        handle_train_ability(1, TEST_ABILITY, &tx, &mut mgr).await;
        let sent = drain_train_msgs(&mut rx);
        assert_eq!(sent, vec![TEST_ABILITY], "valid train forwarded to base");
    }
}
