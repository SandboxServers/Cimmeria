//! `EAmmoType` ordinals and the special-ammo predicate (ammo campaign AM-F,
//! issue #1026).
//!
//! The wire's `ammo_type` int (`InvItem.curAmmoType`, `requestAmmoChange`,
//! `BandolierItem::cur_ammo_type`) and every `*_ammo_type_id` column the
//! loaders compute are the **0-based ordinal** of `resources."EAmmoType"`
//! (`array_position(enum_range(...), value) - 1`), in the `CREATE TYPE`
//! order of `db/resources/Abilities/Types/EAmmoType.sql`, which is also
//! `entities/defs/enumerations.xml`'s order. The constants below are that
//! order; a live-DB test in `cimmeria-cell-catalog`
//! (`live_db_ammo_catalog::live_db::ammo_type_ordinals_match_pg_enum`) pins them
//! against `pg_enum`, so an enum edit fails loudly instead of drifting.

/// `AMMO_NONE`.
pub const AMMO_NONE: i32 = 0;
/// `Bullet_Default`: free reloads (D-AM02).
pub const BULLET_DEFAULT: i32 = 1;
/// `Bullet_Armor_Piercing`.
pub const BULLET_ARMOR_PIERCING: i32 = 2;
/// `Bullet_Hollow_Point`.
pub const BULLET_HOLLOW_POINT: i32 = 3;
/// `Bullet_Incendiary`.
pub const BULLET_INCENDIARY: i32 = 4;
/// `Bullet_EMP`.
pub const BULLET_EMP: i32 = 5;
/// `Bullet_Explosive`.
pub const BULLET_EXPLOSIVE: i32 = 6;
/// `Dagger_Default`.
pub const DAGGER_DEFAULT: i32 = 7;
/// `Dagger_Metallic`.
pub const DAGGER_METALLIC: i32 = 8;
/// `Dagger_Poison`.
pub const DAGGER_POISON: i32 = 9;
/// `Dagger_Electrical`.
pub const DAGGER_ELECTRICAL: i32 = 10;
/// `Dagger_Disease`.
pub const DAGGER_DISEASE: i32 = 11;
/// `Dagger_Plasma`.
pub const DAGGER_PLASMA: i32 = 12;
/// `Dart_Default`: free reloads (D-AM02).
pub const DART_DEFAULT: i32 = 13;
/// `Dart_Poison`.
pub const DART_POISON: i32 = 14;
/// `Dart_Disease`.
pub const DART_DISEASE: i32 = 15;
/// `Dart_Tranquilizer`.
pub const DART_TRANQUILIZER: i32 = 16;
/// `Dart_EMP`.
pub const DART_EMP: i32 = 17;
/// `Dart_Radioactive`.
pub const DART_RADIOACTIVE: i32 = 18;
/// `Dart_Stim`.
pub const DART_STIM: i32 = 19;
/// `Dart_Coagulant`.
pub const DART_COAGULANT: i32 = 20;
/// `Dart_Nanites`.
pub const DART_NANITES: i32 = 21;
/// `Dart_Antidote`.
pub const DART_ANTIDOTE: i32 = 22;
/// `Dart_Adrenaline`.
pub const DART_ADRENALINE: i32 = 23;

/// Every `EAmmoType` label, indexed by ordinal. The live-DB pin compares
/// this slice with `pg_enum` label by label.
pub const LABELS: [&str; 24] = [
    "AMMO_NONE",
    "Bullet_Default",
    "Bullet_Armor_Piercing",
    "Bullet_Hollow_Point",
    "Bullet_Incendiary",
    "Bullet_EMP",
    "Bullet_Explosive",
    "Dagger_Default",
    "Dagger_Metallic",
    "Dagger_Poison",
    "Dagger_Electrical",
    "Dagger_Disease",
    "Dagger_Plasma",
    "Dart_Default",
    "Dart_Poison",
    "Dart_Disease",
    "Dart_Tranquilizer",
    "Dart_EMP",
    "Dart_Radioactive",
    "Dart_Stim",
    "Dart_Coagulant",
    "Dart_Nanites",
    "Dart_Antidote",
    "Dart_Adrenaline",
];

