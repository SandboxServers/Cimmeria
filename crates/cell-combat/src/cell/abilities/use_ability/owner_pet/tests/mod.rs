//! Pets PT-08: owner abilities that act on the owner's pet, through
//! `useAbility`.
//!
//! The fixture is the pet world every pet packet uses (`watched_pet_world`:
//! owner 7 with its pet, and a second player 8 beside them), plus the seeded
//! rows of the abilities under test, mirrored here: the flags, cooldowns,
//! warmups, effects, `script_name`s and `effect_nvps` of
//! `db/resources/Abilities/Seed/abilities.sql` and
//! `db/resources/Effects/Seed/effects.sql` / `effect_nvps.sql`. The live-DB
//! file pins the seed to these values.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use cimmeria_cell_world::test_fixtures::watched_pet_world;
pub(super) use cimmeria_cell_world::test_fixtures::{
    PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};
use cimmeria_entity::abilities::{AbilityDef, EffectDef, TCM_AE_RADIUS, TCM_SINGLE};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;

mod holy_warrior;
mod live_db;
mod passive_and_heals;
mod refusals;
mod telemetry;
mod to_the_death;

pub(super) const HOLY_WARRIOR: i32 = 2824;
pub(super) const TO_THE_DEATH: i32 = 2839;
pub(super) const LORDS_CONCENTRATION: i32 = 1650;
pub(super) const REPAIR_PERCENT: i32 = 967;
pub(super) const REPAIR_REGEN: i32 = 968;

pub(super) const E_HOLY_WARRIOR: i32 = 4220;
pub(super) const E_STANCE_REMOVAL: i32 = 4087;
pub(super) const E_DEATH_ACCURACY: i32 = 4121;
pub(super) const E_DEATH_TIMER: i32 = 4119;
pub(super) const E_PET_DEATH: i32 = 4122;
pub(super) const E_CONCENTRATION: i32 = 350;
pub(super) const E_HEAL_PET: i32 = 3211;
pub(super) const E_HEAL_PET_REGEN: i32 = 3230;

/// `TCM_Group` (no constant in `cimmeria_entity`; only the collection
/// method string matters here).
const TCM_GROUP: &str = "TCM_Group";

fn ability(
    id: i32,
    name: &str,
    cooldown: f32,
    warmup: f32,
    flags: u32,
    effect_ids: &[i32],
) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: name.to_string(),
        cooldown,
        warmup,
        flags,
        is_ranged: false,
        min_range: 0,
        max_range: 0,
        target_type_id: 1,
        effect_ids: effect_ids.to_vec(),
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 100.0,
    }
}

fn effect(
    id: i32,
    ability_id: i32,
    script: Option<&str>,
    pulses: (i32, f32),
    tcm: &str,
    nvps: &[(&str, &str)],
) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id,
        script_name: script.map(str::to_string),
        pulse_count: pulses.0,
        pulse_duration: pulses.1,
        target_collection_method: tcm.to_string(),
        params: nvps
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

/// The seeded ability and effect rows under test.
pub(super) fn seed_rows(mgr: &mut SpaceManager) {
    for def in [
        // flags 1560 = Toggled | Response | DoNotActivate_AutoCycle |
        // Deactivate_AutoCycle.
        ability(
            HOLY_WARRIOR,
            "Holy Warrior",
            2.0,
            0.0,
            1560,
            &[E_HOLY_WARRIOR, E_STANCE_REMOVAL],
        ),
        ability(
            TO_THE_DEATH,
            "To The Death",
            30.0,
            2.0,
            16,
            &[E_PET_DEATH, E_DEATH_ACCURACY, E_DEATH_TIMER],
        ),
        ability(
            LORDS_CONCENTRATION,
            "Lord's Concentration",
            30.0,
            2.0,
            145,
            &[E_CONCENTRATION],
        ),
        ability(
            REPAIR_PERCENT,
            "Repair Turret: Percentage",
            10.0,
            2.0,
            1808,
            &[E_HEAL_PET],
        ),
        ability(
            REPAIR_REGEN,
            "Repair Turret: Regenerate",
            30.0,
            2.0,
            1,
            &[E_HEAL_PET_REGEN],
        ),
    ] {
        mgr.ability_defs.insert(def.ability_id, def);
    }
    for def in [
        effect(
            E_HOLY_WARRIOR,
            HOLY_WARRIOR,
            Some("PetStatBuff"),
            (1, 0.0),
            TCM_AE_RADIUS,
            &[("Accuracy", "100"), ("Defense", "-100")],
        ),
        effect(
            E_STANCE_REMOVAL,
            HOLY_WARRIOR,
            None,
            (1, 0.0),
            TCM_SINGLE,
            &[],
        ),
        effect(E_PET_DEATH, TO_THE_DEATH, None, (1, 0.0), TCM_SINGLE, &[]),
        effect(
            E_DEATH_ACCURACY,
            TO_THE_DEATH,
            Some("PetStatBuff"),
            (1, 60.0),
            TCM_SINGLE,
            &[("Accuracy", "400")],
        ),
        effect(
            E_DEATH_TIMER,
            TO_THE_DEATH,
            Some("PetDeathTimer"),
            (1, 60.0),
            TCM_SINGLE,
            &[],
        ),
        effect(
            E_CONCENTRATION,
            LORDS_CONCENTRATION,
            Some("PetStatBuff"),
            (1, 30.0),
            TCM_GROUP,
            &[("InterruptResistance", "50")],
        ),
        effect(
            E_HEAL_PET,
            REPAIR_PERCENT,
            Some("HealPetHealth"),
            (1, 0.0),
            TCM_SINGLE,
            &[("HealPercentage", "20.00")],
        ),
        effect(
            E_HEAL_PET_REGEN,
            REPAIR_REGEN,
            Some("HealPetHealth"),
            (15, 1.0),
            TCM_SINGLE,
            &[("HealPercentage", "5.00")],
        ),
    ] {
        mgr.effect_defs.insert(def.effect_id, def);
    }
}

