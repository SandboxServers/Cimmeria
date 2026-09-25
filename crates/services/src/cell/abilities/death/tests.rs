//! Tests for the death-transition burst and the shared [`super::resolve_death`]
//! resolver — extracted to a sibling file so `death/mod.rs` stays under the
//! 700-line hard cap.

use super::*;
use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;

/// The state_field bytes a real death emits — mirrors what
/// damage_apply.rs sets at the kill site before invoking
/// `apply_death_transition`.
const DEAD_STATE: u32 = BSF_DEAD | BSF_MOVEMENT_LOCK;

/// Build a `SpaceManager` with one player at id=1 and one NPC at id=2
/// in the SAME startup space. We use the non-instanced "Castle" world
/// because instanced worlds (like Castle_CellBlock) allocate a fresh
/// space on every `create_entity` call — putting the player and NPC
/// in different spaces, where AoI never sees them.
///
/// `connect_entity(1)` + an AoI tick populates the player's witness
/// set so messages addressed to the NPC fan out via
/// `WitnessEntityMethod` instead of being dropped at the empty-witness
/// branch of `send_entity_method`.
fn make_mgr_with_player_and_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Drain everything currently sitting in the channel into a Vec.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// Extract `(entity_id, method_index)` pairs for both player-direct and
/// NPC-witness routes. Tests compare ordering on this projection so the
/// exact wire enum variant doesn't matter — only that the right method
/// targeted the right entity in the right order.
fn methods(msgs: &[CellToBaseMsg]) -> Vec<(u32, u16)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } => Some((*entity_id, *method_index)),
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                ..
            } => Some((*entity_id, *method_index)),
            _ => None,
        })
        .collect()
}

/// NPC target killed by player attacker — full burst:
///   1. `onTargetUpdate(0)` to attacker (drop reticle)
///   2. INTERACTION_TYPE on the corpse
///   3. `onStateFieldUpdate` on the corpse with dead bit
///
/// (No threatened-mob clear because the player wasn't actually threatened
/// by this NPC in this fixture — combat::clear_dead_npc_from_all_player_threat
/// returns an empty vec when nobody had it on their threat list.)
///
/// The INTERACTION_TYPE-before-state-update ordering is load-bearing per
/// the module-level docs; this test pins it.
#[tokio::test]
async fn npc_target_player_attacker_emits_full_burst_in_order() {
    let mut mgr = make_mgr_with_player_and_npc();
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.interaction_type_flags = 1 << 5; // pre-existing bit must survive
    }
    // The corpse-side state_field is whatever the caller already
    // mutated before invoking apply_death_transition — typically
    // BSF_DEAD | BSF_MOVEMENT_LOCK (set by damage_apply.rs at the
    // kill site). Mirror that here so the state_field bytes are
    // what a real death actually emits.
    let target_state = DEAD_STATE;
    let (tx, mut rx) = mpsc::channel(32);

    apply_death_transition(2, 1, target_state, true, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let pairs = methods(&msgs);
    // Locate the three load-bearing entries in order.
    let ix_target = pairs
        .iter()
        .position(|p| *p == (1, method_idx::ON_TARGET_UPDATE))
        .expect("attacker should receive onTargetUpdate(0)");
    let ix_int = pairs
        .iter()
        .position(|p| *p == (2, method_idx::INTERACTION_TYPE))
        .expect("corpse should receive INTERACTION_TYPE");
    let ix_state = pairs
        .iter()
        .position(|p| *p == (2, method_idx::ON_STATE_FIELD_UPDATE))
        .expect("corpse should receive onStateFieldUpdate");
    assert!(
        ix_target < ix_int && ix_int < ix_state,
        "ordering must be onTargetUpdate -> INTERACTION_TYPE -> onStateFieldUpdate; got {pairs:?}"
    );
}

