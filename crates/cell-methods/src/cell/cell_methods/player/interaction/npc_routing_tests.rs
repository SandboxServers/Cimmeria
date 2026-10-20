//! NPC interact routing tests: hostile reroute, loot, trainer.

use super::*;
use crate::test_support::make_space_manager;
use cimmeria_entity::cell_entity::NpcInteractionType;

/// Right-clicking an ALIVE hostile NPC reroutes interact → useAbility.
/// Baseline for the dead-corpse regression test below.
#[tokio::test]
async fn alive_hostile_reroutes_to_useability() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.faction = 10; // hostile
        npc.clear_all_state_flags(); // alive — hard reset (counters too)
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let mut args = Vec::with_capacity(4);
    args.extend_from_slice(&(npc_id as i32).to_le_bytes());
    let handled = dispatch(1, INTERACT, &args, &tx, &mut mgr, &engine).await;
    assert!(handled);

    // Hostile reroute path sends method 16 (onTargetUpdate) first, then
    // useAbility flows downstream. We just check at least one message
    // came through and that NO loot/dialog message did.
    let mut saw_target_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == 16 {
                saw_target_update = true;
            }
            // Loot display would be method 114 — must not appear.
            assert_ne!(method_index, 114, "loot must not fire for an alive hostile");
        }
    }
    assert!(
        saw_target_update,
        "expected onTargetUpdate from hostile reroute"
    );
}

/// Right-clicking a DEAD hostile corpse with loot must reach
/// `handle_interact`, set `looting_entity` on the player, and emit
/// `onLootDisplay` (method 114). Regression for the
/// "right-click on corpse silently rerouted to useAbility" bug —
/// see [docs/reverse-engineering/findings/right-click-routing-on-corpse.md]
/// and [interaction.rs:37-49] (the `is_dead_state` guard).
#[tokio::test]
async fn dead_hostile_with_loot_routes_to_interact() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
    }
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.faction = 10; // hostile
        npc.set_state_flag(crate::cell::combat::BSF_DEAD);
        npc.interaction_type = Some(NpcInteractionType::Loot);
        // Seed real loot. Without this, the test only proves dispatch
        // routes to method 114 — the corpse could be empty and the test
        // would still pass. With a real LootItem in place, asserting
        // count > 0 below catches a future "empty list short-circuit"
        // regression in handle_interact (e.g., if it grows a guard like
        // `interaction_type = Some(Loot) AND target.loot non-empty`).
        npc.loot.push(cimmeria_entity::cell_entity::LootItem {
            design_id: None, // None = naquadah (cash)
            quantity: 50,
            index: 1,
        });
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let mut dispatch_args = Vec::with_capacity(4);
    dispatch_args.extend_from_slice(&(npc_id as i32).to_le_bytes());
    let handled = dispatch(1, INTERACT, &dispatch_args, &tx, &mut mgr, &engine).await;
    assert!(handled);

    // Player should now be marked as looting this corpse.
    assert_eq!(
        mgr.get_entity(1).and_then(|p| p.looting_entity),
        Some(npc_id),
        "interact handler must set looting_entity on the player"
    );

    // onLootDisplay (method 114) must have been queued for the player AND
    // the encoded loot count must be non-zero. Wire layout from
    // send_loot_display: `entity_id:i32, count:u32, items[..], initial:u8`.
    let mut saw_loot_display = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 1 && method_index == 114 {
                // We seeded exactly one LootItem; assert the encoded count
                // matches that, not just `> 0`. A future bug that
                // duplicated entries or announced the wrong length would
                // otherwise still pass this regression guard.
                let count = u32::from_le_bytes(args[4..8].try_into().unwrap());
                assert_eq!(
                    count, 1,
                    "onLootDisplay must announce exactly the seeded loot count (1)"
                );
                // The trailing byte is the `initial` flag from
                // send_loot_display: 1 for the first display (opens the
                // loot window) and 0 for refresh-only packets. The
                // first-open path is the one a right-click on a corpse
                // takes, so this must be 1.
                let initial = *args
                    .last()
                    .expect("onLootDisplay payload missing trailing initial byte");
                assert_eq!(
                    initial, 1,
                    "first-display onLootDisplay must set initial=1 to open the loot window client-side"
                );
                saw_loot_display = true;
            }
            // useAbility / target reticle must NOT have been sent.
            assert_ne!(
                method_index, 16,
                "dead hostile must not fire onTargetUpdate"
            );
        }
    }
    assert!(
        saw_loot_display,
        "expected onLootDisplay (method 114) for dead lootable corpse"
    );
}

