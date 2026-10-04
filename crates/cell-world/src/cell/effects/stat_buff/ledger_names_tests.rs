//! Rule 6 (named telemetry, NT-20): a timed effect's `stat_buff_applied`
//! and `stat_buff_removed` rows name the invoker, the target, the effect and
//! the ability next to their IDs. The expiry row names the invoker from the
//! entry's snapshot, so it still does after the invoker's slot is reused.
//! Removing a name field from either event fails these tests.

use std::time::Instant;

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ACCURACY;

use super::StatBuffRemoval;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture};

const PLAYER: u32 = 1;
const NPC: u32 = 2;
const TEMPLATE: i32 = 7702;
const NAME_ID: i64 = 77_020;
const EFFECT: i32 = 7100;
const ABILITY: i32 = 7200;

/// A player invoker named "Teal'c" and an NPC target named through its
/// template, with the book holding the effect's and the ability's names.
fn world() -> SpaceManager {
    let mut book = cimmeria_names::NameBook::empty();
    book.insert(
        cimmeria_names::Table::Effects,
        i64::from(EFFECT),
        "Staff Burn",
    );
    book.insert(
        cimmeria_names::Table::Abilities,
        i64::from(ABILITY),
        "Staff Blast",
    );
    book.insert(
        cimmeria_names::Table::Templates,
        i64::from(TEMPLATE),
        "jaffa_guard",
    );
    book.insert_template_name_id(i64::from(TEMPLATE), NAME_ID);
    book.insert(cimmeria_names::Table::Texts, NAME_ID, "Jaffa Guard");
    cimmeria_names::global().store(book);

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(NPC, "Agnos", [12.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.account_id = Some(601);
    p.player_id = Some(71);
    p.stamp_log_names(Some("Teal'c"), Some("tealc_login"));
    mgr.get_entity_mut(NPC).unwrap().template_id = Some(TEMPLATE);
    mgr
}

fn spec() -> TimedEffectSpec {
    TimedEffectSpec {
        cast_id: None,
        effect_id: EFFECT,
        ability_id: ABILITY,
        invoker_id: PLAYER,
        effect_flags: 0,
        moniker_ids: vec![],
        stats: vec![(ACCURACY, 200)],
        absorb: Vec::new(),
        state_flags: 0,
        duration_secs: Some(15.0),
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
    }
}

fn event(rows: &[Captured], event: &str) -> Captured {
    rows.iter()
        .find(|r| r.target == "abilities" && r.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no {event} row in {rows:#?}"))
}

fn assert_named(row: &Captured) {
    for (key, want) in [
        ("account_name", "tealc_login"),
        ("player_name", "Teal'c"),
        ("entity_name", "Teal'c"),
        ("source_name", "Teal'c"),
        ("target_name", "Jaffa Guard"),
        ("effect_name", "Staff Burn"),
        ("ability_name", "Staff Blast"),
    ] {
        assert!(row.has_field(key, want), "{key} = {want} missing: {row:#?}");
    }
}

/// Effect apply.
#[test]
fn an_applied_effect_row_names_invoker_target_effect_and_ability() {
    let mut mgr = world();
    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    mgr.apply_timed_effect(NPC, spec(), Instant::now())
        .expect("the buff applies");
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());
    assert_named(&event(&logs.all(), "stat_buff_applied"));
}

/// Effect expire: the row names the invoker from the entry's snapshot, so a
/// player who left (and whose entity id now names nobody) is still named.
#[test]
fn an_expired_effect_row_names_the_invoker_after_it_left() {
    let mut mgr = world();
    mgr.apply_timed_effect(NPC, spec(), Instant::now())
        .expect("the buff applies");
    mgr.destroy_entity(PLAYER);
    let logs = LogCapture::install();
    tracing::callsite::rebuild_interest_cache();
    let removed = mgr.remove_timed_effects(NPC, StatBuffRemoval::Expired, |_| true);
    cimmeria_names::global().store(cimmeria_names::NameBook::empty());
    assert_eq!(removed.len(), 1);
    let row = event(&logs.all(), "stat_buff_removed");
    assert!(row.has_field("reason", "expired"), "{row:#?}");
    assert_named(&row);
}
