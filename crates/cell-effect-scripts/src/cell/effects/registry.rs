//! The table of every effect script, keyed by `script_name`.
//!
//! The composition root (`cimmeria-services`) builds the cell's
//! [`EffectScripts`] registry from [`EFFECT_SCRIPTS`] at startup
//! ([`effect_scripts`]), and the cell installs it on its `SpaceManager`.
//! [`EffectScripts::build`] refuses a duplicate or empty name, so a bad row
//! stops the server at startup instead of shadowing a script.
//!
//! Add a script by writing its `impl EffectScript` in the module for its
//! family and adding one row here. Nothing below this crate changes.
//!
//! Naming convention matches the original game's authoring scheme:
//! PascalCase, no underscores, e.g., `HealFocus` / `MeleeDamage`.

pub use cimmeria_cell_world::cell::effects::registry::*;

use super::EffectScript;
use super::{
    ammo_dart_cc, ammo_dart_support, ammo_dart_tech, ammo_emp, cover_stance, crowd_control,
    pet_scripts, scripts, stat_buff,
};

/// Every effect script, in registration order. The order is the order
/// [`EffectScripts::names`] reports; lookup is by name and never depends on
/// it.
pub static EFFECT_SCRIPTS: &[(&str, &dyn EffectScript)] = &[
    ("HealHealth", &scripts::HealHealth),
    ("HealFocus", &scripts::HealFocus),
    ("MeleeDamage", &scripts::MeleeDamage),
    ("MeleePhysicalDamage", &scripts::MeleePhysicalDamage),
    ("AbsorbShield", &scripts::AbsorbShield),
    // Crowd control (ability mechanics AB-09): ledger entries holding
    // BSF_MovementLock, and the interrupt queued for combat.
    ("Stun", &crowd_control::Stun),
    ("Suppression", &scripts::Suppression),
    ("RangedPhysicalDamage", &scripts::RangedPhysicalDamage),
    ("RangedEnergyDamage", &scripts::RangedEnergyDamage),
    // Cover Stance (ability 1451), granted and removed by the cover hold in
    // `cell::cover::stance` (NA22).
    ("CoverStance", &cover_stance::CoverStance),
    ("RemoveCoverStance", &cover_stance::RemoveCoverStance),
    // Owner abilities that act on pets, and the Heed Our Calling passive
    // (pets PT-08).
    ("PetStatBuff", &pet_scripts::PetStatBuff),
    ("PetDeathTimer", &pet_scripts::PetDeathTimer),
    ("HealPetHealth", &pet_scripts::HealPetHealth),
    ("PetSummonSpeed", &pet_scripts::PetSummonSpeed),
    // Timed stat buffs stacked by stat: the consumable stimpacks.
    ("StatBuff", &stat_buff::StatBuff),
    // Timed ability buffs and debuffs on the same ledger (ability mechanics
    // AB-04): one entry per (effect, invoker).
    ("TimedStat", &stat_buff::TimedStat),
    // "Remove Effect of moniker EFFECT_Stance": a stance clears the old one
    // (ability mechanics AB-08).
    ("RemoveByMoniker", &stat_buff::RemoveByMoniker),
    // Radioactive dart dose (ammo AM-11b).
    ("RadiationDamage", &ammo_dart_tech::RadiationDamage),
    // Antidote and Coagulant darts (ammo AM-11c): remove effects by category.
    ("RemoveEffects", &ammo_dart_support::RemoveEffects),
    // EMP rounds' on-hit effect 9120 (ammo campaign AM-09).
    ("EmpDisrupt", &ammo_emp::EmpDisrupt),
    // Dart_Tranquilizer's on-hit slow (ammo campaign AM-11a).
    ("MovementSlow", &ammo_dart_cc::MovementSlow),
    ("Knockdown", &crowd_control::Knockdown),
    ("Interrupt", &crowd_control::Interrupt),
];

/// The cell's registry, built from [`EFFECT_SCRIPTS`]: `Err` when two rows
/// share a name or a name is empty.
pub fn effect_scripts() -> Result<EffectScripts, EffectScriptError> {
    EffectScripts::build(EFFECT_SCRIPTS.iter().copied())
}

/// The script registered under `name`, straight from the table. Tests use
/// it; the cell looks scripts up in the registry installed on its
/// `SpaceManager`.
pub fn lookup(name: &str) -> Option<&'static dyn EffectScript> {
    EFFECT_SCRIPTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|&(_, script)| script)
}

