//! `player_loaded Castle` restore chains 1361-1365.
//!
//! `CellEntity.interaction_type_flags` and `available_interactions` are
//! in-memory only — nothing persists them across a server restart — so
//! every cue mission 708 paints has to be repainted on load. Without
//! these chains a player who relogs mid-mission finds the guard, the
//! panel, the report NPC or the DHD rendered as scenery and has no way
//! to progress.
//!
//! One chain per step that owns a cue. Two steps deliberately own none:
//!
//! - **2416** paints nothing. Chains 1350/1351 fire at the 2416 → 2417
//!   TRANSITION, and the resulting 2417 state is covered by 1362/1363.
//! - **4469** paints nothing. The gate is already open and the player
//!   only has to walk through it.
//!
//! All five are idempotent: `|` on an already-set bit is a no-op, and
//! `add_dialog_set` on an already-bound slot appends a duplicate entry
//! that resolves to the same topic.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

use super::{actions_of, count_flag_ops, engine_for, fire, BANG, GLOW, JAFFA, LIVEWIRE, TAURI};
use crate::test_support::require_db_or_skip;

/// Context for a Castle world load on a given step and archetype.
fn load_ctx(step_id: i32, archetype: i32) -> ExecutionContext {
    let mut ctx = super::step_ctx(step_id, archetype);
    ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
    ctx
}

/// Chain 1361: both step-2415 affordances come back.
#[tokio::test]
async fn chain_1361_restores_both_diagnosis_cues_at_step_2415() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1361).await;

    let resolved = fire(&engine, TriggerType::PlayerLoaded, &load_ctx(2415, TAURI));
    let actions = actions_of(&resolved, 1361);
    assert_eq!(
        actions.len(),
        2,
        "chain 1361 must restore exactly the guard cue and the panel glow; \
         got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_SurrenderGuard", "|", BANG),
        1,
        "chain 1361 must repaint the surrender guard's '!'; got {actions:?}",
    );
    assert_eq!(
        count_flag_ops(&actions, "Castle_AccessPanel", "|", GLOW),
        1,
        "chain 1361 must repaint the Access Panel glow — it is the only bit \
         making template 147 clickable; got {actions:?}",
    );
}

/// Chains 1362/1363: the report cue comes back for the right faction
/// only. A Jaffa who relogs must not find Col. Marsh flagged, and vice
/// versa — the same exclusivity the live chains 1350/1351 enforce.
#[tokio::test]
async fn the_report_cue_restores_for_the_players_own_faction_only() {
    let pool = require_db_or_skip!();
    let marsh = engine_for(&pool, 1362).await;
    let mohkatan = engine_for(&pool, 1363).await;

    let tauri = load_ctx(2417, TAURI);
    let resolved = fire(&marsh, TriggerType::PlayerLoaded, &tauri);
    let actions = actions_of(&resolved, 1362);
    assert_eq!(
        count_flag_ops(&actions, "Castle_ColMarsh", "|", BANG),
        1,
        "chain 1362 must repaint Col. Marsh's '!' for a Tau'ri at step 2417; \
         got {actions:?}",
    );
    assert!(
        actions_of(&fire(&mohkatan, TriggerType::PlayerLoaded, &tauri), 1363).is_empty(),
        "chain 1363 must not repaint Moh'katan for a Tau'ri",
    );

    let jaffa = load_ctx(2417, JAFFA);
    let resolved = fire(&mohkatan, TriggerType::PlayerLoaded, &jaffa);
    let actions = actions_of(&resolved, 1363);
    assert_eq!(
        count_flag_ops(&actions, "Castle_Mohkatan", "|", BANG),
        1,
        "chain 1363 must repaint Moh'katan's '!' for a Jaffa at step 2417; \
         got {actions:?}",
    );
    assert!(
        actions_of(&fire(&marsh, TriggerType::PlayerLoaded, &jaffa), 1362).is_empty(),
        "chain 1362 must not repaint Col. Marsh for a Jaffa",
    );
}

/// Chain 1364: the DHD's Livewire affordance comes back at step 2418.
/// Without it a relog at 2418 leaves the DHD dialable (template 162's
/// INT_Dhd survives, it is a template column) but un-hackable, and the
/// player cannot repair it.
#[tokio::test]
async fn chain_1364_restores_the_dhd_livewire_cue_at_step_2418() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1364).await;

    let resolved = fire(&engine, TriggerType::PlayerLoaded, &load_ctx(2418, TAURI));
    let actions = actions_of(&resolved, 1364);
    assert_eq!(
        count_flag_ops(&actions, "Castle_DHD", "|", LIVEWIRE),
        1,
        "chain 1364 must repaint the DHD's Livewire cue; got {actions:?}",
    );
}

/// Chain 1365: the "Dial Harset" topic is re-bound at step 4462.
/// Inert until packet CA02 keeps NULL-dialog rows in the cache; the seed
/// row still has to be correct before then.
#[tokio::test]
async fn chain_1365_restores_the_dial_topic_bind_at_step_4462() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1365).await;

    let resolved = fire(&engine, TriggerType::PlayerLoaded, &load_ctx(4462, TAURI));
    let actions = actions_of(&resolved, 1365);
    assert_eq!(
        actions.len(),
        1,
        "chain 1365 must resolve exactly one action; got {actions:?}",
    );
    assert!(
        matches!(
            actions[0],
            Action::AddDialogSet {
                dialog_set_id: 3073,
                slot: 162,
                ..
            }
        ),
        "chain 1365 must re-bind row 3073 to template 162; got {:?}",
        actions[0],
    );
}

/// Every restore chain is gated on its own step and on the `Castle`
/// world name. Two failure shapes are covered at once: repainting a cue
/// on a step that never lit it (the player would see a "!" over an NPC
/// with nothing to say), and firing a Castle chain on a Cellblock load.
#[tokio::test]
async fn every_restore_chain_is_gated_on_its_step_and_on_the_castle_world() {
    let pool = require_db_or_skip!();

    // (chain, the step it owns, the archetype it wants)
    let chains = [
        (1361, 2415, TAURI),
        (1362, 2417, TAURI),
        (1363, 2417, JAFFA),
        (1364, 2418, TAURI),
        (1365, 4462, TAURI),
    ];

    for (chain_id, own_step, archetype) in chains {
        let engine = engine_for(&pool, chain_id).await;

        for other_step in [2415, 2416, 2417, 2418, 4462, 4469] {
            if other_step == own_step {
                continue;
            }
            assert!(
                actions_of(
                    &fire(
                        &engine,
                        TriggerType::PlayerLoaded,
                        &load_ctx(other_step, archetype)
                    ),
                    chain_id as i64
                )
                .is_empty(),
                "chain {chain_id} owns step {own_step} and must not repaint on a \
                 load at step {other_step}",
            );
        }

        let mut cellblock = super::step_ctx(own_step, archetype);
        cellblock.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        assert!(
            actions_of(
                &fire(&engine, TriggerType::PlayerLoaded, &cellblock),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} is keyed on world `Castle`; a Cellblock load must \
             not fire it",
        );
    }
}
