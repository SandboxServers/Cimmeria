//! The cell plugin table (#962, `docs/architecture/plugin-architecture.md`).
//!
//! The facade is the composition root: it is the one crate that names every
//! feature plugin, so a feature crate stays a leaf. Order matters where two
//! plugins subscribe to one hook point (they fire in this order), so the
//! table is an explicit list, never link-time discovery.
//!
//! It also builds the effect-script registry ([`effect_scripts`], #962 step
//! 4) from `cimmeria-cell-effect-scripts`' table, for the same reason: the
//! scripts crate stays a leaf only this crate names.

use cimmeria_cell_duel::DuelPlugin;
use cimmeria_cell_org::OrgPlugin;
use cimmeria_cell_pets::PetsPlugin;
use cimmeria_cell_world::cell::effects::registry::{EffectScriptError, EffectScripts};
use cimmeria_cell_world::cell::plugin::{CellPlugin, CellPlugins, PluginError};

/// Every cell plugin, in hook-firing order.
///
/// No two plugins share a hook point yet: pets use the owner-sweep and
/// arrival stages and the base-destroy hook; duels use the gate-crossing
/// stage and the disconnect-teardown, travel and death hooks; org uses the
/// base-disconnect and world-entry hooks. So the order reaches no wire
/// output today; a plugin that joins a shared point must pin its order with
/// a test (ADR §3.2).
pub fn cell_plugin_table() -> [&'static dyn CellPlugin; 3] {
    [&PetsPlugin, &DuelPlugin, &OrgPlugin]
}

/// The built and checked plugin registry the orchestrator installs on the
/// cell: `Err` when a registration is invalid or a plugin-owned cell method
/// has no handler.
pub fn cell_plugins() -> Result<CellPlugins, PluginError> {
    let plugins = CellPlugins::build(&cell_plugin_table())?;
    plugins.check_complete()?;
    Ok(plugins)
}

/// The effect-script registry the orchestrator installs on the cell (#962
/// step 4): every row of `cimmeria-cell-effect-scripts`' table, in table
/// order. `Err` when two rows share a name or a name is empty.
pub fn effect_scripts() -> Result<EffectScripts, EffectScriptError> {
    EffectScripts::build(cimmeria_cell_effect_scripts::EFFECT_SCRIPTS.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_cell_world::cell::plugin::PLUGIN_OWNED_CELL_METHODS;

    /// The shipped table builds and covers every plugin-owned cell method,
    /// so the orchestrator's startup check passes. (That no static router arm
    /// still claims one of them is `cimmeria-cell-methods`'
    /// `plugin_owned_methods_are_not_routed_here`.)
    #[test]
    fn the_default_table_builds_and_is_complete() {
        let plugins = cell_plugins().expect("the shipped plugin table must build");
        assert_eq!(plugins.plugin_names(), &["pets", "duel", "org"]);
        assert_eq!(
            plugins.cell_method_indices().collect::<Vec<_>>(),
            PLUGIN_OWNED_CELL_METHODS.to_vec()
        );
    }

    /// #962 test rule, for step 2: a table missing `DuelPlugin` must not
    /// start. `check_complete` names both duel methods, so the orchestrator
    /// refuses to start the cell (`plugin_table_incomplete`) instead of
    /// letting a duel answer or forfeit fall through to "Unhandled cell
    /// method call".
    #[test]
    fn a_table_without_the_duel_plugin_fails_the_startup_check() {
        use cimmeria_wire::cell::cell_methods::player::constants::{
            DUEL_FORFEIT, SEND_DUEL_RESPONSE,
        };

        let plugins =
            CellPlugins::build(&[&PetsPlugin, &OrgPlugin]).expect("pets and org still build");
        match plugins.check_complete() {
            Err(PluginError::MissingCellMethods { missing }) => {
                let indices: Vec<u16> = missing.iter().map(|(i, _)| *i).collect();
                assert_eq!(indices, vec![SEND_DUEL_RESPONSE, DUEL_FORFEIT]);
                assert_eq!(
                    missing.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
                    vec!["sendDuelResponse", "duelForfeit"]
                );
            }
            other => panic!("a table without DuelPlugin must fail check_complete: {other:?}"),
        }
    }

    /// #962 test rule, for step 3: a table missing `OrgPlugin` must not
    /// start. `check_complete` names the twelve OrganizationMember methods
    /// and `onOrganizationCreation`, so the orchestrator refuses to start the
    /// cell instead of letting a squad invite answer, a leave or an
    /// organization name fall through to "Unhandled cell method call".
    #[test]
    fn a_table_without_the_org_plugin_fails_the_startup_check() {
        use cimmeria_wire::cell::cell_methods::organization::{INVITE_RESPONSE, TRANSFER_CASH};
        use cimmeria_wire::cell::cell_methods::player::constants::ORG_CREATION;

        let plugins =
            CellPlugins::build(&[&PetsPlugin, &DuelPlugin]).expect("pets and duel still build");
        match plugins.check_complete() {
            Err(PluginError::MissingCellMethods { missing }) => {
                let indices: Vec<u16> = missing.iter().map(|(i, _)| *i).collect();
                let expected: Vec<u16> = (INVITE_RESPONSE..=TRANSFER_CASH)
                    .chain(std::iter::once(ORG_CREATION))
                    .collect();
                assert_eq!(indices, expected);
                let names: Vec<&str> = missing.iter().map(|(_, n)| *n).collect();
                assert_eq!(names.first(), Some(&"organizationInviteResponse"));
                assert_eq!(names.last(), Some(&"onOrganizationCreation"));
                assert!(
                    !names.contains(&"unknown"),
                    "every org index is a client cell method: {names:?}"
                );
            }
            other => panic!("a table without OrgPlugin must fail check_complete: {other:?}"),
        }
    }

    /// #962 step 4: the shipped script table builds, so the orchestrator
    /// installs a registry instead of refusing to start, and it registers
    /// every script the cell registry used to name, in the old order.
    #[test]
    fn the_effect_script_table_builds_and_names_every_script() {
        let scripts = effect_scripts().expect("the shipped effect script table must build");
        assert_eq!(
            scripts.names().collect::<Vec<_>>(),
            [
                "HealHealth",
                "HealFocus",
                "MeleeDamage",
                "MeleePhysicalDamage",
                "AbsorbShield",
                "Stun",
                "Suppression",
                "RangedPhysicalDamage",
                "RangedEnergyDamage",
                "CoverStance",
                "RemoveCoverStance",
                "PetStatBuff",
                "PetDeathTimer",
                "HealPetHealth",
                "PetSummonSpeed",
                "StatBuff",
                "RadiationDamage",
                "RemoveEffects",
                "EmpDisrupt",
                "MovementSlow",
            ]
        );
    }

    /// #962 test rule, for step 4: a table that registers a script twice
    /// does not build, so the orchestrator refuses to start
    /// (`effect_scripts_invalid`) instead of one row shadowing the other.
    #[test]
    fn a_duplicated_effect_script_row_fails_the_startup_build() {
        let table = cimmeria_cell_effect_scripts::EFFECT_SCRIPTS;
        let doubled = table.iter().copied().chain(std::iter::once(table[5]));
        assert_eq!(
            EffectScripts::build(doubled).unwrap_err(),
            EffectScriptError::DuplicateScript { name: "Stun" }
        );
    }
}