/// The label of an ordinal, or `None` for a value outside `0..=23`.
pub fn label(ammo_type: i32) -> Option<&'static str> {
    usize::try_from(ammo_type)
        .ok()
        .and_then(|i| LABELS.get(i).copied())
}

/// Whether `ammo_type` is **finite** special ammo, drawn from a bag stack on
/// reload once `ammo.finite_special` is on (D-AM02).
///
/// False for the four free types, `AMMO_NONE` and the three `*_Default`
/// ammos (`Bullet_Default`, `Dart_Default`, `Dagger_Default`), which keep
/// today's free reloads; true for every other valid ordinal. The same four
/// are excluded by the `*_special_only_chk` constraints on
/// `resources.ammo_item_types` and `resources.ammo_modifiers`. No dagger
/// type has a reserve item yet (D-AM08), so a caller that needs the item
/// asks the ammo catalog and gets a miss. False for anything outside
/// `0..=23`: an unknown value is never treated as a reserve type.
pub const fn is_special(ammo_type: i32) -> bool {
    match ammo_type {
        AMMO_NONE | BULLET_DEFAULT | DAGGER_DEFAULT | DART_DEFAULT => false,
        BULLET_ARMOR_PIERCING..=DART_ADRENALINE => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_special_for_every_ordinal() {
        let free = [AMMO_NONE, BULLET_DEFAULT, DAGGER_DEFAULT, DART_DEFAULT];
        for ammo_type in 0..=23 {
            assert_eq!(
                is_special(ammo_type),
                !free.contains(&ammo_type),
                "ordinal {ammo_type} ({:?})",
                label(ammo_type)
            );
        }
    }

    #[test]
    fn is_special_rejects_out_of_range_values() {
        for ammo_type in [-1, 24, 255, i32::MIN, i32::MAX] {
            assert!(!is_special(ammo_type), "{ammo_type}");
        }
    }

    #[test]
    fn labels_line_up_with_the_constants() {
        let named = [
            (AMMO_NONE, "AMMO_NONE"),
            (BULLET_DEFAULT, "Bullet_Default"),
            (BULLET_ARMOR_PIERCING, "Bullet_Armor_Piercing"),
            (BULLET_HOLLOW_POINT, "Bullet_Hollow_Point"),
            (BULLET_INCENDIARY, "Bullet_Incendiary"),
            (BULLET_EMP, "Bullet_EMP"),
            (BULLET_EXPLOSIVE, "Bullet_Explosive"),
            (DAGGER_DEFAULT, "Dagger_Default"),
            (DAGGER_METALLIC, "Dagger_Metallic"),
            (DAGGER_POISON, "Dagger_Poison"),
            (DAGGER_ELECTRICAL, "Dagger_Electrical"),
            (DAGGER_DISEASE, "Dagger_Disease"),
            (DAGGER_PLASMA, "Dagger_Plasma"),
            (DART_DEFAULT, "Dart_Default"),
            (DART_POISON, "Dart_Poison"),
            (DART_DISEASE, "Dart_Disease"),
            (DART_TRANQUILIZER, "Dart_Tranquilizer"),
            (DART_EMP, "Dart_EMP"),
            (DART_RADIOACTIVE, "Dart_Radioactive"),
            (DART_STIM, "Dart_Stim"),
            (DART_COAGULANT, "Dart_Coagulant"),
            (DART_NANITES, "Dart_Nanites"),
            (DART_ANTIDOTE, "Dart_Antidote"),
            (DART_ADRENALINE, "Dart_Adrenaline"),
        ];
        assert_eq!(named.len(), LABELS.len());
        for (ordinal, name) in named {
            assert_eq!(label(ordinal), Some(name));
        }
        assert_eq!(label(-1), None);
        assert_eq!(label(24), None);
    }
}
