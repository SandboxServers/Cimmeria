//! An absorb shield's NVPs (ability-mechanics AB-10): `ShieldAmount`
//! (capacity per damage type) and `ShieldType` (the types, by name or by the
//! server's `DT_*` number). Here, not beside the `AbsorbShield` script, so
//! the launch gate in `cimmeria-cell-combat` reads the same pools the script
//! grants.

use super::{EffectDef, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC, DT_UNTYPED};
use crate::stats::{ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PHYSICAL, ABSORB_PSIONIC, ABSORB_UNTYPED};

/// The capacity NVP.
pub const SHIELD_AMOUNT_NVP: &str = "ShieldAmount";
/// The damage-type list NVP.
pub const SHIELD_TYPE_NVP: &str = "ShieldType";

/// The `absorb*` stat a damage type's pool fills. An unknown type falls
/// back to the untyped pool, as the pipeline's drain does.
pub fn shield_pool_id(damage_type: i8) -> i32 {
    match damage_type {
        DT_PHYSICAL => ABSORB_PHYSICAL,
        DT_ENERGY => ABSORB_ENERGY,
        DT_HAZMAT => ABSORB_HAZMAT,
        DT_PSIONIC => ABSORB_PSIONIC,
        _ => ABSORB_UNTYPED,
    }
}

/// One `ShieldType` token as a server damage type.
fn damage_type_of(token: &str) -> Option<i8> {
    let t = token.trim();
    if let Ok(n) = t.parse::<i8>() {
        return [DT_UNTYPED, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC]
            .contains(&n)
            .then_some(n);
    }
    match t.to_ascii_lowercase().as_str() {
        "physical" => Some(DT_PHYSICAL),
        "energy" => Some(DT_ENERGY),
        "hazmat" | "contamination" => Some(DT_HAZMAT),
        "psionic" => Some(DT_PSIONIC),
        "untyped" => Some(DT_UNTYPED),
        _ => None,
    }
}

/// The damage types `effect` blocks: its `ShieldType` list, untyped when
/// absent or blank, `None` when a token is not a damage type.
pub fn shield_types(effect: &EffectDef) -> Option<Vec<i8>> {
    let Some(raw) = effect.params.get(SHIELD_TYPE_NVP) else {
        return Some(vec![DT_UNTYPED]);
    };
    let tokens: Vec<&str> = raw.split(',').filter(|t| !t.trim().is_empty()).collect();
    if tokens.is_empty() {
        return Some(vec![DT_UNTYPED]);
    }
    let mut out: Vec<i8> = Vec::new();
    for token in tokens {
        let dt = damage_type_of(token)?;
        if !out.contains(&dt) {
            out.push(dt);
        }
    }
    Some(out)
}

/// The `(absorb stat, amount)` pools `effect` grants, or `None` when it has
/// no positive `ShieldAmount` or an unknown `ShieldType`.
pub fn shield_pools(effect: &EffectDef) -> Option<Vec<(i32, i32)>> {
    let amount = effect.param_i32(SHIELD_AMOUNT_NVP);
    if amount <= 0 {
        return None;
    }
    Some(
        shield_types(effect)?
            .into_iter()
            .map(|dt| (shield_pool_id(dt), amount))
            .collect(),
    )
}