/// NPC attacker killing an NPC: no reticle drop on a non-player attacker.
/// The corpse-side burst still fires.
#[tokio::test]
async fn npc_attacker_skips_on_target_update() {
    let mut mgr = make_mgr_with_player_and_npc();
    // Re-flag entity 1 as an NPC so `attacker_is_player = false`.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = false;
        p.player_id = None;
    }
    let (tx, mut rx) = mpsc::channel(32);

    apply_death_transition(2, 1, DEAD_STATE, false, false, &tx, &mut mgr).await;

    let pairs = methods(&drain(&mut rx));
    assert!(
        !pairs.contains(&(1, method_idx::ON_TARGET_UPDATE)),
        "non-player attacker must not receive onTargetUpdate; got {pairs:?}"
    );
}

/// Player target dying: skip both the threatened-mob clear (target is a
/// player, not a dying NPC) and the corpse-side INTERACTION_TYPE update
/// (player corpses don't loot). The reticle drop and state-field flip
/// still fire.
#[tokio::test]
async fn player_target_skips_interaction_type_and_threat_clear() {
    let mut mgr = make_mgr_with_player_and_npc();
    // entity 2 is the dying target; promote it to a player.
    if let Some(t) = mgr.get_entity_mut(2) {
        t.is_player = true;
        t.player_id = Some(200);
    }
    let (tx, mut rx) = mpsc::channel(32);

    apply_death_transition(2, 1, DEAD_STATE, true, true, &tx, &mut mgr).await;

    let pairs = methods(&drain(&mut rx));
    assert!(
        !pairs.contains(&(2, method_idx::INTERACTION_TYPE)),
        "player target must not receive INTERACTION_TYPE; got {pairs:?}"
    );
    assert!(
        pairs.contains(&(2, method_idx::ON_STATE_FIELD_UPDATE)),
        "player target must still receive onStateFieldUpdate; got {pairs:?}"
    );
}

/// INTERACTION_TYPE payload is the entity's `interaction_type_flags`
/// re-cast to `u64` and serialized little-endian. A refactor that
/// truncates to u32 (the column is `i64`) would silently lose the
/// high-bit `INT_NormalLoot` (1<<62), so we pin the byte layout.
#[tokio::test]
async fn interaction_type_payload_is_little_endian_u64_of_full_flags() {
    let mut mgr = make_mgr_with_player_and_npc();
    let high_bit_flag: i64 = 1 << 62; // INT_NormalLoot equivalent
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.interaction_type_flags = high_bit_flag;
    }
    let (tx, mut rx) = mpsc::channel(32);

    apply_death_transition(2, 1, DEAD_STATE, true, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let int_msg = msgs
        .iter()
        .find_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            }
            | CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *entity_id == 2 && *method_index == method_idx::INTERACTION_TYPE => {
                Some(args.clone())
            }
            _ => None,
        })
        .expect("INTERACTION_TYPE must be sent for an NPC target");
    assert_eq!(
        int_msg.len(),
        8,
        "INTERACTION_TYPE payload must be exactly 8 bytes"
    );
    let u = u64::from_le_bytes(int_msg.try_into().unwrap());
    assert_eq!(
        u, high_bit_flag as u64,
        "payload must reproduce the full i64 flag bits as little-endian u64 — preserves the high INT_NormalLoot bit"
    );
}

/// `ON_STATE_FIELD_UPDATE` for the corpse carries the caller's already-
/// mutated `target_state`. Pin the byte layout so a refactor that
/// re-reads from the entity (after the threat-clear step has run!)
/// can't silently drop the BSF_Dead bit.
#[tokio::test]
async fn corpse_state_field_update_carries_caller_supplied_state() {
    let mut mgr = make_mgr_with_player_and_npc();
    let (tx, mut rx) = mpsc::channel(32);
    let target_state: u32 = 0x1234_5678;

    apply_death_transition(2, 1, target_state, true, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    // Find the LAST onStateFieldUpdate addressed at the corpse — the
    // module ships several on-state-field-update messages in this flow
    // (one optional per-player threat-clear, then the final corpse one);
    // the corpse one is the load-bearing tail.
    let final_state_args = msgs
        .iter()
        .rev()
        .find_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                args,
                ..
            }
            | CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *entity_id == 2 && *method_index == method_idx::ON_STATE_FIELD_UPDATE => {
                Some(args.clone())
            }
            _ => None,
        })
        .expect("corpse must receive onStateFieldUpdate");
    assert_eq!(final_state_args.len(), 4);
    assert_eq!(
        u32::from_le_bytes(final_state_args.try_into().unwrap()),
        target_state
    );
}

