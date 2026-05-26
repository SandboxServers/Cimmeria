//! Static registry of effect scripts.
//!
//! Add a new script by:
//! 1. Implementing [`super::EffectScript`] in [`super::scripts`]
//! 2. Adding the script to the `match` in [`lookup`]
//!
//! Static dispatch via `match` (not a HashMap) so the cost is a single
//! branch + the trait-object pointer — no hash compute per effect
//! invocation. Five names today, scales fine until ~30 entries.

use super::scripts;
use super::EffectScript;

/// Lookup a script by name. Returns the trait object on hit, `None`
/// when no script is registered for the given name.
///
/// Naming convention matches the original game's authoring scheme:
/// PascalCase, no underscores, e.g., `HealFocus` / `MeleeDamage`.
pub fn lookup(name: &str) -> Option<&'static dyn EffectScript> {
    match name {
        "HealHealth" => Some(&scripts::HealHealth),
        "HealFocus" => Some(&scripts::HealFocus),
        "MeleeDamage" => Some(&scripts::MeleeDamage),
        "AbsorbShield" => Some(&scripts::AbsorbShield),
        "Stun" => Some(&scripts::Stun),
        "Suppression" => Some(&scripts::Suppression),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_scripts_resolve() {
        assert!(lookup("HealHealth").is_some());
        assert!(lookup("HealFocus").is_some());
        assert!(lookup("MeleeDamage").is_some());
        assert!(lookup("AbsorbShield").is_some());
        assert!(lookup("Stun").is_some());
        assert!(lookup("Suppression").is_some());
    }

    #[test]
    fn unknown_script_returns_none() {
        assert!(lookup("BogusName").is_none());
        assert!(lookup("").is_none());
        assert!(lookup("healfocus").is_none(), "case-sensitive");
    }
}
