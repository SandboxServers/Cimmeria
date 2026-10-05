//! Effect monikers: the tags a live effect carries so another effect can
//! find and remove it ("Remove Effect of moniker EFFECT_Stance").
//!
//! A moniker id is the CRC-32 of its name, the same function behind every
//! `resources.monikers` row (`Soldier_Command` = 3212632871). In 2009 the
//! server tracked live effects' monikers in `effectMonikers` (entity id,
//! moniker CRC), but the seed carries no effect-moniker column (audit B-74):
//! `abilities.moniker_ids` are broad ability groups, and 1470900795 is on
//! most combat abilities, so removing by an ability moniker would strip
//! nearly every buff.
//!
//! So effect monikers come from two NVPs the ability-mechanics generator
//! writes (`tools/ability_mechanics/families/stat.py`, RECONSTRUCTION):
//!
//! - [`EFFECT_MONIKER_NVP`] on an effect that *carries* a moniker (a stance's
//!   held buff carries `EFFECT_Stance`);
//! - [`REMOVE_MONIKER_NVP`] on an effect that *removes* the entries carrying
//!   one, run by the [`REMOVE_BY_MONIKER_SCRIPT`] script.
//!
//! Only the names in [`KNOWN_EFFECT_MONIKERS`] resolve; any other name is
//! ignored with a warning by the script that reads it, so a typo in a seed
//! row can never match an ability moniker by accident.

/// CRC-32 of `EFFECT_Stance`. No ability in the seed carries it (pinned by a
/// live-DB guard), so removing it can only take off a stance's own entries.
pub const EFFECT_STANCE_MONIKER: i64 = 3_785_086_315;

/// NVP naming the moniker an effect's ledger entry carries.
pub const EFFECT_MONIKER_NVP: &str = "EffectMoniker";

/// NVP naming the moniker whose entries a removal effect takes off.
pub const REMOVE_MONIKER_NVP: &str = "RemoveMoniker";

/// The script that removes ledger entries by [`REMOVE_MONIKER_NVP`]. It
/// changes nothing on its own, so it never makes an ability hostile
/// (`ability_is_beneficial` skips it).
pub const REMOVE_BY_MONIKER_SCRIPT: &str = "RemoveByMoniker";

/// Effect monikers the server knows, by name.
pub const KNOWN_EFFECT_MONIKERS: &[(&str, i64)] = &[("EFFECT_Stance", EFFECT_STANCE_MONIKER)];

/// The id of the effect moniker called `name`, if the server knows it.
pub fn effect_moniker_id(name: &str) -> Option<i64> {
    KNOWN_EFFECT_MONIKERS
        .iter()
        .find(|(n, _)| *n == name.trim())
        .map(|&(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bitwise CRC-32 (IEEE, reflected), the function `zlib.crc32` computes.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// The seed's own moniker rows use this function, so a constant computed
    /// the same way names the same moniker the 2009 data meant.
    #[test]
    fn moniker_ids_are_the_crc32_of_their_names() {
        assert_eq!(crc32(b"Soldier_Command"), 3_212_632_871, "monikers.sql row");
        assert_eq!(
            crc32(b"CATEGORY_Weapons"),
            3_901_383_057,
            "monikers.sql row"
        );
        for &(name, id) in KNOWN_EFFECT_MONIKERS {
            assert_eq!(i64::from(crc32(name.as_bytes())), id, "{name}");
        }
    }

    #[test]
    fn only_known_names_resolve() {
        assert_eq!(
            effect_moniker_id("EFFECT_Stance"),
            Some(EFFECT_STANCE_MONIKER)
        );
        assert_eq!(
            effect_moniker_id(" EFFECT_Stance "),
            Some(EFFECT_STANCE_MONIKER)
        );
        assert_eq!(effect_moniker_id("EFFECT_Shield"), None);
        assert_eq!(effect_moniker_id("1470900795"), None);
    }
}
