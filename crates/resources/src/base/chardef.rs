//! The CharDef identity table: what a client's `CharDefId` (1-23) means.
//!
//! The client's character creator sends a `CharDefId`, and this table turns
//! it into the four identity values the `sgw_player` row stores: alignment,
//! archetype, gender and bodyset. It is the client contract (the ids are the
//! client's cooked CharDef rows), so it stays in code.
//!
//! **Where a character starts is not here.** World, spawn point, start
//! level, kit and the debug-kit flag are the data-driven start profile
//! (`resources.char_creation`, [`super::start_profiles`], Class Start v6
//! CS-02). The identity columns of that table must agree with this one; the
//! live-DB test `chardef_identity_matches_the_start_profile_rows_live_db`
//! (in `start_profiles`) fails when they drift.

/// What a `CharDefId` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharDefIdentity {
    /// `sgw_player.alignment`: 1 = Praxis, 2 = SGU (`EAlignment` ordinal).
    pub alignment: i32,
    /// `sgw_player.archetype`: the `EArchetype` ordinal (1 = Soldier).
    pub archetype: i32,
    /// `sgw_player.gender`: `EGender` + 1 (1 = male, 2 = female), because
    /// the column's constraint requires 1-3.
    pub gender: i32,
    /// The bodyset in the doubled `BS_X.BS_X` form the varchar(64) column
    /// stores.
    pub bodyset: &'static str,
}

/// The identity of `CharDefId` `id`, or `None` for an id the client never
/// sends.
pub fn chardef_lookup(id: i32) -> Option<CharDefIdentity> {
    const PRAXIS: i32 = 1;
    const SGU: i32 = 2;
    const MALE: i32 = 1;
    const FEMALE: i32 = 2;
    let (alignment, archetype, gender, bodyset) = match id {
        1 => (PRAXIS, 1, MALE, "BS_HumanMale.BS_HumanMale"), // Praxis Soldier
        2 => (SGU, 1, MALE, "BS_HumanMale.BS_HumanMale"),    // SGU Soldier
        3 => (PRAXIS, 2, MALE, "BS_HumanMale.BS_HumanMale"), // Praxis Commando
        4 => (SGU, 2, MALE, "BS_HumanMale.BS_HumanMale"),    // SGU Commando
        5 => (PRAXIS, 4, MALE, "BS_HumanMale.BS_HumanMale"), // Praxis Archeologist
        6 => (SGU, 4, MALE, "BS_HumanMale.BS_HumanMale"),    // SGU Archeologist
        7 => (PRAXIS, 8, MALE, "BS_JaffaMale.BS_JaffaMale"), // Praxis (Loyalist) Jaffa
        8 => (SGU, 7, MALE, "BS_JaffaMale.BS_JaffaMale"),    // SGU Shol'va (Free Jaffa)
        9 => (SGU, 5, MALE, "BS_Asgard.BS_Asgard"),          // SGU Asgard
        10 => (PRAXIS, 6, MALE, "BS_GoauldMale.BS_GoauldMale"), // Praxis Goa'uld
        11 => (PRAXIS, 1, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        12 => (SGU, 1, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        13 => (PRAXIS, 2, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        14 => (SGU, 2, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        15 => (PRAXIS, 4, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        16 => (SGU, 4, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        17 => (PRAXIS, 8, FEMALE, "BS_JaffaFemale.BS_JaffaFemale"),
        18 => (SGU, 7, FEMALE, "BS_JaffaFemale.BS_JaffaFemale"),
        19 => (PRAXIS, 6, FEMALE, "BS_GoauldFemale.BS_GoauldFemale"),
        20 => (PRAXIS, 3, MALE, "BS_HumanMale.BS_HumanMale"), // Praxis Scientist
        21 => (SGU, 3, MALE, "BS_HumanMale.BS_HumanMale"),    // SGU Scientist
        22 => (PRAXIS, 3, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        23 => (SGU, 3, FEMALE, "BS_HumanFemale.BS_HumanFemale"),
        _ => return None,
    };
    Some(CharDefIdentity {
        alignment,
        archetype,
        gender,
        bodyset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chardef_lookup_alignment_values() {
        for (id, alignment) in [(20, 1), (22, 1), (21, 2), (23, 2)] {
            assert_eq!(chardef_lookup(id).unwrap().alignment, alignment, "{id}");
        }
    }

    #[test]
    fn chardef_lookup_bodyset_doubled_format() {
        // All bodysets use "BS_X.BS_X" doubled format for the DB varchar(64) column.
        for id in 1..=23 {
            let bodyset = chardef_lookup(id).unwrap().bodyset;
            let (a, b) = bodyset.split_once('.').expect("doubled form");
            assert_eq!(a, b, "chardef {id} bodyset '{bodyset}'");
        }
    }

    #[test]
    fn chardef_lookup_covers_exactly_the_client_ids() {
        for id in 1..=23 {
            assert!(chardef_lookup(id).is_some(), "chardef_lookup({id})");
        }
        for id in [0, 24, -1] {
            assert_eq!(chardef_lookup(id), None, "{id}");
        }
    }

    #[test]
    fn free_jaffa_asgard_and_goauld_identities() {
        // The three non-human SGU/Praxis races the start profiles single out.
        for (id, archetype, alignment) in [(8, 7, 2), (18, 7, 2), (9, 5, 2), (10, 6, 1), (19, 6, 1)]
        {
            let c = chardef_lookup(id).unwrap();
            assert_eq!((c.archetype, c.alignment), (archetype, alignment), "{id}");
        }
    }
}
