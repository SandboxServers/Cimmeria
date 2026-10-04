//! `.dummy` in combat (Copilot findings on #1177): the disposition decides
//! whether a player's offensive cast may land, whatever the template's
//! faction; and taking a used dummy away leaves no player in combat with it.

use std::time::{Duration, Instant};

use cimmeria_cell_world::test_fixtures::{seed_mechanic_effect, MECHANIC_FIXTURE_EFFECT};
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

use super::dummy::{dummy_world, template};
use super::{console, CALLER};
use crate::cell::abilities::handle_use_ability;
use crate::cell::client_methods::being::ON_STATE_FIELD_UPDATE;
use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
use crate::cell::console::abilities::lab_dummy_tick;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{LabDummy, SpaceManager};

const WITNESS: u32 = 2;
/// A ranged attack the caller knows, with a mechanic (AB-12).
const ATTACK: i32 = 9100;
/// A template whose own faction (1, World Object) is not hostile.
const FACTION_1_TEMPLATE: i32 = 350;

fn attack_def() -> AbilityDef {
    AbilityDef {
        ability_id: ATTACK,
        name: "Lab Attack".to_string(),
        cooldown: 5.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: true,
        min_range: 0.0,
        max_range: 30.0,
        target_type_id: 0,
        effect_ids: vec![MECHANIC_FIXTURE_EFFECT],
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
        type_id: Default::default(),
        passive: false,
    }
}

fn combat_world() -> SpaceManager {
    let mut mgr = dummy_world();
    let mut t = template(FACTION_1_TEMPLATE);
    t.faction = Some(1);
    mgr.spawn_templates.insert(FACTION_1_TEMPLATE, t);
    mgr.ability_defs.insert(ATTACK, attack_def());
    seed_mechanic_effect(&mut mgr);
    mgr.get_entity_mut(CALLER)
        .unwrap()
        .abilities
        .add_ability(ATTACK);
    mgr
}

/// Place one dummy with `line`, put it in the caller's view, and fire
/// [`ATTACK`] at it. Returns whether the cast launched.
async fn shoot(line: &str) -> (bool, SpaceManager) {
    let mut mgr = combat_world();
    console(&mut mgr, None, line).await;
    let dummy = mgr.lab_dummies_of(CALLER)[0];
    let _ = mgr.compute_aoi_changes();
    let (tx, _rx) = mpsc::channel(256);
    let launched = handle_use_ability(CALLER, ATTACK, dummy as i32, &tx, &mut mgr).await;
    (launched, mgr)
}

/// Revert proof: drop the faction assignment in `dummy::place` and the
/// friendly faction-10 dummy launches while the hostile faction-1 one is
/// refused.
#[tokio::test]
async fn ab_l2_dummy_disposition_decides_whether_an_offensive_cast_lands() {
    // Template 34 is faction 10: `friendly` must still refuse the attack.
    let (launched, mgr) = shoot(".dummy friendly").await;
    assert!(!launched, "a friendly dummy is not a valid attack target");
    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(
        !caller.abilities.is_on_cooldown(ATTACK),
        "refused before the cooldown"
    );

    // Template 350 is faction 1: `hostile` must still accept it.
    let (launched, mgr) = shoot(&format!(".dummy hostile {FACTION_1_TEMPLATE}")).await;
    assert!(
        launched,
        "a hostile dummy takes an attack whatever its template"
    );
    assert!(mgr
        .get_entity(CALLER)
        .unwrap()
        .abilities
        .is_on_cooldown(ATTACK));

    // And the plain hostile default (template 34, faction 10) as before.
    let (launched, _) = shoot(".dummy").await;
    assert!(launched);
}

/// The caller hits only the dummy; the witness hits the dummy and also
/// fights the world's NPC. Returns the dummy id.
fn engage(mgr: &mut SpaceManager, npc: u32) -> u32 {
    let dummy = mgr.lab_dummies_of(CALLER)[0];
    let _ = generate_threat(mgr, CALLER, dummy, 10.0, AggroCause::Damage);
    let _ = generate_threat(mgr, WITNESS, dummy, 10.0, AggroCause::Damage);
    let _ = generate_threat(mgr, WITNESS, npc, 10.0, AggroCause::Damage);
    for p in [CALLER, WITNESS] {
        let e = mgr.get_entity(p).unwrap();
        assert!(e.threatened_mobs.contains(&dummy), "fixture: {p} fights it");
        assert!(e.state_field & BSF_IN_COMBAT != 0, "fixture: {p} in combat");
    }
    dummy
}

/// `(entity, state)` of every `onStateFieldUpdate` a player's own client got.
fn state_updates(msgs: &[CellToBaseMsg]) -> Vec<(u32, u32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STATE_FIELD_UPDATE,
                args,
            } => Some((*entity_id, u32::from_le_bytes(args[..4].try_into().ok()?))),
            _ => None,
        })
        .collect()
}

fn assert_left_combat_cleanly(mgr: &SpaceManager, msgs: &[CellToBaseMsg], dummy: u32, npc: u32) {
    let caller = mgr.get_entity(CALLER).unwrap();
    assert!(caller.threatened_mobs.is_empty(), "the dummy is drained");
    assert_eq!(
        caller.state_field & BSF_IN_COMBAT,
        0,
        "the caller left combat"
    );
    let witness = mgr.get_entity(WITNESS).unwrap();
    assert!(!witness.threatened_mobs.contains(&dummy));
    assert!(
        witness.threatened_mobs.contains(&npc),
        "still fighting the NPC"
    );
    assert!(
        witness.state_field & BSF_IN_COMBAT != 0,
        "so still in combat"
    );
    let updates = state_updates(msgs);
    assert!(
        updates.contains(&(CALLER, caller.state_field)),
        "the caller's client is told: {updates:?}"
    );
    assert!(
        updates.iter().all(|&(e, _)| e != WITNESS),
        "nothing changed for the witness: {updates:?}"
    );
}

/// Revert proof (both tests): drop the drain from `dummy::despawn` and the
/// caller stays in combat with a dummy that no longer exists.
#[tokio::test]
async fn ab_l2_dummy_clear_after_a_hit_takes_its_attackers_out_of_combat() {
    let (mut mgr, npc) = super::world(2);
    mgr.spawn_templates.insert(34, template(34));
    console(&mut mgr, None, ".dummy").await;
    let dummy = engage(&mut mgr, npc);

    let msgs = console(&mut mgr, None, ".dummy clear").await;

    assert!(mgr.get_entity(dummy).is_none());
    assert_left_combat_cleanly(&mgr, &msgs, dummy, npc);
}

#[tokio::test]
async fn ab_l2_dummy_expiry_after_a_hit_takes_its_attackers_out_of_combat() {
    let (mut mgr, npc) = super::world(2);
    mgr.spawn_templates.insert(34, template(34));
    console(&mut mgr, None, ".dummy").await;
    let dummy = engage(&mut mgr, npc);
    mgr.get_entity_mut(dummy)
        .unwrap()
        .extensions
        .get_mut::<LabDummy>()
        .unwrap()
        .expires_at = Instant::now() - Duration::from_secs(1);
    let (tx, mut rx) = mpsc::channel(64);

    lab_dummy_tick(&tx, &mut mgr).await;

    let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(mgr.get_entity(dummy).is_none());
    assert_left_combat_cleanly(&mgr, &msgs, dummy, npc);
}
