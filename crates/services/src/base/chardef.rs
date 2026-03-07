/// Given a CharDefId (1-23), return (alignment, archetype, gender, bodyset, world_location, pos_x, pos_y, pos_z).
/// Returns None for unknown IDs.
///
/// Derived from `db/resources/Archetypes/Seed/char_creation.sql`.
///
/// Starting coordinates:
/// - Praxis (Castle_CellBlock): (-334.231, 73.472, -228.026)
/// - SGU (SGC_W1): (201.5, 1.31, 49.724)
pub(crate) fn chardef_lookup(id: i32) -> Option<(i32, i32, i32, &'static str, &'static str, f32, f32, f32)> {
    // (alignment, archetype, gender, bodyset, starting_world, pos_x, pos_y, pos_z)
    // Alignment: 1=Praxis, 2=SGU
    // Gender: 0=Male, 1=Female (EGender enum: GENDER_Male=0, GENDER_Female=1)
    //   DB constraint requires 1-3, so we store gender+1: 1=Male, 2=Female
    // Bodyset uses doubled format: "BS_X.BS_X" (matches DB varchar(64) column)
    const PRAXIS_POS: (f32, f32, f32) = (-334.231, 73.472, -228.026);
    const SGU_POS: (f32, f32, f32) = (201.5, 1.31, 49.724);

    match id {
        1  => Some((1, 1, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Soldier Male
        2  => Some((2, 1, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Soldier Male
        3  => Some((1, 2, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Commando Male
        4  => Some((2, 2, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Commando Male
        5  => Some((1, 4, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Archeologist Male
        6  => Some((2, 4, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Archeologist Male
        7  => Some((1, 8, 1, "BS_JaffaMale.BS_JaffaMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Jaffa Male
        8  => Some((2, 7, 1, "BS_JaffaMale.BS_JaffaMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Shol'va Male
        9  => Some((2, 5, 1, "BS_Asgard.BS_Asgard",           "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Asgard Male
        10 => Some((1, 6, 1, "BS_GoauldMale.BS_GoauldMale",   "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Goa'uld Male
        11 => Some((1, 1, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Soldier Female
        12 => Some((2, 1, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Soldier Female
        13 => Some((1, 2, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Commando Female
        14 => Some((2, 2, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Commando Female
        15 => Some((1, 4, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Archeologist Female
        16 => Some((2, 4, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Archeologist Female
        17 => Some((1, 8, 2, "BS_JaffaFemale.BS_JaffaFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Jaffa Female
        18 => Some((2, 7, 2, "BS_JaffaFemale.BS_JaffaFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Shol'va Female
        19 => Some((1, 6, 2, "BS_GoauldFemale.BS_GoauldFemale","Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Goa'uld Female
        20 => Some((1, 3, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Scientist Male
        21 => Some((2, 3, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Scientist Male
        22 => Some((1, 3, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Scientist Female
        23 => Some((2, 3, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Scientist Female
        _  => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chardef_lookup_alignment_values() {
        // id 20 = Praxis Scientist Male -> alignment 1
        let entry20 = chardef_lookup(20).unwrap();
        assert_eq!(entry20.0, 1, "chardef 20 should be Praxis (alignment=1)");

        // id 22 = Praxis Scientist Female -> alignment 1
        let entry22 = chardef_lookup(22).unwrap();
        assert_eq!(entry22.0, 1, "chardef 22 should be Praxis (alignment=1)");

        // id 21 = SGU Scientist Male -> alignment 2
        let entry21 = chardef_lookup(21).unwrap();
        assert_eq!(entry21.0, 2, "chardef 21 should be SGU (alignment=2)");

        // id 23 = SGU Scientist Female -> alignment 2
        let entry23 = chardef_lookup(23).unwrap();
        assert_eq!(entry23.0, 2, "chardef 23 should be SGU (alignment=2)");
    }

    #[test]
    fn chardef_lookup_bodyset_doubled_format() {
        // All bodysets use "BS_X.BS_X" doubled format for the DB varchar(64) column.
        for id in [1, 9, 19] {
            let entry = chardef_lookup(id).unwrap();
            assert!(
                entry.3.contains('.'),
                "chardef {id} bodyset '{}' should contain a dot",
                entry.3
            );
        }
    }

    #[test]
    fn chardef_lookup_has_starting_coordinates() {
        // Every valid chardef should have at least one non-zero coordinate.
        for id in 1..=23 {
            let entry = chardef_lookup(id).unwrap();
            let (x, y, z) = (entry.5, entry.6, entry.7);
            assert!(
                x != 0.0 || y != 0.0 || z != 0.0,
                "chardef {id} should have non-zero coordinates, got ({x}, {y}, {z})"
            );
        }
    }

    #[test]
    fn chardef_lookup_praxis_starting_pos() {
        // Praxis entries spawn near (-334.2, 73.5, -228.0) in Castle_CellBlock.
        let entry = chardef_lookup(1).unwrap(); // Praxis Soldier Male
        assert_eq!(entry.0, 1, "should be Praxis alignment");
        assert!((entry.5 - (-334.231)).abs() < 1.0, "pos_x off: {}", entry.5);
        assert!((entry.6 - 73.472).abs() < 1.0, "pos_y off: {}", entry.6);
        assert!((entry.7 - (-228.026)).abs() < 1.0, "pos_z off: {}", entry.7);
    }

    #[test]
    fn chardef_lookup_sgu_starting_pos() {
        // SGU entries spawn near (201.5, 1.3, 49.7) in SGC_W1.
        let entry = chardef_lookup(2).unwrap(); // SGU Soldier Male
        assert_eq!(entry.0, 2, "should be SGU alignment");
        assert!((entry.5 - 201.5).abs() < 1.0, "pos_x off: {}", entry.5);
        assert!((entry.6 - 1.31).abs() < 1.0, "pos_y off: {}", entry.6);
        assert!((entry.7 - 49.724).abs() < 1.0, "pos_z off: {}", entry.7);
    }

    #[test]
    fn chardef_lookup_all_ids_valid() {
        for id in 1..=23 {
            assert!(
                chardef_lookup(id).is_some(),
                "chardef_lookup({id}) should return Some"
            );
        }
    }
}
