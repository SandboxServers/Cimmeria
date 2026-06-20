//! `choose_npc_ability` selector tests — three-bucket selection across
//! cooldown gating, all-cooling hold-fire, empty-bucket fallback, and
//! the ammo-bearing ability case.

use super::make_aggression_fixture;

/// Selector: NPC with two abilities, the lower-id one on cooldown →
/// returns the higher-id one. Pins that the cooldown gate participates
/// in selection (without it, the selector always returns the first
/// sorted ability and the test would still pass for the wrong reason).
#[tokio::test]
async fn selector_skips_cooling_ability() {
    let mut mgr = make_aggression_fixture(200_004, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_004) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(221);
        npc.abilities.add_ability(592);
        // Sort order is ascending — put the first one on cooldown so the
        // selector must skip past it.
        npc.abilities
            .start_ability_cooldown(221, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_004, &mgr);
    assert_eq!(chosen, Some(592), "cooling 221 → selector must pick 592");
}

/// Selector: NPC with three abilities, the two lowest-id ones on
/// cooldown → returns the third. Pins selection across a non-trivial
/// bucket (the single-cooling-of-two case is the minimal partition; a
/// multi-cooling case verifies the `find` walks past every cooling
/// ability rather than stopping at the first).
#[tokio::test]
async fn selector_skips_multiple_cooling_abilities() {
    let mut mgr = make_aggression_fixture(200_008, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_008) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(100);
        npc.abilities.add_ability(200);
        npc.abilities.add_ability(300);
        // Sort order is ascending — cool 100 and 200, leave 300 fresh.
        npc.abilities
            .start_ability_cooldown(100, std::time::Duration::from_secs(10));
        npc.abilities
            .start_ability_cooldown(200, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_008, &mgr);
    assert_eq!(
        chosen,
        Some(300),
        "selector must walk past 100 and 200 (both cooling) to reach 300",
    );
}

/// Selector: every known ability is on cooldown → `None` so the AI tick
/// holds fire rather than misfiring against an unfireable ability.
#[tokio::test]
async fn selector_returns_none_when_all_cooling() {
    let mut mgr = make_aggression_fixture(200_005, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_005) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(221);
        npc.abilities
            .start_ability_cooldown(221, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_005, &mgr);
    assert_eq!(chosen, None, "all cooling → hold fire");
}

/// Selector: NPC with no abilities at all (misconfigured template) falls
/// back to `NPC_DEFAULT_ABILITY` so the tick never wedges silently.
#[tokio::test]
async fn selector_falls_back_to_default_when_no_abilities() {
    let mut mgr = make_aggression_fixture(200_006, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_006) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_006, &mgr);
    assert_eq!(
        chosen,
        Some(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "empty bucket → fallback to NPC_DEFAULT_ABILITY",
    );
}

/// Selector: ability with `required_ammo > 0` is still picked — NPCs have
/// infinite ammo (the ammo gate at the dispatch site is player-only).
/// Pinning this guards against re-introducing the regression where every
/// NPC carrying Pistol Shot 592 (`required_ammo = 1`) silently held
/// fire forever. The test seeds the ability def with `required_ammo = 1`
/// directly — without this seed, `space_mgr.ability_defs.get(&592)`
/// would return `None` and a re-introduced ammo gate would silently
/// fall through to `required_ammo = 0` and pass.
#[tokio::test]
async fn selector_picks_ammo_bearing_ability_for_npc() {
    use cimmeria_entity::abilities::AbilityDef;

    let mut mgr = make_aggression_fixture(200_007, 10, 1, [5.0, 0.0, 0.0]);
    mgr.ability_defs.insert(
        crate::cell::combat::NPC_DEFAULT_ABILITY,
        AbilityDef {
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            name: "Pistol Shot".to_string(),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            // Mirror the seeded DB value — Pistol Shot is required_ammo=1.
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_007, &mgr);
    assert_eq!(
        chosen,
        Some(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "selector must pick 592 even with required_ammo > 0 — NPCs have \
         infinite ammo and the dispatch-site ammo check is player-only",
    );
}