/// A neutral NPC (faction != 10) with no dialog/template/tag must NOT be
/// rerouted to combat. The is_hostile early branch is gated on faction
/// 10 specifically — if a future regression drops the faction check the
/// fall-through would silently auto-attack neutral civilians on right-
/// click.
#[tokio::test]
async fn neutral_npc_with_no_interaction_does_not_route_to_combat() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.player_id = Some(42);
    }
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        // Neutral / non-hostile faction. Faction 10 is the only value
        // that enables the hostile reroute — anything else (including
        // the default 0) must fall through to the dialog/interact path.
        npc.faction = 0;
        npc.clear_all_state_flags();
        // No tag, no template name, no interaction_type configured —
        // i.e. there's nothing for the dialog/interact path to do
        // either. The point of the test is that *nothing combat-related*
        // happens, not that some interaction succeeds.
        npc.tag = None;
        npc.npc_name = None;
        npc.interaction_type = None;
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let mut args = Vec::with_capacity(4);
    args.extend_from_slice(&(npc_id as i32).to_le_bytes());
    let handled = dispatch(1, INTERACT, &args, &tx, &mut mgr, &engine).await;
    assert!(handled, "INTERACT must always return true (handled)");

    // Method 16 = onTargetUpdate is the canonical hostile-reroute marker
    // emitted at the top of the is_hostile branch (interaction.rs:53-56).
    // Asserting it never appears is a precise probe for "the hostile
    // reroute didn't run" — the alive_hostile_reroutes_to_useability test
    // pins the converse direction, so together they bracket the branch.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            assert_ne!(
                method_index, 16,
                "neutral NPC must not fire onTargetUpdate from INTERACT",
            );
        }
    }
}

/// Routing consolidation regression: exactly ONE onTrainerOpen per
/// interact, even when the NPC carries the legacy
/// `NpcInteractionType::Trainer` tag in addition to a template-driven
/// `trainer_ability_list_id`.
///
/// Pre-consolidation there were two code paths claiming to handle
/// trainer NPCs:
/// - `cell_methods/player/interaction.rs::dispatch` (INTERACT arm) →
///   the canonical try_open_trainer path → correctly built the
///   per-archetype, per-known-set list.
/// - `cell::interactions::dispatch::handle_interact` (the fall-through
///   below) → a stub at `cell::interactions::trainer::send_trainer_open`
///   that fabricated the list from `archetype_ability_tree` with
///   everything marked trainable: no per-archetype offering, no
///   already-known filter, no level/prereq check.
///
/// Both paths fired only if `NpcInteractionType::Trainer { archetype_id }`
/// was assigned AND the template had a `trainer_ability_list_id` — rare
/// in production today (no NPC has the variant set) but a latent
/// footgun: any future content edit setting
/// `interaction_type = Trainer { ... }` would silently emit a
/// double-onTrainerOpen with two different ability lists.
///
/// Post-consolidation: the canonical handler is the
/// `template_trainer_lists` path; the legacy `NpcInteractionType::Trainer`
/// arm in `interactions::dispatch::handle_interact` now logs a warning
/// and returns `None` without sending a duplicate frame.
#[tokio::test]
async fn trainer_interact_emits_exactly_one_on_trainer_open() {
    use crate::ability_tree::{AbilityTreeCatalog, TreeNode};

    let mut mgr = make_space_manager();

    // Player setup.
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        p.archetype_id = Some(2); // Commando
        p.level = 1;
    }

    // Trainer NPC with BOTH template_trainer_lists registration AND the
    // legacy `NpcInteractionType::Trainer` tag — this is the bug-shape
    // scenario where pre-consolidation both paths would fire. After
    // consolidation only the template-driven path emits onTrainerOpen.
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.template_id = Some(25);
        npc.interaction_type = Some(NpcInteractionType::Trainer { archetype_id: 2 });
    }
    mgr.template_trainer_lists.insert(25, 1);
    mgr.trainer_abilities.insert((1, 2), vec![597]);
    crate::test_support::seed_ability_defs(&mut mgr, &[597]);
    mgr.ability_tree_catalog =
        AbilityTreeCatalog::from_nodes([TreeNode::with_defaults(2, 1, 597, 1, vec![])]);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let mut args = Vec::with_capacity(4);
    args.extend_from_slice(&(npc_id as i32).to_le_bytes());
    let handled = dispatch(1, INTERACT, &args, &tx, &mut mgr, &engine).await;
    assert!(handled);

    // Count emitted onTrainerOpen frames. Pre-consolidation, both the
    // template-driven `try_open_trainer` AND the legacy
    // `NpcInteractionType::Trainer` arm in handle_interact would have
    // sent one each — a guard that asserted "> 0" would have masked
    // the bug. Asserting `== 1` is what catches it.
    let mut on_trainer_open_count = 0u32;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_TRAINER_OPEN {
                assert_eq!(entity_id, 1, "frame must target the player");
                let wire_trainer_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_trainer_id as u32, npc_id,
                    "onTrainerOpen wire TrainerID must be the NPC id"
                );
                on_trainer_open_count += 1;
            }
        }
    }
    assert_eq!(
        on_trainer_open_count, 1,
        "trainer interact must emit exactly one onTrainerOpen — pre- \
         consolidation both the template-driven path AND the dead \
         NpcInteractionType::Trainer arm would fire, double-emitting \
         with two divergent ability lists"
    );
}
