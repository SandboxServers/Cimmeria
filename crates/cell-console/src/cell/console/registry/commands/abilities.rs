//! Ability lab commands (ability-mechanics AB-L2) — the family
//! `console/abilities/` implements. `[target]` is the caller's selection
//! when it is in view, else the caller (`Target::None` passes it through).

use super::{spec, Spec, Target};

pub(super) const SPECS: &[Spec] = &[
    spec(
        "effects",
        0,
        0,
        Target::None,
        "Show the ability state of your selected target (else you): warmup, cooldowns, pulsing effects, ledger entries, state flags",
    ),
    spec(
        "cooldowns",
        0,
        2,
        Target::None,
        "List your cooldowns; .cooldowns reset [abilityId] clears all (or one) and tells your client so the hotbar sweep stops",
    ),
    spec(
        "dummy",
        0,
        3,
        Target::None,
        "Place a lab target that never attacks: .dummy [hostile|friendly] [templateId]; 1,000,000 Health, gone after 10 min or at logout; .dummy caster <abilityId> [intervalSecs] casts that ability at you every interval (default 8 s); .dummy clear removes yours",
    ),
    spec(
        "cleareffects",
        0,
        0,
        Target::None,
        "Strip every timed and pulsing effect from your selected target (else you), reason cleansed, and clear the client's icons",
    ),
];
