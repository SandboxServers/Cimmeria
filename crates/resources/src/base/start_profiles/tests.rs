//! Profile validation and lookups, on the seed-shaped fixture.

use super::fixture::{seeded, CELLBLOCK_START, DAKARA_E1_START, LEGACY_KIT, SGC_W1_START};
use super::*;

const PRAXIS: i32 = 1;
const SGU: i32 = 2;

#[test]
fn every_seeded_profile_is_valid_and_starts_at_level_1() {
    let profiles = seeded();
    assert_eq!(profiles.profiles().len(), 23);
    for p in profiles.profiles() {
        assert_eq!(p.problems(), vec![], "{}", p.profile_id);
        assert_eq!(p.start_level, 1, "{}", p.profile_id);
        assert!(!p.debug_kit, "no seeded profile sets the debug kit (L2)");
    }
}

/// Free Jaffa go home to Dakara_E1, SGU humans and the Asgard holding state
/// to SGC_W1, Praxis (Goa'uld holding state included) to the Cellblock. The
/// old two-world rule sent a Free Jaffa to SGC_W1.
#[test]
fn home_for_follows_the_profile_of_alignment_and_archetype() {
    let profiles = seeded();
    let home = |alignment, archetype| {
        let p = profiles.home_for(alignment, archetype).unwrap();
        (p.world.as_str(), p.position)
    };
    assert_eq!(home(SGU, 7), ("Dakara_E1", DAKARA_E1_START));
    for archetype in [1, 2, 3, 4, 5] {
        assert_eq!(
            home(SGU, archetype),
            ("SGC_W1", SGC_W1_START),
            "{archetype}"
        );
    }
    for archetype in [1, 2, 3, 4, 6, 8] {
        assert_eq!(
            home(PRAXIS, archetype),
            ("Castle_CellBlock", CELLBLOCK_START),
            "{archetype}"
        );
    }
    // A hand-made row: an alignment no profile has takes the archetype's
    // first profile, an archetype no profile has the alignment's; with
    // neither there is no home.
    assert_eq!(home(0, 1).0, "Castle_CellBlock");
    assert_eq!(home(0, 7).0, "Dakara_E1");
    assert_eq!(home(SGU, 0).0, "SGC_W1");
    assert!(profiles.home_for(9, 0).is_none());
}

#[test]
fn start_position_names_every_start_world_and_nothing_else() {
    let profiles = seeded();
    assert_eq!(
        profiles.start_position("castle_cellblock"),
        Some(CELLBLOCK_START)
    );
    assert_eq!(profiles.start_position("SGC_W1"), Some(SGC_W1_START));
    assert_eq!(profiles.start_position("Dakara_E1"), Some(DAKARA_E1_START));
    assert_eq!(profiles.start_position("Harset"), None);
    assert_eq!(
        profiles.start_worlds(),
        vec!["Castle_CellBlock", "SGC_W1", "Dakara_E1"]
    );
}

/// Canonical profiles reset to their own starters (Free Jaffa's racial core
/// and signature), holding states to the legacy kit, and a debug-kit
/// character gets the debug kit on top. An archetype with no profile is
/// `None` (the reset refuses).
#[test]
fn reset_abilities_per_profile() {
    let profiles = seeded();
    assert_eq!(profiles.reset_abilities(1, false), Some(vec![]));
    assert_eq!(
        profiles.reset_abilities(7, false),
        Some(vec![597, 1218, 1984])
    );
    assert_eq!(
        profiles.reset_abilities(5, false),
        Some(LEGACY_KIT.to_vec())
    );
    assert_eq!(
        profiles.reset_abilities(6, false),
        Some(LEGACY_KIT.to_vec())
    );
    assert_eq!(profiles.reset_abilities(2, true), Some(LEGACY_KIT.to_vec()));
    assert_eq!(
        profiles.reset_abilities(7, true),
        Some(vec![592, 594, 597, 1218, 1646, 1984])
    );
    assert_eq!(profiles.reset_abilities(0, false), None);
}

fn profile() -> StartProfile {
    seeded().by_char_def(3).unwrap().clone()
}

#[test]
fn problems_name_each_bad_field() {
    let mut p = profile();
    p.world = " ".into();
    p.profile_id.clear();
    p.start_level = 0;
    p.position = [f32::NAN, 0.0, 0.0];
    p.items.push(KitItem {
        item_id: 55,
        stack_size: 0,
    });
    let reasons: Vec<_> = p.problems().iter().map(ProfileProblem::reason).collect();
    assert_eq!(
        reasons,
        vec![
            "empty_profile_id",
            "empty_world",
            "non_finite_position",
            "start_level_out_of_range",
            "non_positive_stack_size",
        ]
    );
    let mut p = profile();
    p.position = [0.0; 3];
    p.start_level = MAX_START_LEVEL + 1;
    let reasons: Vec<_> = p.problems().iter().map(ProfileProblem::reason).collect();
    assert_eq!(reasons, vec!["origin_position", "start_level_out_of_range"]);
}

/// The universal kit must not come back on a canonical profile (OD-CS01,
/// OD-CS04); a holding state may carry it.
#[test]
fn a_canonical_profile_with_a_legacy_kit_row_is_a_problem() {
    let mut p = profile();
    p.abilities.push(KitAbility {
        ability_id: 592,
        source: KitSource::LegacyKit,
    });
    assert_eq!(
        p.problems(),
        vec![ProfileProblem::LegacyKitOnCanonicalProfile { ability_id: 592 }]
    );
    p.start_state = StartState::NonCanonicalBlockedLegacy;
    assert_eq!(p.problems(), vec![]);
}

#[test]
fn column_text_round_trips_and_unknown_values_are_refused() {
    for k in KitSource::ALL {
        assert_eq!(KitSource::try_from(k.as_str()), Ok(k));
    }
    assert!(
        KitSource::try_from("gm").is_err(),
        "creation never writes gm"
    );
    assert!(KitSource::try_from("").is_err());
    for s in [StartState::Canonical, StartState::NonCanonicalBlockedLegacy] {
        assert_eq!(StartState::try_from(s.as_str()), Ok(s));
    }
    assert!(StartState::try_from("canonical").is_err());
    assert_eq!(KitSource::LegacyKit.provenance_kind(), None);
    assert_eq!(KitSource::Signature.provenance_kind(), Some("signature"));
}

/// A start level is the profile's own column: nothing in the loader or the
/// profile reads `resources.missions` (preflight finding: v6's "Free Jaffa
/// level 3" came from the Dakara missions' seeded levels). Fails if the
/// loader is changed to derive a level from a mission row.
#[test]
fn no_start_level_is_derived_from_missions() {
    for (file, src) in [
        ("load.rs", include_str!("load.rs")),
        ("mod.rs", include_str!("mod.rs")),
    ] {
        let lower = src.to_ascii_lowercase();
        assert!(
            !lower.contains("resources.missions") && !lower.contains("mission_level"),
            "start_profiles/{file} must not read a level from the missions table"
        );
    }
}