/// Install the full registry on `mgr`, as the cell does at startup. For
/// tests that build their own `SpaceManager` and dispatch a script.
pub fn install(mgr: &mut crate::cell::space_manager::SpaceManager) {
    mgr.install_effect_scripts(effect_scripts().expect("the effect script table builds"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_scripts_resolve() {
        assert!(lookup("HealHealth").is_some());
        assert!(lookup("HealFocus").is_some());
        assert!(lookup("MeleeDamage").is_some());
        assert!(lookup("MeleePhysicalDamage").is_some());
        assert!(lookup("AbsorbShield").is_some());
        assert!(lookup("Stun").is_some());
        assert!(lookup("Knockdown").is_some());
        assert!(lookup("Interrupt").is_some());
        assert!(lookup("Suppression").is_some());
        assert!(lookup("RangedPhysicalDamage").is_some());
        assert!(lookup("RangedEnergyDamage").is_some());
        assert!(lookup("CoverStance").is_some());
        assert!(lookup("RemoveCoverStance").is_some());
        assert!(lookup("StatBuff").is_some());
        assert!(lookup("TimedStat").is_some());
        for pet_script in [
            "PetStatBuff",
            "PetDeathTimer",
            "HealPetHealth",
            "PetSummonSpeed",
        ] {
            assert!(lookup(pet_script).is_some(), "{pet_script}");
        }
    }

    #[test]
    fn unknown_script_returns_none() {
        assert!(lookup("BogusName").is_none());
        assert!(lookup("").is_none());
        assert!(lookup("healfocus").is_none(), "case-sensitive");
    }

    /// The table builds (no duplicate, no empty name), and the built
    /// registry answers every row with the row's own script, in table order.
    #[test]
    fn the_table_builds_and_the_registry_resolves_every_row() {
        let registry = effect_scripts().expect("the shipped table must build");
        assert_eq!(registry.len(), EFFECT_SCRIPTS.len());
        assert_eq!(
            registry.names().collect::<Vec<_>>(),
            EFFECT_SCRIPTS.iter().map(|(n, _)| *n).collect::<Vec<_>>()
        );
        for &(name, script) in EFFECT_SCRIPTS {
            let found = registry.lookup(name).expect(name);
            assert!(
                std::ptr::addr_eq(found as *const dyn EffectScript, script),
                "{name} resolves to its own row"
            );
        }
    }

    /// `install` puts the registry on a manager, and dispatch then finds a
    /// script that a bare manager does not.
    #[test]
    fn install_makes_dispatch_find_the_scripts() {
        use super::super::test_fixtures::effect_with_nvp;
        use super::super::{dispatch_by_name, EffectContext};
        use crate::cell::space_manager::SpaceManager;

        let effect = effect_with_nvp("HealAmount", "1");
        let mut bare = SpaceManager::new(1);
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut bare,
        };
        assert!(
            !dispatch_by_name("HealHealth", &mut ctx),
            "a bare manager has no scripts"
        );

        let mut mgr = SpaceManager::new(1);
        install(&mut mgr);
        let mut ctx = EffectContext {
            source_id: 1,
            target_id: 1,
            effect: &effect,
            space_mgr: &mut mgr,
        };
        assert!(dispatch_by_name("HealHealth", &mut ctx));
    }
}

/// The missing-registration guard on the shipped data.
#[cfg(test)]
mod live_db_tests {
    use super::*;
    use crate::cell::spawner::load_effect_defs;
    use crate::test_support::require_db_or_skip;

    /// The two seeded rows whose `script_name` no script has ever answered,
    /// before or after the move (the static `match` had no arm for either):
    ///
    /// - effect 658 "Reloads weapon" (ability 596 Reload) names `Reload`. The
    ///   reload runs through the reload pipeline, not an effect script.
    /// - effect 2907 "test" (ability 2134) carries an empty name.
    ///
    /// Both fall back to the legacy NVP path, as they always did, and the
    /// cell's startup `effect_script_unregistered` WARN names them. Fixing
    /// either row in the seed fails this test on purpose: drop it from here.
    const KNOWN_UNSCRIPTED: [(&str, i32); 2] = [("", 2907), ("Reload", 658)];

    /// #962 test rule: every `script_name` the seeded effect rows carry has a
    /// row in [`EFFECT_SCRIPTS`], apart from [`KNOWN_UNSCRIPTED`]. Dropping or
    /// misspelling a row fails here (and the cell logs
    /// `effect_script_unregistered` at startup), instead of the effect
    /// silently falling back to the legacy NVP path at cast time. The seed
    /// does name scripts, so an empty scan is a broken load, not a pass.
    #[tokio::test]
    async fn every_seeded_script_name_is_registered() {
        let pool = require_db_or_skip!();
        let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
        let named = defs.values().filter(|d| d.script_name.is_some()).count();
        assert!(named > 0, "the seed names effect scripts");
        let registry = effect_scripts().expect("the shipped table must build");
        let expected: Vec<(String, Vec<i32>)> = KNOWN_UNSCRIPTED
            .iter()
            .map(|&(name, effect_id)| (name.to_string(), vec![effect_id]))
            .collect();
        assert_eq!(
            registry.unregistered(defs.values()),
            expected,
            "seeded script names with no EFFECT_SCRIPTS row"
        );
    }
}
