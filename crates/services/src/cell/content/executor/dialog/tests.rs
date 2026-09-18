//! Unit tests for the dialog action handlers.
//!
//! Split out of `mod.rs` to keep that file under the 700-line cap; the
//! `#[cfg(test)] mod tests;` declaration there resolves to this file
//! unchanged.

use super::*;
use crate::test_support::{make_space_manager, LogCapture};
use std::collections::HashMap;
use tracing::Level;

fn empty_params() -> HashMap<String, serde_json::Value> {
    HashMap::new()
}

/// Chain `params["target_entity_id"]` is the most direct source — it's
/// stamped by `fire_interact_*` for chains fired off an interact. The
/// wire `EntityId` of `onDialogDisplay` must match that, not the
/// player's id.
#[tokio::test]
async fn display_uses_target_entity_id_from_chain_params() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    const NPC_ID: u32 = 0xABCD;
    let mut params = empty_params();
    params.insert("target_entity_id".into(), serde_json::json!(NPC_ID as u64));

    let (tx, mut rx) = mpsc::channel(4);
    display(
        /* dialog_id */ 4001, /* entity_id */ 1, /* chain_id */ 99, &params, &tx,
        &mut mgr,
    )
    .await;

    let msg = rx.try_recv().expect("must emit onDialogDisplay");
    match msg {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(
                wire_entity_id as u32, NPC_ID,
                "params.target_entity_id must win over any fallback; got {wire_entity_id}, expected {NPC_ID}"
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// When the chain didn't stamp `target_entity_id` (e.g. an
/// `OnDialogChoice`-triggered follow-up dialog), fall back to the
/// player's `last_interaction_target` pin.
#[tokio::test]
async fn display_falls_back_to_last_interaction_target() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    const NPC_ID: u32 = 0xBEEF;
    if let Some(p) = mgr.get_entity_mut(1) {
        p.last_interaction_target = Some(NPC_ID);
    }

    let params = empty_params();
    let (tx, mut rx) = mpsc::channel(4);
    display(2299, 1, 1021, &params, &tx, &mut mgr).await;

    let msg = rx.try_recv().expect("must emit onDialogDisplay");
    match msg {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(wire_entity_id as u32, NPC_ID);
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// With neither `target_entity_id` in params nor a
/// `last_interaction_target` pin, the handler must abort with a warn
/// rather than emit a wire frame that binds the player as the
/// speaker. The warn level is load-bearing — operators rely on it
/// to correlate "dialog never opened" with a chain that wasn't
/// fired off an interact path.
#[tokio::test]
async fn display_aborts_with_warn_when_no_npc_id_available() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    // last_interaction_target intentionally None.

    let params = empty_params();
    let (tx, mut rx) = mpsc::channel(4);
    display(4001, 1, 9999, &params, &tx, &mut mgr).await;

    assert!(
        rx.try_recv().is_err(),
        "must not emit onDialogDisplay when no NPC id can be resolved"
    );
    assert!(
        capture
            .find_message(Level::WARN, "DisplayDialog: no NPC entity id")
            .is_some(),
        "abort must surface a WARN — silent return masks the chain-author bug"
    );
}

/// When no NPC resolves and the dialog is in the monologue cache,
/// the wire EntityId must be the player's own id (not bail). The
/// per-screen `speaker_id = 0` lookup on the client falls back to
/// the player's name, rendering as inner thought.
#[tokio::test]
async fn display_binds_player_when_no_npc_and_dialog_is_monologue() {
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    // Cache dialog 2982 as a known monologue.
    mgr.monologue_dialog_ids.insert(2982);

    let params = empty_params();
    let (tx, mut rx) = mpsc::channel(4);
    display(2982, 1, 1001, &params, &tx, &mut mgr).await;

    let msg = rx.try_recv().expect(
        "monologue dialog must emit onDialogDisplay even with no NPC \
         resolved — reverting the monologue branch in display() fails here",
    );
    match msg {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(
                wire_entity_id, 1,
                "monologue must bind the player's own entity id as the wire EntityId"
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// Castle dialog_set_map 3062: `dialog_id IS NULL`, `interaction_flags =
/// 16777216` (`INT_AStoryMissionActive`, the `!` over an NPC's head).
/// `Castle.py` binds it on Sgt. Gerschon (template 149) with
/// `addDialog(149, 3062)`.
///
/// **Regression guard for defect B3.** Before CA02 the loader dropped
/// NULL-dialog rows, so this bind was a `dialog_set_maps cache miss` warn
/// and a no-op — the `!` never appeared. The guard asserts the three
/// things that must hold for a flag-only bind:
///
/// 1. the bind is recorded with `dialog_id: None` (not a substituted 0),
/// 2. exactly one `InteractionType` push goes out, carrying the `!` bit
///    merged over the NPC's base flags,
/// 3. no dialog is displayed — a flag-only row has nothing to show, and
///    Gerschon's dialog comes from a separate `interact_tag` chain.
#[tokio::test]
async fn add_dialog_set_with_null_dialog_pushes_flag_only() {
    use crate::cell::spawner::DialogSetMapEntry;
    use cimmeria_common::EntityId;

    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;
    /// `INT_AStoryMissionActive` — bit 24, the `!` indicator.
    const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;
    /// Arbitrary pre-existing base flag on the NPC, to prove the push
    /// merges rather than replaces.
    const NPC_BASE_FLAGS: i64 = 0x2;

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    mgr.dialog_set_maps.insert(
        SET_MAP_3062,
        DialogSetMapEntry {
            dialog_id: None,
            interaction_flags: INT_A_STORY_MISSION_ACTIVE,
        },
    );
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(2));
    }
    if let Some(n) = mgr.get_entity_mut(2) {
        n.template_id = Some(TEMPLATE_GERSCHON);
        n.interaction_type_flags = NPC_BASE_FLAGS;
    }

    let (tx, mut rx) = mpsc::channel(8);
    add_dialog_set(
        SET_MAP_3062,
        TEMPLATE_GERSCHON,
        /* entity_id */ 1,
        /* chain_id */ 1201,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(1)
            .and_then(|p| p.available_interactions.get(&TEMPLATE_GERSCHON))
            .map(Vec::as_slice),
        Some([(SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE)].as_slice()),
        "the bind must be recorded with dialog_id None -- a substituted 0 \
         would make the click open an empty dialog"
    );

    let msg = rx.try_recv().expect(
        "flag-only bind must still push InteractionType -- with the loader \
         reverted to dropping NULL rows this is a cache miss and nothing is sent",
    );
    match msg {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            entity_is_player,
        } => {
            assert_eq!(witness_id, 1, "push is per-player, to the binding player");
            assert_eq!(entity_id, 2, "push targets the NPC, not the player");
            assert_eq!(method_index, crate::mercury::method_idx::INTERACTION_TYPE);
            assert!(!entity_is_player);
            assert_eq!(
                args,
                ((NPC_BASE_FLAGS | INT_A_STORY_MISSION_ACTIVE) as u64)
                    .to_le_bytes()
                    .to_vec(),
                "payload is the merged flags as UINT64 LE -- the `!` bit OR'd \
                 over the NPC's base flags"
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }

    assert!(
        rx.try_recv().is_err(),
        "a flag-only bind must emit the InteractionType push and nothing else -- \
         no onDialogDisplay, because the row has no dialog"
    );
}

/// **The same guard for the other public entry point (PR #661 review).**
///
/// `Action::AddDialog` reaches the same `available_interactions` tuple and the
/// same `InteractionType` push as `Action::AddDialogSet`, but by a separate
/// route: it takes `entity_template` as its own parameter rather than deriving
/// the slot from the action's template field, and it warns and returns when
/// that is `None`. Nothing structural stops the two from drifting — a future
/// "substitute 0 for a NULL dialog id" patch applied to one and not the other
/// would leave `add_dialog` broken with every `add_dialog_set` guard green.
///
/// `Castle.py` calls `addDialog(149, 3062)`, so this is the shape the original
/// content actually uses.
#[tokio::test]
async fn add_dialog_with_null_dialog_pushes_flag_only() {
    use crate::cell::spawner::DialogSetMapEntry;
    use cimmeria_common::EntityId;

    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3062: i32 = 3062;
    /// `INT_AStoryMissionActive` — bit 24, the `!` indicator.
    const INT_A_STORY_MISSION_ACTIVE: i64 = 16_777_216;
    const NPC_BASE_FLAGS: i64 = 0x2;

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    mgr.dialog_set_maps.insert(
        SET_MAP_3062,
        DialogSetMapEntry {
            dialog_id: None,
            interaction_flags: INT_A_STORY_MISSION_ACTIVE,
        },
    );
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(2));
    }
    if let Some(n) = mgr.get_entity_mut(2) {
        n.template_id = Some(TEMPLATE_GERSCHON);
        n.interaction_type_flags = NPC_BASE_FLAGS;
    }

    let (tx, mut rx) = mpsc::channel(8);
    add_dialog(
        SET_MAP_3062,
        Some(TEMPLATE_GERSCHON),
        /* entity_id */ 1,
        /* chain_id */ 1201,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(
        mgr.get_entity(1)
            .and_then(|p| p.available_interactions.get(&TEMPLATE_GERSCHON))
            .map(Vec::as_slice),
        Some([(SET_MAP_3062, None, INT_A_STORY_MISSION_ACTIVE)].as_slice()),
        "add_dialog must record dialog_id None too -- a substituted 0 on this \
         route would make the click open an empty dialog"
    );

    match rx.try_recv().expect(
        "add_dialog on a flag-only row must push InteractionType -- with the \
         loader reverted to dropping NULL rows this is a cache miss and nothing \
         is sent",
    ) {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            entity_is_player,
        } => {
            assert_eq!(witness_id, 1, "push is per-player, to the binding player");
            assert_eq!(entity_id, 2, "push targets the NPC, not the player");
            assert_eq!(method_index, crate::mercury::method_idx::INTERACTION_TYPE);
            assert!(!entity_is_player);
            assert_eq!(
                args,
                ((NPC_BASE_FLAGS | INT_A_STORY_MISSION_ACTIVE) as u64)
                    .to_le_bytes()
                    .to_vec(),
                "payload is the merged flags as UINT64 LE, identical to the \
                 add_dialog_set route"
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }

    assert!(
        rx.try_recv().is_err(),
        "one push, and no onDialogDisplay -- the row has no dialog"
    );
}

/// Companion to the flag-only guard: a bind whose row *does* carry a
/// dialog still behaves exactly as before. Pins that widening
/// `dialog_id` to `Option` didn't change the with-dialog path, and that
/// the dialog id stays off the wire in both cases — the pushed payload is
/// the flags bitfield alone, per
/// `entities/defs/SGWSpawnableEntity.def:114-116`.
#[tokio::test]
async fn add_dialog_set_with_dialog_pushes_same_flag_only_payload() {
    use crate::cell::spawner::DialogSetMapEntry;
    use cimmeria_common::EntityId;

    const TEMPLATE_GERSCHON: i32 = 149;
    const SET_MAP_3060: i32 = 3060;
    const DIALOG_2573: i32 = 2573;
    const FLAGS: i64 = 16_777_216;

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    mgr.dialog_set_maps.insert(
        SET_MAP_3060,
        DialogSetMapEntry {
            dialog_id: Some(DIALOG_2573),
            interaction_flags: FLAGS,
        },
    );
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(2));
    }
    if let Some(n) = mgr.get_entity_mut(2) {
        n.template_id = Some(TEMPLATE_GERSCHON);
    }

    let (tx, mut rx) = mpsc::channel(8);
    add_dialog_set(SET_MAP_3060, TEMPLATE_GERSCHON, 1, 1201, &tx, &mut mgr).await;

    assert_eq!(
        mgr.get_entity(1)
            .and_then(|p| p.available_interactions.get(&TEMPLATE_GERSCHON))
            .map(Vec::as_slice),
        Some([(SET_MAP_3060, Some(DIALOG_2573), FLAGS)].as_slice())
    );

    match rx
        .try_recv()
        .expect("with-dialog bind must push InteractionType")
    {
        CellToBaseMsg::WitnessEntityMethod {
            method_index, args, ..
        } => {
            assert_eq!(method_index, crate::mercury::method_idx::INTERACTION_TYPE);
            assert_eq!(
                args,
                (FLAGS as u64).to_le_bytes().to_vec(),
                "the dialog id is server-side state and must never appear in \
                 the InteractionType payload"
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "exactly one push");
}

/// **Precedence guard.** A monologue dialog must bind the player even
/// when an NPC *is* resolvable — both from the sticky
/// `last_interaction_target` pin and from an explicit
/// `target_entity_id` chain param.
///
/// `last_interaction_target` is never cleared, so once the player has
/// clicked any NPC there is permanently an NPC in scope. If the
/// monologue check were a fallback rather than the first branch, every
/// monologue fired after an interact — a minigame victory chain, a
/// follow-up `dialog_choice` — would portray that NPC speaking lines
/// the author wrote as the player's inner thoughts. Castle mission
/// 701's dialog 2575 ("You manage to free Capt. Copplemann...") is
/// exactly this shape, and `Castle.py` passes an explicit
/// `displayDialog(None, 2575)`.
#[tokio::test]
async fn monologue_binds_player_even_when_an_npc_is_resolvable() {
    const NPC_ID: u32 = 0xC0FFEE;
    const MONOLOGUE: i32 = 2575;

    // Case 1: NPC available via the sticky pin.
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.monologue_dialog_ids.insert(MONOLOGUE);
    if let Some(p) = mgr.get_entity_mut(1) {
        p.last_interaction_target = Some(NPC_ID);
    }

    let (tx, mut rx) = mpsc::channel(4);
    display(MONOLOGUE, 1, 1234, &empty_params(), &tx, &mut mgr).await;

    match rx.try_recv().expect("monologue must still display") {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let wire = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(
                wire, 1,
                "a monologue must bind the PLAYER, not the pinned NPC \
                 ({NPC_ID}) — reverting the monologue-first ordering \
                 fails here",
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    // Case 2: NPC available via an explicit chain param, which is an
    // even stronger source than the pin.
    let mut params = empty_params();
    params.insert("target_entity_id".into(), serde_json::json!(NPC_ID as u64));
    display(MONOLOGUE, 1, 1234, &params, &tx, &mut mgr).await;

    match rx.try_recv().expect("monologue must still display") {
        CellToBaseMsg::EntityMethodCall { args, .. } => {
            let wire = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(
                wire, 1,
                "a monologue must outrank an explicit target_entity_id too",
            );
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// Companion guard: a dialog NOT in the monologue cache still
/// bails when no NPC resolves. Pins that the fallback is gated on
/// the cache, not an "accept anything" hole — an NPC dialog whose
/// chain context was lost must still warn, because binding the
/// player there would blank the NPC portrait and substitute the
/// player's name for every screen.
#[tokio::test]
async fn display_still_aborts_for_npc_dialog_not_in_monologue_cache() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    // monologue_dialog_ids intentionally empty for dialog 4001 (NPC
    // dialog — Future Col Marsh).

    let params = empty_params();
    let (tx, mut rx) = mpsc::channel(4);
    display(4001, 1, 9999, &params, &tx, &mut mgr).await;

    assert!(
        rx.try_recv().is_err(),
        "non-monologue dialog with no NPC must still bail"
    );
    assert!(
        capture
            .find_message(Level::WARN, "DisplayDialog: no NPC entity id")
            .is_some(),
        "non-monologue bail must still surface the WARN"
    );
}

/// **Regression guard (PR #661 review, item 1):** a second bind on the same
/// template must push the fold of *every* bind, not just the entry it added.
///
/// `InteractionType` replaces the client's bitfield rather than OR-ing into it,
/// so pushing only the new entry's flags clears every indicator bound earlier
/// on that template. The player would watch the `!` vanish when a second topic
/// was bound, and it would not come back until the NPC re-entered AoI and
/// `compute_player_aoi` re-sent the full fold. Two binds on one template is a
/// designed shape since CA02 — an interaction-only indicator alongside a topic
/// that carries a dialog — so the bind path has to fold like the two paths that
/// already do (`remove_dialog_set` and the AoI re-send).
///
/// Reverting `send_interaction_update_if_visible` to `base_flags |
/// entry.interaction_flags` fails on the second push.
#[tokio::test]
async fn second_bind_on_same_template_pushes_both_indicator_bits() {
    use crate::cell::spawner::DialogSetMapEntry;
    use cimmeria_common::EntityId;

    const TEMPLATE: i32 = 149;
    const SET_MAP_FLAG_ONLY: i32 = 3062;
    const SET_MAP_WITH_DIALOG: i32 = 3060;
    /// `INT_AStoryMissionActive` — bit 24, the `!`.
    const BIT_ACTIVE: i64 = 16_777_216;
    /// `INT_MissionWorldObject` — bit 30, a different indicator.
    const BIT_WORLD_OBJECT: i64 = 1_073_741_824;

    let mut mgr = make_space_manager();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    mgr.dialog_set_maps.insert(
        SET_MAP_FLAG_ONLY,
        DialogSetMapEntry {
            dialog_id: None,
            interaction_flags: BIT_ACTIVE,
        },
    );
    mgr.dialog_set_maps.insert(
        SET_MAP_WITH_DIALOG,
        DialogSetMapEntry {
            dialog_id: Some(2573),
            interaction_flags: BIT_WORLD_OBJECT,
        },
    );
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        p.witnesses.insert(EntityId(2));
    }
    if let Some(n) = mgr.get_entity_mut(2) {
        n.template_id = Some(TEMPLATE);
    }

    let (tx, mut rx) = mpsc::channel(8);

    add_dialog_set(SET_MAP_FLAG_ONLY, TEMPLATE, 1, 1201, &tx, &mut mgr).await;
    match rx.try_recv().expect("first bind must push") {
        CellToBaseMsg::WitnessEntityMethod { args, .. } => assert_eq!(
            args,
            (BIT_ACTIVE as u64).to_le_bytes().to_vec(),
            "first push carries just the first bind's bit"
        ),
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }

    add_dialog_set(SET_MAP_WITH_DIALOG, TEMPLATE, 1, 1202, &tx, &mut mgr).await;
    match rx.try_recv().expect("second bind must push") {
        CellToBaseMsg::WitnessEntityMethod { args, .. } => assert_eq!(
            args,
            ((BIT_ACTIVE | BIT_WORLD_OBJECT) as u64)
                .to_le_bytes()
                .to_vec(),
            "second push must carry BOTH bits -- folding only the new entry \
             clears the first bind's indicator on the client until the next \
             AoI entry"
        ),
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "one push per bind");
}
