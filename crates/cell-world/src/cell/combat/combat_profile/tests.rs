use super::*;

fn s(v: &str) -> String {
    v.to_string()
}

#[test]
fn explicit_set_is_kept_in_order_under_both_policies() {
    for policy in [
        PlaceholderPolicy::Legacy,
        PlaceholderPolicy::RequireWeaponEvidence,
    ] {
        let p = resolve_npc_combat_profile(&[710, 584], None, policy);
        assert_eq!(p.abilities, vec![710, 584]);
        assert_eq!(p.source, CombatProfileSource::ExplicitSet);
    }
}

#[test]
fn empty_set_under_legacy_gets_placeholder_without_components() {
    let p = resolve_npc_combat_profile(&[], None, PlaceholderPolicy::Legacy);
    assert_eq!(p.abilities, vec![NPC_DEFAULT_ABILITY]);
    assert_eq!(p.source, CombatProfileSource::LegacyPlaceholder);
    assert_eq!(p.weapon_evidence, WeaponEvidence::None);
}

#[test]
fn empty_set_under_legacy_gets_placeholder_with_staff() {
    let comps = [s("WP-Jaffa.WP_Staff_Plasma_4A")];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::Legacy);
    assert_eq!(p.abilities, vec![NPC_DEFAULT_ABILITY]);
    assert_eq!(p.source, CombatProfileSource::LegacyPlaceholder);
}

#[test]
fn human_smg_earns_placeholder_under_require_weapon_evidence() {
    let comps = [s("WP-Human.WP_SMG_1A")];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
    assert_eq!(p.abilities, vec![NPC_DEFAULT_ABILITY]);
    assert_eq!(p.source, CombatProfileSource::LegacyPlaceholder);
    assert_eq!(
        p.weapon_evidence,
        WeaponEvidence::Component("WP-Human.WP_SMG_1A".to_string())
    );
}

#[test]
fn jaffa_staff_is_unresolved_under_require_weapon_evidence() {
    let comps = [s("WP-Jaffa.WP_Staff_Plasma_4A")];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
    assert!(p.abilities.is_empty());
    assert_eq!(p.source, CombatProfileSource::Unresolved);
    assert_eq!(
        p.weapon_evidence,
        WeaponEvidence::Component("WP-Jaffa.WP_Staff_Plasma_4A".to_string())
    );
}

#[test]
fn human_firearm_anywhere_in_components_earns_placeholder() {
    let comps = [s("WP-Jaffa.WP_Staff_Plasma_4A"), s("WP-Human.WP_SMG_1A")];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
    assert_eq!(p.abilities, vec![NPC_DEFAULT_ABILITY]);
    assert_eq!(p.source, CombatProfileSource::LegacyPlaceholder);
    assert_eq!(
        p.weapon_evidence,
        WeaponEvidence::Component("WP-Jaffa.WP_Staff_Plasma_4A".to_string())
    );
}

#[test]
fn human_prefix_needs_the_dot() {
    let comps = [s("WP-Humanoid.X")];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
    assert!(p.abilities.is_empty());
    assert_eq!(p.source, CombatProfileSource::Unresolved);
}

/// NC-50b (D-NC13): a melee human is not a firearm. Templates 28, 92, 1415 and
/// 1465 carry only `WP-Human.WP_HandToHand_1A`, so they must idle. Revert
/// proof: restoring the `WP-Human.` prefix makes this test fail.
#[test]
fn hand_to_hand_human_gets_no_placeholder_under_require_weapon_evidence() {
    let comps = [
        s("BS_HumanMale.BS_HM_Torso_00"),
        s("WP-Human.WP_HandToHand_1A"),
    ];
    let p = resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
    assert!(p.abilities.is_empty());
    assert_eq!(p.source, CombatProfileSource::Unresolved);
    assert_eq!(
        p.weapon_evidence,
        WeaponEvidence::Component("WP-Human.WP_HandToHand_1A".to_string())
    );
    assert_eq!(
        placeholder_ability(PlaceholderPolicy::RequireWeaponEvidence),
        None
    );
}

/// NC-50b (D-NC13): every firearm class in `HUMAN_FIREARM_PREFIXES` still earns
/// the placeholder, pistol and SMG included. Revert proof: a melee-only
/// narrowing that drops firearms fails here.
#[test]
fn human_firearms_still_earn_placeholder_under_require_weapon_evidence() {
    for weapon in [
        "WP-Human.WP_Pistol_1A",
        "WP-Human.WP_SMG_1A",
        "WP-Human.WP_Rifle_5D",
        "WP-Human.WP_Shotgun_1A",
        "WP-Human.WP_HMG_1A",
    ] {
        let comps = [s(weapon)];
        let p =
            resolve_npc_combat_profile(&[], Some(&comps), PlaceholderPolicy::RequireWeaponEvidence);
        assert_eq!(p.abilities, vec![NPC_DEFAULT_ABILITY], "{weapon}");
        assert_eq!(p.source, CombatProfileSource::LegacyPlaceholder, "{weapon}");
    }
}

#[test]
fn no_weapon_evidence_is_unresolved_under_require_weapon_evidence() {
    let none = resolve_npc_combat_profile(&[], None, PlaceholderPolicy::RequireWeaponEvidence);
    assert!(none.abilities.is_empty());
    assert_eq!(none.source, CombatProfileSource::Unresolved);
    assert_eq!(none.weapon_evidence, WeaponEvidence::None);

    let empty: [String; 0] = [];
    let some_empty =
        resolve_npc_combat_profile(&[], Some(&empty), PlaceholderPolicy::RequireWeaponEvidence);
    assert!(some_empty.abilities.is_empty());
    assert_eq!(some_empty.source, CombatProfileSource::Unresolved);
    assert_eq!(some_empty.weapon_evidence, WeaponEvidence::None);
}

#[test]
fn weapon_evidence_skips_armour_and_returns_first_wp() {
    let comps = [
        s("BS_Body_1"),
        s("AR_Armour_2"),
        s("WP-Human.WP_SMG_1A"),
        s("WP-Jaffa.X"),
    ];
    assert_eq!(
        weapon_evidence(Some(&comps)),
        WeaponEvidence::Component("WP-Human.WP_SMG_1A".to_string())
    );
    let no_wp = [s("BS_Body_1"), s("AR_Armour_2")];
    assert_eq!(weapon_evidence(Some(&no_wp)), WeaponEvidence::None);
    assert_eq!(weapon_evidence(None), WeaponEvidence::None);
}

#[test]
fn placeholder_ability_depends_on_policy() {
    assert_eq!(placeholder_ability(PlaceholderPolicy::Legacy), Some(592));
    assert_eq!(
        placeholder_ability(PlaceholderPolicy::RequireWeaponEvidence),
        None
    );
}

#[test]
fn first_report_is_true_once_per_template() {
    assert!(first_report(0x7000_0101));
    assert!(!first_report(0x7000_0101));
    assert!(first_report(0x7000_0102));
    assert!(!first_report(0x7000_0102));
}

/// D-NC1 pinned: production runs `RequireWeaponEvidence`. Revert proof:
/// setting the constant back to `Legacy` fails here.
#[test]
fn production_policy_is_require_weapon_evidence() {
    assert_eq!(PLACEHOLDER_POLICY, PlaceholderPolicy::RequireWeaponEvidence);
}