/// The pet world with the seeded rows, every ability known by the owner.
/// Returns the pet id.
pub(super) fn world() -> (SpaceManager, u32) {
    let (mut mgr, pet) = watched_pet_world();
    seed_rows(&mut mgr);
    let owner = mgr.get_entity_mut(OWNER).expect("owner");
    for id in [
        HOLY_WARRIOR,
        TO_THE_DEATH,
        LORDS_CONCENTRATION,
        REPAIR_PERCENT,
        REPAIR_REGEN,
    ] {
        owner.abilities.add_ability(id);
    }
    (mgr, pet)
}

pub(super) fn stat(mgr: &SpaceManager, entity: u32, id: i32) -> i32 {
    mgr.get_entity(entity)
        .and_then(|e| e.stats.get(id))
        .map_or(i32::MIN, |s| s.cur)
}

pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// `(method_index, args)` of every method sent to `entity`'s own client.
fn to_client(msgs: &[CellToBaseMsg], entity: u32) -> Vec<(u16, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if *entity_id == entity => Some((*method_index, args.clone())),
            _ => None,
        })
        .collect()
}

/// The `onErrorCode` `(InstanceID, ErrorCodeID)` pairs `entity` was sent.
pub(super) fn error_codes(msgs: &[CellToBaseMsg], entity: u32) -> Vec<(i32, u16)> {
    to_client(msgs, entity)
        .into_iter()
        .filter(|(m, _)| *m == method_idx::ON_ERROR_CODE)
        .map(|(_, a)| {
            (
                i32::from_le_bytes([a[1], a[2], a[3], a[4]]),
                u16::from_le_bytes([a[5], a[6]]),
            )
        })
        .collect()
}

/// Whether `entity` was sent the `CHAN_FEEDBACK` line `text`.
pub(super) fn got_line(msgs: &[CellToBaseMsg], entity: u32, text: &str) -> bool {
    let want = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    to_client(msgs, entity)
        .into_iter()
        .any(|(m, a)| m == method_idx::ON_PLAYER_COMMUNICATION && a == want)
}

/// Whether `witness` was sent an `onStatUpdate` for `entity`.
pub(super) fn stat_update_to(msgs: &[CellToBaseMsg], witness: u32, entity: u32) -> bool {
    msgs.iter().any(|m| {
        matches!(m, CellToBaseMsg::WitnessEntityMethod {
            witness_id, entity_id, method_index, ..
        } if *witness_id == witness && *entity_id == entity
            && *method_index == method_idx::ON_STAT_UPDATE)
    })
}

/// Whether the owner's cooldown for `ability` is running.
pub(super) fn on_cooldown(mgr: &SpaceManager, ability: i32) -> bool {
    mgr.get_entity(OWNER)
        .is_some_and(|e| e.abilities.is_on_cooldown(ability))
}

/// Clear the owner's cooldowns so a test can press again at once.
pub(super) fn ready_again(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(OWNER)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
}

/// Past a 2 s owner warmup.
pub(super) fn after_warmup() -> Instant {
    Instant::now() + Duration::from_millis(2_100)
}