/// When the dying entity is itself an auto-cycling player, their OWN
/// `auto_cycle` + stash + `BSF_AUTO_CYCLING` must clear in the same
/// death burst — not just the loops of OTHER players targeting them.
///
/// Bug shape this catches (the symptom that drove the fix): player
/// dies mid-auto-fire → respawns → the `auto_cycle` flag and stash
/// are still set → the tick auto-resumes the loop against the old
/// target without the player consenting. The death path is the right
/// place to wipe this state since `is_dead_state` becomes true here.
#[tokio::test]
async fn dying_player_own_auto_cycle_clears_and_broadcasts() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr_with_player_and_npc();
    // Promote entity 2 to a player who is auto-cycling at someone else.
    if let Some(t) = mgr.get_entity_mut(2) {
        t.is_player = true;
        t.player_id = Some(200);
        t.abilities.auto_cycle = true;
        t.abilities.auto_cycle_ability_id = Some(592);
        t.state_field |= BSF_AUTO_CYCLING;
    }
    let (tx, mut rx) = mpsc::channel(32);

    apply_death_transition(2, 1, DEAD_STATE, true, true, &tx, &mut mgr).await;

    let dying = mgr.get_entity(2).unwrap();
    assert!(
        !dying.abilities.auto_cycle,
        "dying player's auto_cycle must clear so respawn doesn't resume the loop",
    );
    assert!(dying.abilities.auto_cycle_ability_id.is_none());
    assert_eq!(
        dying.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF_AUTO_CYCLING must clear so the dying player's button un-highlights",
    );

    // Verify the broadcast went out to the dying player. There will
    // be multiple onStateFieldUpdate messages targeting entity 2 in
    // this burst (the auto-cycle clear and the corpse dead-state
    // flip); the auto-cycle one is identifiable because its payload
    // has BSF_AUTO_CYCLING (0x02) CLEARED and the corpse one carries
    // the DEAD_STATE bits (0x41).
    let msgs = drain(&mut rx);
    let auto_cycle_broadcast = msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: 2,
            method_index,
            args,
        } if *method_index == method_idx::ON_STATE_FIELD_UPDATE => {
            let state = u32::from_le_bytes(args.clone().try_into().unwrap_or([0; 4]));
            // Auto-cycle clear: BSF_AUTO_CYCLING bit cleared,
            // BSF_DEAD not yet set (that's the LATER broadcast).
            state & BSF_AUTO_CYCLING == 0 && state & BSF_DEAD == 0
        }
        _ => false,
    });
    assert!(
        auto_cycle_broadcast,
        "dying player must receive an onStateFieldUpdate clearing BSF_AUTO_CYCLING",
    );
}

