//! Ammo campaign AM-06: `.giveammo` and `.infiniteammo` (cell half).
//!
//! Filter prefix: `am06_`.
//!
//! Bug shapes: a non-GM reaching either handler; a grant forwarded for a
//! misparsed, ambiguous or free type, a non-positive or unclamped count,
//! or aimed at the wrong entity; a bad argument that changes nothing
//! *silently*; the infinite-ammo switch keyed by entity instead of
//! character, or not toggled at all; refusals missing from telemetry.

use cimmeria_cell_catalog::cell::spawner::AmmoCatalog;
use cimmeria_entity::ammo_infinite;
use cimmeria_entity::ammo_type::{
    BULLET_ARMOR_PIERCING, BULLET_EMP, BULLET_HOLLOW_POINT, DART_EMP,
};
use tracing::Level;

use super::decode_feedback;
use super::pt07_giveability::{console, say, world, CALLER};
use crate::cell::console::gm::give_ammo::{parse_ammo_type, MAX_ROUNDS};
use crate::cell::messages::{CellToBaseMsg, GmGiveAmmo};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const WITNESS: u32 = 2;

/// `world(level)` with the AM-F reserve items for HP, AP, Bullet_EMP and
/// Dart_EMP (Dagger_Metallic is special but deliberately unmapped).
fn ammo_world(access_level: u32) -> SpaceManager {
    let (mut mgr, _npc) = world(access_level);
    mgr.ammo_catalog = AmmoCatalog::from_rows(
        [],
        [
            (BULLET_ARMOR_PIERCING, 9000),
            (BULLET_HOLLOW_POINT, 9001),
            (BULLET_EMP, 9003),
            (DART_EMP, 9008),
        ],
    );
    mgr
}

fn grants(msgs: &[CellToBaseMsg]) -> Vec<GmGiveAmmo> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmGiveAmmo(g) => Some(*g),
            _ => None,
        })
        .collect()
}

fn lines(msgs: &[CellToBaseMsg]) -> Vec<String> {
    msgs.iter().filter_map(decode_feedback).collect()
}

#[test]
fn am06_parse_ammo_type_accepts_ordinals_labels_short_names_and_item_ids() {
    let items = |i: i32| (i == 9001).then_some(BULLET_HOLLOW_POINT);
    for arg in [
        "3",
        "Bullet_Hollow_Point",
        "bullet_hollow_point",
        "hollow_point",
        "HollowPoint",
        "hp",
        "9001",
    ] {
        assert_eq!(
            parse_ammo_type(arg, items),
            Ok(BULLET_HOLLOW_POINT),
            "{arg}"
        );
    }
    assert_eq!(parse_ammo_type("ap", items), Ok(BULLET_ARMOR_PIERCING));
    assert_eq!(parse_ammo_type("dart_emp", items), Ok(DART_EMP));
    let amb = parse_ammo_type("emp", items).unwrap_err();
    assert!(
        amb.contains("Bullet_EMP") && amb.contains("Dart_EMP"),
        "{amb}"
    );
    assert!(parse_ammo_type("banana", items).is_err());
    assert!(
        parse_ammo_type("24", items).is_err(),
        "past the enum, not an item"
    );
    assert!(parse_ammo_type("9002", items).is_err(), "not a mapped item");
}

/// A player or trial GM gets the generic refusal for every name, including
/// the aliases, and neither handler runs.
#[tokio::test]
async fn am06_non_gm_is_refused_with_feedback_and_nothing_changes() {
    for access_level in [0, 1] {
        for line in [
            ".giveammo hp 10",
            ".gmgiveammo hp 10",
            ".infiniteammo on",
            ".gmsetinfiniteammo on",
        ] {
            let mut mgr = ammo_world(access_level);
            let msgs = say(&mut mgr, line).await;
            let cmd = line[1..].split_whitespace().next().unwrap();
            assert_eq!(
                lines(&msgs),
                vec![format!(".{cmd} is a GM command; you do not have GM rights")],
                "level {access_level} {line}"
            );
            assert!(grants(&msgs).is_empty(), "{line}: nothing reaches the base");
            assert!(!ammo_infinite::is_on(71), "{line}: switch untouched");
        }
    }
}

/// The grant goes to the base with the resolved type, the catalog's item
/// and the exact count, for the caller; no optimistic line (the base
/// answers after the commit).
#[tokio::test]
async fn am06_giveammo_forwards_the_resolved_grant_for_the_caller() {
    let mut mgr = ammo_world(2);
    for line in [".giveammo hollowpoint 700", ".gmgiveammo 3 700"] {
        let msgs = console(&mut mgr, None, line).await;
        assert_eq!(
            grants(&msgs),
            vec![GmGiveAmmo {
                entity_id: CALLER,
                player_id: 71,
                gm_entity_id: CALLER,
                gm_player_id: 71,
                gm_account_id: Some(601),
                ammo_type: BULLET_HOLLOW_POINT,
                item_id: 9001,
                rounds: 700,
            }],
            "{line}"
        );
        assert!(lines(&msgs).is_empty(), "{line}: {:?}", lines(&msgs));
    }
}

#[tokio::test]
async fn am06_giveammo_grants_a_selected_player_and_clamps_the_count() {
    let mut mgr = ammo_world(2);
    let msgs = console(&mut mgr, Some(WITNESS), ".giveammo ap 99999").await;
    let g = grants(&msgs);
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].entity_id, g[0].player_id), (WITNESS, 72));
    assert_eq!((g[0].gm_entity_id, g[0].gm_player_id), (CALLER, 71));
    assert_eq!(
        (g[0].ammo_type, g[0].item_id),
        (BULLET_ARMOR_PIERCING, 9000)
    );
    assert_eq!(g[0].rounds, MAX_ROUNDS);
    assert_eq!(
        lines(&msgs),
        vec![format!(".giveammo: 99999 rounds clamped to {MAX_ROUNDS}")]
    );
}

