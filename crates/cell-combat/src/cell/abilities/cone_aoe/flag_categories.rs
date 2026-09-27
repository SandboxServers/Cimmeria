//! Effect-flag category logging.
//!
//! Surfaces the packed `EffectDef::flags` bitmask as structured tracing
//! events at fan-out time so operators can see which effect categories are
//! landing, even before the full mechanics ship in Phase G.

use cimmeria_entity::abilities::{
    EffectDef, EF_DOT, EF_INTERRUPT_CHANCE, EF_MENTAL_RESIST_ROLL, EF_STUN, EF_SUPPRESSION,
};

/// Read a packed flags integer and surface any non-trivial categories as
/// structured tracing events so operators can see which effects are
/// landing. v1 doesn't fully implement Stun / Suppression / Interrupt /
/// Resist-roll mechanics — they show up as separate effect scripts in
/// Phase G — but the flag inspection happens at fan-out time so the
/// observability is consistent.
pub fn log_effect_flag_categories(
    entity_id: u32,
    target_id: u32,
    ability_id: i32,
    effect: &EffectDef,
) {
    let flags = effect.flags;
    if flags == 0 {
        return;
    }
    // Bit checks — flags are a packed bitmask so multiple categories
    // can fire from one effect.
    let mut categories: Vec<&'static str> = Vec::new();
    if flags & EF_STUN == EF_STUN {
        categories.push("stun");
    }
    if flags & EF_INTERRUPT_CHANCE == EF_INTERRUPT_CHANCE {
        categories.push("interrupt_chance");
    }
    if flags & EF_MENTAL_RESIST_ROLL == EF_MENTAL_RESIST_ROLL {
        categories.push("mental_resist_roll");
    }
    if flags & EF_SUPPRESSION == EF_SUPPRESSION {
        categories.push("suppression");
    }
    if flags & EF_DOT == EF_DOT {
        categories.push("dot");
    }
    if categories.is_empty() {
        return;
    }
    tracing::debug!(
        target: "abilities",
        event = "effect_flag_categories",
        entity_id,
        target_id,
        ability_id,
        effect_id = effect.effect_id,
        flags,
        categories = ?categories,
        "Effect carries category flags — v1 logs them; full mechanics arrive in Phase G"
    );
}