/// When a PLAYER dies, `apply_death_transition` must route a
/// `ContactListPresenceEvent` (event_id=EVENT_DEATH, data_value=0) through
/// the `tx` channel so the base-side contact-list fanout fires.
///
/// Regression guard: removing the `tx.send(ContactListPresenceEvent {...})`
/// block from the `if target_is_player` branch causes this test to fail
/// because no `ContactListPresenceEvent` appears in the drained messages.
#[tokio::test]
async fn player_death_emits_contact_list_presence_event() {
    use crate::base::contact_list::wire::EVENT_DEATH;

    let mut mgr = make_mgr_with_player_and_npc();
    // Promote entity 2 (the target) to a player with a known character name.
    if let Some(t) = mgr.get_entity_mut(2) {
        t.is_player = true;
        t.player_id = Some(200);
        t.character_name = Some("Teal'c".to_string());
    }
    let (tx, mut rx) = mpsc::channel(64);

    apply_death_transition(2, 1, DEAD_STATE, true, true, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let presence_event = msgs.iter().find(|m| {
        matches!(
            m,
            CellToBaseMsg::ContactListPresenceEvent {
                event_id,
                data_value: 0,
                ..
            } if *event_id == EVENT_DEATH
        )
    });
    assert!(
        presence_event.is_some(),
        "player death must emit ContactListPresenceEvent(EVENT_DEATH, 0); \
         got: {msgs:?}"
    );

    // Pin the player_name carried in the event — it must be the entity's
    // character_name, not a fallback like "entity:2".
    if let Some(CellToBaseMsg::ContactListPresenceEvent { player_name, .. }) = presence_event {
        assert_eq!(
            player_name, "Teal'c",
            "ContactListPresenceEvent must carry the entity's character_name"
        );
    }
}

/// NPC death must NOT emit a `ContactListPresenceEvent` — the event is
/// player-only. High-volume NPC kills during combat must not flood the
/// contact-list fanout channel.
///
/// Regression guard: moving the `tx.send(ContactListPresenceEvent {...})`
/// block outside the `if target_is_player` guard would cause this test to
/// fail by finding an unexpected presence event for an NPC target.
#[tokio::test]
async fn npc_death_does_not_emit_contact_list_presence_event() {
    let mut mgr = make_mgr_with_player_and_npc();
    // entity 2 stays as NPC (is_player = false by default from the fixture).
    let (tx, mut rx) = mpsc::channel(64);

    apply_death_transition(2, 1, DEAD_STATE, true, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let has_presence = msgs
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::ContactListPresenceEvent { .. }));
    assert!(
        !has_presence,
        "NPC death must NOT emit ContactListPresenceEvent; got: {msgs:?}"
    );
}

/// Regression: when the dying entity was channelling an effect on a
/// target, `apply_death_transition` must cancel the channel so the
/// target stops taking pulses from a dead caster. Without the
/// cancellation hook in death.rs, the channel would tick until its
/// safety cap (60 pulses) or the target died.
#[tokio::test]
async fn channeller_death_cancels_active_channels() {
    use cimmeria_entity::abilities::EffectDef;
    use cimmeria_entity::cell_entity::ActiveEffectInstance;

    let mut mgr = make_mgr_with_player_and_npc();

    // Register a channelled effect on entity 2 (the dying target),
    // sourced by entity 1 (the killer). We bypass register_active_effect
    // so we don't have to dispatch over the network for the registration.
    let channel_effect = EffectDef {
        effect_id: 12345,
        ability_id: 9999,
        pulse_count: 0,
        pulse_duration: 0.5,
        ..Default::default()
    };
    mgr.effect_defs
        .insert(channel_effect.effect_id, channel_effect.clone());
    if let Some(target) = mgr.get_entity_mut(2) {
        target.active_effects.push(ActiveEffectInstance {
            effect_id: 12345,
            ability_id: 9999,
            invoker_id: 2, // dying entity is its own channel source
            remaining_pulses: 30,
            total_pulses: 60,
            next_pulse_at: std::time::Instant::now(),
            pulse_interval_secs: 0.5,
            invoker_position_at_register: None,
        });
    }
    assert_eq!(
        mgr.get_entity(2).unwrap().active_effects.len(),
        1,
        "pre-condition: channel registered"
    );

    let (tx, mut _rx) = mpsc::channel(64);
    apply_death_transition(2, 1, DEAD_STATE, true, true, &tx, &mut mgr).await;

    let active_after = &mgr.get_entity(2).unwrap().active_effects;
    assert!(
        active_after.is_empty(),
        "channel from dead invoker must be cancelled; got {active_after:?}"
    );
}
