//! Names from small, closed tables compiled into the server: archetypes,
//! racial paradigms and ammo types. These need no database and answer
//! before the boot load, so they are plain functions, not [`NameBook`]
//! lookups. Each seed-backed table is pinned against its seed by a live-DB
//! test.
//!
//! [`NameBook`]: crate::NameBook

use cimmeria_entity::ammo_type::label;

/// The archetype names, indexed by `EArchetype` ordinal (`sgw_player.archetype`
/// 0 to 8), as `resources.archetypes.name` spells them. The seed leaves
/// `ARCHETYPE_Any` (0) blank; it is no class a player can pick, but a row
/// that carries it still means "any", so it gets that name here.
pub const ARCHETYPE_NAMES: [&str; 9] = [
    "Any",
    "Soldier",
    "Commando",
    "Scientist",
    "Archeologist",
    "Asgard",
    "Goa'uld",
    "Shol'va",
    "Jaffa",
];

/// The name of archetype `id` (an `EArchetype` ordinal), or `None` outside 0 to 8.
pub fn archetype_name(id: i32) -> Option<&'static str> {
    usize::try_from(id)
        .ok()
        .and_then(|i| ARCHETYPE_NAMES.get(i).copied())
}

/// `resources.racial_paradigm` (`id`, `name`), for player-facing text. The
/// seed has exactly these five rows; `racial_paradigm_names_match_the_seed`
/// pins them against the database.
pub const RACIAL_PARADIGM_NAMES: [(i32, &str); 5] = [
    (1, "Common"),
    (2, "Human"),
    (3, "Goa'uld"),
    (4, "Asgard"),
    (5, "Ancient"),
];

/// The name of racial paradigm `id`, or `None` for an id the seed does not
/// have.
pub fn racial_paradigm_name(id: i32) -> Option<&'static str> {
    RACIAL_PARADIGM_NAMES
        .iter()
        .find(|&&(pid, _)| pid == id)
        .map(|&(_, name)| name)
}

/// "Hollow Point" for `Bullet_Hollow_Point`: the `EAmmoType` label without
/// its family prefix, for feedback lines. An ordinal with no label is
/// "special".
pub fn ammo_name(ammo_type: i32) -> String {
    let raw = label(ammo_type).unwrap_or("special");
    let tail = raw.split_once('_').map_or(raw, |(_, t)| t);
    tail.replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_HOLLOW_POINT};

    #[test]
    fn archetype_names_follow_the_enum_order() {
        assert_eq!(archetype_name(0), Some("Any"));
        assert_eq!(archetype_name(1), Some("Soldier"));
        assert_eq!(archetype_name(6), Some("Goa'uld"));
        assert_eq!(archetype_name(7), Some("Shol'va"));
        assert_eq!(archetype_name(8), Some("Jaffa"));
        assert_eq!(archetype_name(9), None);
        assert_eq!(archetype_name(-1), None);
    }

    #[test]
    fn racial_paradigm_names_are_the_seed_rows() {
        assert_eq!(racial_paradigm_name(3), Some("Goa'uld"));
        assert_eq!(racial_paradigm_name(0), None);
        assert_eq!(racial_paradigm_name(6), None);
    }

    #[test]
    fn ammo_name_drops_the_family_prefix() {
        assert_eq!(ammo_name(BULLET_HOLLOW_POINT), "Hollow Point");
        assert_eq!(ammo_name(BULLET_ARMOR_PIERCING), "Armor Piercing");
        assert_eq!(ammo_name(-1), "special");
    }
}