/// Every bad argument answers visibly, sends nothing, and logs a WARN on
/// `ammo` with its reason and the caller's identity.
#[tokio::test]
async fn am06_giveammo_bad_arguments_are_refused_visibly_and_logged() {
    for (line, reason, says) in [
        (
            ".giveammo banana 10",
            "unknown_ammo_type",
            "no ammo type named 'banana'",
        ),
        (".giveammo emp 10", "unknown_ammo_type", "ambiguous"),
        (
            ".giveammo 99 10",
            "unknown_ammo_type",
            "neither an ammo type",
        ),
        (
            ".giveammo Bullet_Default 10",
            "not_special_ammo",
            "free default ammo",
        ),
        (
            ".giveammo dagger_metallic 10",
            "no_reserve_item",
            "no reserve item",
        ),
        (".giveammo hp 0", "bad_quantity", "positive whole number"),
        (".giveammo hp -5", "bad_quantity", "positive whole number"),
        (".giveammo hp lots", "bad_quantity", "got 'lots'"),
    ] {
        let mut mgr = ammo_world(2);
        let capture = LogCapture::install();
        let msgs = console(&mut mgr, None, line).await;
        assert!(grants(&msgs).is_empty(), "{line}");
        let fb = lines(&msgs);
        assert_eq!(fb.len(), 1, "{line}: {fb:?}");
        assert!(
            fb[0].starts_with(".giveammo: ") && fb[0].contains(says),
            "{line}: {fb:?}"
        );
        let hits: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN
                    && c.target == "ammo"
                    && c.has_field("event", "gm_give_ammo")
                    && c.has_field("reason", reason)
            })
            .collect();
        assert_eq!(hits.len(), 1, "{line}: {:#?}", capture.all());
        assert!(
            hits[0].has_field("account_id", "601") && hits[0].has_field("player_id", "71"),
            "{line}: {:#?}",
            hits[0]
        );
    }
    // Arity is the framework's, and still answers.
    let mut mgr = ammo_world(2);
    let msgs = console(&mut mgr, None, ".giveammo hp").await;
    assert!(grants(&msgs).is_empty());
    assert!(
        lines(&msgs)[0].contains("not enough arguments"),
        "{:?}",
        lines(&msgs)
    );
}

/// On, unchanged, off, a bad argument and a bare query: the switch follows
/// the character (not the entity), and every use answers. One test, one
/// sentinel character, because the switch is process-wide.
#[tokio::test]
async fn am06_infiniteammo_toggles_by_character_and_always_answers() {
    const CHARACTER: i32 = 0x7AA0_0611;
    let mut mgr = ammo_world(2);
    mgr.get_entity_mut(CALLER).unwrap().player_id = Some(CHARACTER);
    assert!(!ammo_infinite::is_on(CHARACTER));

    let capture = LogCapture::install();
    let fb = lines(&console(&mut mgr, None, ".infiniteammo on").await);
    assert!(ammo_infinite::is_on(CHARACTER), "set by character id");
    assert!(
        fb[0].starts_with("infiniteammo set: ON") && fb[0].contains("clip still empties"),
        "{fb:?}"
    );
    let toggled: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::INFO
                && c.target == "ammo"
                && c.has_field("event", "gm_infinite_ammo_toggled")
        })
        .collect();
    assert_eq!(toggled.len(), 1, "{:#?}", capture.all());
    assert!(toggled[0].has_field("on", "true") && toggled[0].has_field("account_id", "601"));
    drop(capture);

    let fb = lines(&console(&mut mgr, None, ".gmsetinfiniteammo 1").await);
    assert!(fb[0].starts_with("infiniteammo unchanged: ON"), "{fb:?}");

    let fb = lines(&console(&mut mgr, None, ".infiniteammo maybe").await);
    assert!(fb[0].contains("expected 'on' or 'off'"), "{fb:?}");
    assert!(
        ammo_infinite::is_on(CHARACTER),
        "a bad argument changes nothing"
    );

    let fb = lines(&console(&mut mgr, None, ".infiniteammo").await);
    assert!(
        fb[0].starts_with("infiniteammo: ON"),
        "a bare query reports: {fb:?}"
    );

    let fb = lines(&console(&mut mgr, None, ".infiniteammo off").await);
    assert!(!ammo_infinite::is_on(CHARACTER));
    assert!(fb[0].starts_with("infiniteammo set: OFF"), "{fb:?}");
}

/// A selected player is the subject; the GM's own switch is untouched.
#[tokio::test]
async fn am06_infiniteammo_applies_to_a_selected_player() {
    const SUBJECT: i32 = 0x7AA0_0612;
    let mut mgr = ammo_world(2);
    mgr.get_entity_mut(CALLER).unwrap().player_id = Some(0x7AA0_0613);
    mgr.get_entity_mut(WITNESS).unwrap().player_id = Some(SUBJECT);
    let fb = lines(&console(&mut mgr, Some(WITNESS), ".infiniteammo on").await);
    assert!(ammo_infinite::is_on(SUBJECT));
    assert!(!ammo_infinite::is_on(0x7AA0_0613));
    assert!(fb[0].contains("(for entity 2)"), "{fb:?}");
    ammo_infinite::set(SUBJECT, false);
}
