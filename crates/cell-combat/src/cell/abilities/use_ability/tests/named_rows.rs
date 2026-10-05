//! Rule 6 (named telemetry, NT-20): the combat rows people read most name
//! every ID they carry. A refused press names the caster, its target and
//! the ability; a hit names both sides, the ability and the effect; a kill
//! names the attacker, the victim and the ability. Removing a name field
//! from one of those events fails its test here.
//!
//! The NameBook is the process's global one, so each test stores a book
//! holding the fixture's names and puts an empty one back after (the
//! pattern of `fire_los::the_refusal_rows_name_error_39`).

use std::time::Duration;

use tracing::Level;

use super::warmup::{warmup_mgr_in, INSTANT_ABILITY};
use super::*;
use crate::test_support::{Captured, LogCapture};

const CASTER: &str = "Teal'c";
const LOGIN: &str = "tealc_login";
const NPC_TEMPLATE: i32 = 7701;
const NPC_NAME_ID: i64 = 77_010;
const NPC: &str = "Jaffa Guard";
const ABILITY: &str = "Staff Blast";
const EFFECT: &str = "Staff Burn";

/// Player 1 (stamped with a character and a login name) and hostile NPC 2
/// (named through its template) in `world`, with the book holding the
/// ability's, the effect's and the NPC's names.
fn named_world(world: &str) -> SpaceManager {
    let mut book = cimmeria_names::NameBook::empty();
    book.insert(
        cimmeria_names::Table::Abilities,
        i64::from(INSTANT_ABILITY),
        ABILITY,
    );
    book.insert(cimmeria_names::Table::Effects, 500, EFFECT);
    book.insert(
        cimmeria_names::Table::Templates,
        i64::from(NPC_TEMPLATE),
        "jaffa_guard",
    );
    book.insert_template_name_id(i64::from(NPC_TEMPLATE), NPC_NAME_ID);
    book.insert(cimmeria_names::Table::Texts, NPC_NAME_ID, NPC);
    cimmeria_names::global().store(book);

    let mut mgr = warmup_mgr_in(world);
    let player = mgr.get_entity_mut(1).unwrap();
    player.account_id = Some(6);
    player.stamp_log_names(Some(CASTER), Some(LOGIN));
    mgr.get_entity_mut(2).unwrap().template_id = Some(NPC_TEMPLATE);
    mgr
}

fn clear_book() {
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());
}

fn row(rows: &[Captured], target: &str, event: &str) -> Captured {
    rows.iter()
        .find(|r| r.target == target && r.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no {target} {event} row in {rows:#?}"))
}

fn assert_names(row: &Captured, pairs: &[(&str, &str)]) {
    for (key, want) in pairs {
        assert!(row.has_field(key, want), "{key} = {want} missing: {row:#?}");
    }
}

/// Ability reject: a press on cooldown writes the gate's `launch refused`
/// row and the `ability_refused` metric row, each naming who pressed what.
#[tokio::test]
async fn a_refused_press_names_the_caster_its_target_and_the_ability() {
    let mut mgr = named_world("NT20_Refused");
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .start_ability_cooldown(INSTANT_ABILITY, Duration::from_secs(60));
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    assert!(!handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);
    clear_book();
    let rows = logs.all();

    let gate = row(&rows, "abilities", "use_ability_on_cooldown");
    assert_names(
        &gate,
        &[
            ("account_name", LOGIN),
            ("player_name", CASTER),
            ("entity_name", CASTER),
            ("caster_name", CASTER),
            // The gate row names the ability from its definition.
            ("ability_name", "charged"),
            ("wire_target_name", NPC),
            ("target_name", NPC),
        ],
    );
    let refused = row(&rows, "abilities", "ability_refused");
    assert_names(
        &refused,
        &[
            ("player_name", CASTER),
            ("entity_name", CASTER),
            ("ability_name", ABILITY),
        ],
    );
}

/// Ability use and damage: an instant hit's `ability_launched` row names the
/// caster and its target, and its `nvp_damage_resolved` row names both
/// sides, the ability and the effect that dealt the damage.
#[tokio::test]
async fn a_hit_names_both_sides_the_ability_and_the_effect() {
    let mut mgr = named_world("NT20_Hit");
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    assert!(handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);
    clear_book();
    let rows = logs.all();

    let hit = row(&rows, "abilities.effect", "nvp_damage_resolved");
    assert_names(
        &hit,
        &[
            ("account_name", LOGIN),
            ("player_name", CASTER),
            ("entity_name", CASTER),
            ("target_name", NPC),
            ("ability_name", ABILITY),
            ("effect_name", EFFECT),
        ],
    );
    let launched = row(&rows, "abilities", "ability_launched");
    assert_names(
        &launched,
        &[
            ("account_name", LOGIN),
            ("player_name", CASTER),
            ("entity_name", CASTER),
            ("target_name", NPC),
            ("wire_target_name", NPC),
            ("ability_name", "charged"),
        ],
    );
    let planned = row(&rows, "abilities.effect", "effect_planned");
    assert_names(
        &planned,
        &[
            ("entity_name", CASTER),
            ("target_name", NPC),
            ("ability_name", ABILITY),
            ("effect_name", EFFECT),
        ],
    );
}

/// Death: the killing blow's `target_killed` row names the attacker, the
/// victim and the ability.
#[tokio::test]
async fn a_kill_names_the_attacker_the_victim_and_the_ability() {
    let mut mgr = named_world("NT20_Kill");
    if let Some(stat) = mgr
        .get_entity_mut(2)
        .unwrap()
        .stats
        .get_mut(cimmeria_entity::stats::HEALTH)
    {
        stat.update(0, 1, 100_000);
    }
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    assert!(handle_use_ability(1, INSTANT_ABILITY, 2, &tx, &mut mgr).await);
    clear_book();
    let rows = logs.all();

    let killed = rows
        .iter()
        .find(|r| r.level == Level::INFO && r.has_field("event", "target_killed"))
        .cloned()
        .unwrap_or_else(|| panic!("no target_killed row: {rows:#?}"));
    assert_names(
        &killed,
        &[
            ("attacker_name", CASTER),
            ("target_name", NPC),
            ("ability_name", ABILITY),
        ],
    );
}
