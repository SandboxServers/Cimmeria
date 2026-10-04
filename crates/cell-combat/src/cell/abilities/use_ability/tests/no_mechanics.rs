//! AB-12 (D-AB10): a player's press of an ability with no mechanic gets
//! `onErrorCode` and a feedback line, and charges nothing.
//!
//! Revert proof (2026-10-03): with the `refuse_without_mechanics` call
//! disabled in `handle.rs`,
//! `a_press_with_no_mechanic_is_refused_with_feedback_and_no_timer` fails on
//! its "no cooldown timer" assertion (the `onTimerUpdate` goes out), and
//! the negative-log guard fails because the press commits.

use tracing::Level;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::*;
use crate::cell::cover::COVER_STANCE_ABILITY;
use crate::cell::spawner::{
    AmmoCatalog, AmmoModifier, DeployableCatalog, DeployableSpec, PetSummon, PetSummonCatalog,
    EVENT_ITEM_MELEE,
};
use crate::test_support::LogCapture;

use super::super::no_mechanics::{
    ability_has_mechanics, NO_EFFECT_TEXT, NO_MECHANICS_ERROR_CODE, REASON_NO_MECHANICS,
};

/// An animation-only ability: an event set, no effects (102 of the
/// reachable abilities look like this; audit B-02).
const SILENT: i32 = 4242;
const PLAYER: u32 = 1;

fn silent_def() -> AbilityDef {
    let mut def = make_ability(SILENT, 0, 30);
    def.effect_ids = vec![];
    def.event_set_id = Some(55);
    def
}

fn scene() -> SpaceManager {
    let mut mgr = make_mgr();
    make_player(&mut mgr, PLAYER, [0.0; 3]);
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .add_ability(SILENT);
    mgr.ability_defs.insert(SILENT, silent_def());
    mgr
}

/// The two messages a refusal sends, byte for byte: `onErrorCode`
/// `(SystemID 0, InstanceID = ability, ErrorCodeID 167)`, then the
/// `CHAN_FEEDBACK` line from "SYSTEM".
fn expected_feedback(ability_id: i32) -> Vec<(u32, u16, Vec<u8>)> {
    let mut err = vec![0u8];
    err.extend_from_slice(&ability_id.to_le_bytes());
    err.extend_from_slice(
        &client_enum("CONDITION_FEEDBACK_EntityDoesNotHaveAbility").to_le_bytes(),
    );
    vec![
        (PLAYER, method_idx::ON_ERROR_CODE, err),
        (
            PLAYER,
            method_idx::ON_PLAYER_COMMUNICATION,
            serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, NO_EFFECT_TEXT),
        ),
    ]
}

/// A `CONDITION_FEEDBACK_*` value read from the enum file the client
/// parses (`entities/defs/enumerations.xml`), not restated as a literal.
fn client_enum(name: &str) -> u16 {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../entities/defs/enumerations.xml");
    let xml = std::fs::read_to_string(&path).expect("enumerations.xml");
    let line = xml
        .lines()
        .find(|l| l.contains(&format!("<Name>{name}</Name>")))
        .unwrap_or_else(|| panic!("{name} missing from enumerations.xml"));
    line.split("<Value>")
        .nth(1)
        .and_then(|v| v.split("</Value>").next())
        .expect("a <Value> on the line")
        .trim()
        .parse()
        .expect("a numeric value")
}

fn calls(msgs: &[CellToBaseMsg]) -> Vec<(u32, u16, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => Some((*entity_id, *method_index, args.clone())),
            _ => None,
        })
        .collect()
}

fn sent_timer(msgs: &[CellToBaseMsg]) -> bool {
    calls(msgs)
        .iter()
        .any(|(_, idx, _)| *idx == crate::cell::client_methods::being::ON_TIMER_UPDATE)
}

#[test]
fn the_feedback_text_and_code_are_the_decided_ones() {
    assert_eq!(NO_EFFECT_TEXT, "That ability has no effect yet.");
    assert_eq!(
        NO_MECHANICS_ERROR_CODE,
        client_enum("CONDITION_FEEDBACK_EntityDoesNotHaveAbility")
    );
    // The chat line's bytes, hand-built: WSTRING "SYSTEM", flags 0,
    // channel 9 (CHAN_FEEDBACK), WSTRING of the 31-character text.
    let mut want = 6u32.to_le_bytes().to_vec();
    for ch in "SYSTEM".encode_utf16() {
        want.extend_from_slice(&ch.to_le_bytes());
    }
    want.extend_from_slice(&[0, 9]);
    want.extend_from_slice(&31u32.to_le_bytes());
    for ch in NO_EFFECT_TEXT.encode_utf16() {
        want.extend_from_slice(&ch.to_le_bytes());
    }
    assert_eq!(expected_feedback(SILENT)[1].2, want);
}

