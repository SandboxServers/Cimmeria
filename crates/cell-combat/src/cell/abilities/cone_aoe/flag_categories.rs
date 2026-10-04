//! Effect-flag logging.
//!
//! Surfaces the packed `EffectDef::flags` bitmask as a structured tracing
//! event at fan-out time, so an operator can see which `EEffectFlag` bits
//! the landing effects carry.
//!
//! The names are the client's `entities/defs/enumerations.xml`
//! `EEffectFlag` tokens. The old version tested made-up "category" values
//! (`EF_STUN = 12` is really ClearOnDeath + ClearOnDamage, `EF_DOT = 516`
//! is ClearOnDeath + SequenceOnStart), so its labels were wrong on every
//! row (ability-mechanics B-25, fixed by AB-06).

use cimmeria_entity::abilities::EffectDef;

/// Every `EEffectFlag` token, in bit order (`enumerations.xml:1094-1123`).
const EFFECT_FLAG_NAMES: [(u32, &str); 25] = [
    (1, "EF_Beneficial_Effect"),
    (2, "EF_Offline_Time_Counts"),
    (4, "EF_ClearOnDeath"),
    (8, "EF_ClearOnDamage"),
    (16, "EF_DontUseQR"),
    (32, "EF_HasInductionBar"),
    (64, "EF_SequenceOnFinish"),
    (128, "EF_SequenceOnPulse"),
    (256, "EF_SequenceOnFail"),
    (512, "EF_SequenceOnStart"),
    (1024, "EF_SequenceOnRemove"),
    (2048, "EF_ClearOnRez"),
    (4096, "EF_HideIconOnClient"),
    (8192, "EF_OnlySendToGroup"),
    (16_384, "EF_OnlySendToSelf"),
    (32_768, "EF_RemoveOnDisguiseZeroed"),
    (65_536, "EF_RemoveOnBandolierSlotChange"),
    (131_072, "EF_ResolveOnAbilityUser"),
    (262_144, "EF_DisableDisguiseWhenRemoved"),
    (524_288, "EF_AlwaysPersist"),
    (1_048_576, "EF_Response"),
    (2_097_152, "EF_RemoveOnStealthZeroed"),
    (4_194_304, "EF_CalculateQRFromTarget"),
    (8_388_608, "EF_PromptConfirmationDialog"),
    (16_777_216, "EF_SequenceOnConfirmation"),
];

/// Every bit a client `EEffectFlag` defines.
const KNOWN_EFFECT_FLAG_BITS: u32 = (1 << 25) - 1;

/// The `EEffectFlag` names set in `flags`, in bit order.
pub(crate) fn effect_flag_names(flags: u32) -> Vec<&'static str> {
    EFFECT_FLAG_NAMES
        .iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|&(_, name)| name)
        .collect()
}

/// Log the `EEffectFlag` bits an effect carries. Nothing is logged for an
/// effect with no flags. Bits the client enum does not define are logged
/// as `unknown_bits` instead of being dropped.
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
    tracing::debug!(
        target: "abilities",
        event = "effect_flag_categories",
        entity_id,
        target_id,
        ability_id,
        effect_id = effect.effect_id,
        flags,
        categories = ?effect_flag_names(flags),
        unknown_bits = flags & !KNOWN_EFFECT_FLAG_BITS,
        "effect carries EEffectFlag bits"
    );
}