/// The pipeline guard: refused, the exact two feedback messages and
/// nothing else, no cooldown charged, no timer sent. A second press is
/// answered again (nothing was charged to silence it).
#[tokio::test]
async fn a_press_with_no_mechanic_is_refused_with_feedback_and_no_timer() {
    let mut mgr = scene();
    let (tx, mut rx) = mpsc::channel(64);

    let committed = handle_use_ability(PLAYER, SILENT, 0, &tx, &mut mgr).await;
    let msgs = drain(&mut rx);
    // First, so a revert fails here: the pre-AB-12 launch sent the timer.
    assert!(
        !sent_timer(&msgs),
        "no cooldown timer may be sent for a refused press: {msgs:#?}"
    );
    assert!(
        !committed,
        "a press of an ability with no mechanic must not commit"
    );
    assert_eq!(calls(&msgs), expected_feedback(SILENT));
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(
        !player.abilities.is_on_cooldown(SILENT),
        "no cooldown charged"
    );
    assert_eq!(player.abilities.last_fired_ability_id, None);

    assert!(!handle_use_ability(PLAYER, SILENT, 0, &tx, &mut mgr).await);
    assert_eq!(calls(&drain(&mut rx)), expected_feedback(SILENT));
}

/// The refusal comes before the target checks: with a target the launch
/// would reject silently selected (a dead mob, a friendly player the #444
/// gate refuses, a mob out of range), the press still gets the no-effect
/// answer and nothing else. Fails if the gate runs after target validation.
#[tokio::test]
async fn a_rejected_target_does_not_swallow_the_no_effect_answer() {
    const DEAD_MOB: u32 = 20;
    const FRIEND: u32 = 21;
    const FAR_MOB: u32 = 22;
    let mut mgr = scene();
    for (id, x) in [(DEAD_MOB, 3.0), (FAR_MOB, 500.0)] {
        mgr.create_entity(id, "Castle_CellBlock", [x, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(id).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    }
    mgr.get_entity_mut(DEAD_MOB).unwrap().state_field |= cimmeria_wire::state_field::BSF_DEAD;
    make_player(&mut mgr, FRIEND, [2.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(64);

    for target in [DEAD_MOB, FRIEND, FAR_MOB] {
        assert!(!handle_use_ability(PLAYER, SILENT, target as i32, &tx, &mut mgr).await);
        let msgs = drain(&mut rx);
        assert_eq!(
            calls(&msgs),
            expected_feedback(SILENT),
            "target {target}: the no-effect answer, and only it"
        );
    }
}

/// Control: the same ability with one effect that runs a script commits
/// and sends its timer, so the guard above refuses for the right reason.
#[tokio::test]
async fn the_same_press_with_a_mechanic_commits_and_sends_its_timer() {
    let mut mgr = scene();
    mgr.ability_defs.get_mut(&SILENT).unwrap().effect_ids = vec![FIXTURE_EFFECT];
    let (tx, mut rx) = mpsc::channel(64);

    assert!(handle_use_ability(PLAYER, SILENT, 0, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(sent_timer(&msgs), "{msgs:#?}");
    assert!(!calls(&msgs)
        .iter()
        .any(|(_, idx, _)| *idx == method_idx::ON_ERROR_CODE));
}

/// The gate is for players: an NPC's animation-only attack still fires.
#[tokio::test]
async fn an_npc_cast_with_no_mechanic_still_commits() {
    let mut mgr = scene();
    mgr.create_entity(9, "Castle_CellBlock", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(9).unwrap().abilities.add_ability(SILENT);
    let (tx, _rx) = mpsc::channel(64);

    assert!(handle_use_ability(9, SILENT, 0, &tx, &mut mgr).await);
}

/// The active weapon's own binding is never refused, mechanic or not: the
/// basic attack must not go quiet while AB-03 fills in weapon numbers.
#[tokio::test]
async fn a_weapon_granted_ability_with_no_mechanic_still_commits() {
    let mut mgr = scene();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.abilities.remove_ability(SILENT);
    p.weapon_holstered = false;
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 0,
            item_id: 77,
            clip_size: 0,
            default_ammo_type: 0,
            current_ammo: 0,
            cur_ammo_type: 0,
        },
    );
    mgr.item_event_set_abilities
        .insert((77, EVENT_ITEM_MELEE), SILENT);
    let (tx, _rx) = mpsc::channel(64);

    assert!(handle_use_ability(PLAYER, SILENT, 0, &tx, &mut mgr).await);
}

/// Negative-log guard: one DEBUG `abilities` row per refusal, with the
/// reason and the player's identity, and no WARN from the refusal.
#[tokio::test]
async fn the_refusal_logs_one_debug_row_with_reason_and_identity() {
    let mut mgr = scene();
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert!(!handle_use_ability(PLAYER, SILENT, 0, &tx, &mut mgr).await);
    let all = logs.all();

    let rows: Vec<_> = all
        .iter()
        .filter(|c| c.target == "abilities" && c.has_field("event", "no_mechanics_refused"))
        .collect();
    assert_eq!(rows.len(), 1, "one row per refusal: {all:#?}");
    let row = rows[0];
    assert_eq!(row.level, Level::DEBUG);
    assert!(row.has_field("reason", REASON_NO_MECHANICS), "{row:?}");
    assert!(row.has_field("ability_id", &SILENT.to_string()), "{row:?}");
    assert!(row.has_field("entity_id", &PLAYER.to_string()), "{row:?}");
    assert!(row.has_field("account_id", "901"), "{row:?}");
    assert!(row.has_field("player_id", "101"), "{row:?}");
    assert!(row.has_field("animates", "true"), "{row:?}");
    assert!(
        !all.iter().any(|c| c.level == Level::WARN),
        "a client-pressable refusal must not WARN: {all:#?}"
    );
}

// ── The predicate ───────────────────────────────────────────────────────

fn bare(id: i32) -> AbilityDef {
    let mut def = make_ability(id, 0, 30);
    def.effect_ids = vec![];
    def
}

#[test]
fn no_effect_and_no_binding_has_no_mechanics() {
    let mgr = make_mgr();
    assert!(!ability_has_mechanics(&mgr, &bare(SILENT)));
    assert!(
        !ability_has_mechanics(&mgr, &silent_def()),
        "an event set is not a mechanic"
    );
}

#[test]
fn a_damage_nvp_or_a_script_is_a_mechanic() {
    let mut mgr = make_mgr();
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "20".to_string());
    mgr.effect_defs.insert(
        700_001,
        EffectDef {
            effect_id: 700_001,
            params,
            ..Default::default()
        },
    );
    let mut def = bare(SILENT);
    def.effect_ids = vec![700_001];
    assert!(ability_has_mechanics(&mgr, &def));
    def.effect_ids = vec![FIXTURE_EFFECT];
    assert!(ability_has_mechanics(&mgr, &def));
}

#[test]
fn each_binding_outside_the_effect_rows_is_a_mechanic() {
    let mut mgr = make_mgr();

    // A weapon shot spends a round and carries the loaded ammo's modifier.
    let mut shot = bare(SILENT);
    shot.required_ammo = 1;
    assert!(ability_has_mechanics(&mgr, &shot));

    // Cover Stance is granted through the cover hold (NA22).
    assert!(ability_has_mechanics(&mgr, &bare(COVER_STANCE_ABILITY)));

    mgr.pet_summons = PetSummonCatalog::from_rows([PetSummon {
        ability_id: 2826,
        template_id: 350,
        max_active: 1,
    }]);
    assert!(ability_has_mechanics(&mgr, &bare(2826)));

    mgr.deployable_specs = DeployableCatalog::from_rows([DeployableSpec {
        ability_id: 1012,
        template_id: 400,
        lifetime_effect_id: 5065,
        pulse_effect_id: 5066,
        max_active: 1,
    }]);
    assert!(ability_has_mechanics(&mgr, &bare(1012)));

    mgr.ammo_catalog = AmmoCatalog::from_rows(
        [AmmoModifier {
            ammo_type: 3,
            damage_mult: 1.25,
            penetration_mult: 0.75,
            damage_type: None,
            on_hit_effect_id: None,
            toggle_ability_id: 715,
            beneficial: false,
        }],
        [],
    );
    assert!(ability_has_mechanics(&mgr, &bare(715)));

    // None of them leaks to an unrelated ability.
    assert!(!ability_has_mechanics(&mgr, &bare(SILENT)));
}
