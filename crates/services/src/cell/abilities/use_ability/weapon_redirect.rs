//! Archetype-default ranged auto-attack → active-weapon RANGED redirect.
//!
//! Read-only resolution step run at the very top of `handle_use_ability`,
//! before any mutable borrow: if the called ability is the universal
//! archetype-default ranged starter (Pistol Shot, 592) and the player's
//! active weapon binds a *different* RANGED ability, this returns the
//! weapon's binding (plus its freshly-looked-up `AbilityDef`) so the rest
//! of the handler keys on the weapon's ability.

use cimmeria_entity::abilities::AbilityDef;

use super::super::super::space_manager::SpaceManager;

/// The universal archetype-starter default ranged auto-attack ability.
///
/// Granted to every char_def (1-14) at character creation per
/// `db/resources/Archetypes/Seed/char_creation_abilities.sql`. The
/// action-bar slot the player drags Pistol Shot into stays bound to
/// `592` across weapon swaps; the fire-time redirect in
/// `handle_use_ability` resolves it to the active weapon's RANGED
/// binding so a player firing this with a P90 equipped fires SMG
/// Auto Attack (559), with a pistol fires Pistol Auto Attack (579),
/// etc. See [`resolve_weapon_redirect`] for the full rationale + scope
/// limits.
///
/// Single-id constant rather than a slice because every archetype
/// shares this one — Soldier, Commando, Scientist, Archaeologist all
/// get 592. If a future archetype defines a different default ranged
/// starter, add it as a second constant + extend the redirect check
/// to walk a small `&[i32]` slice.
const ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK: i32 = 592;

/// Resolve the archetype-default weapon redirect.
///
/// Returns the (possibly redirected) `ability_id` and the `AbilityDef`
/// that should be used for the rest of the handler. When no redirect
/// applies the inputs are returned unchanged.
///
/// Ability 592 (Pistol Shot) is granted to every char_def at
/// character creation as the universal "default ranged auto-attack"
/// (see `db/resources/Archetypes/Seed/char_creation_abilities.sql`:
/// every char_def 1-14 gets 592). The action-bar slot the player
/// drags it into stays bound to 592 across weapon swaps — the
/// client has no wire path for single-slot rebind (confirmed by
/// Ghidra scan, `client.def` audit, and the deprecated python
/// SGWPlayer reference). So when the player equips a P90 (which
/// binds 559 SMG Auto Attack to its RANGED event) and presses the
/// action-bar slot, the wire arrives as `useAbility(592)` and the
/// server fires Pistol Shot — wrong animation, wrong damage
/// profile, wrong ammo pool. Reported as "P90 fires using the
/// pistol shot ability and not the SMG shot ability."
///
/// Redirect rule: if the called ability is the archetype default
/// ranged auto-attack AND the player has a weapon active whose
/// RANGED binding differs, swap `ability_id` to the weapon's
/// binding before validation runs. The rest of the handler uses
/// the post-redirect id naturally — known-set check, cooldown,
/// range, effect dispatch, on-wire packets all key on the weapon's
/// ability. The player drags Pistol Shot to slot 1 once; equipping
/// a P90 makes that slot fire SMG; equipping a pistol fires Pistol
/// Auto Attack; the slot follows the weapon without ever needing a
/// rebind from the server.
///
/// **What's NOT in scope here:**
/// - Melee equivalent (Strike, 594). Strike's animation/identity is
///   more distinct than Pistol Shot's, and weapons each bind a
///   different MELEE id (P90: 595, pistol: 708). Could redirect
///   symmetrically but the user only reported the ranged break;
///   handle melee in a follow-up if it turns out to need the same
///   treatment.
/// - Heal Focus (597) and other non-attack archetype starters —
///   these have no weapon binding, the lookup returns None,
///   redirect is a no-op.
/// - Weapon-bound abilities already in the known set (e.g. player
///   firing 559 directly with P90 equipped). The redirect only
///   fires for the archetype-default starter id; firing the weapon
///   binding directly skips this entire block.
pub(super) fn resolve_weapon_redirect(
    entity_id: u32,
    ability_id: i32,
    ability_def: Option<AbilityDef>,
    space_mgr: &SpaceManager,
) -> (i32, Option<AbilityDef>) {
    if ability_id != ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK {
        return (ability_id, ability_def);
    }
    let Some(item_id) = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.bandolier_items.get(&e.active_bandolier_slot))
        .map(|b| b.item_id)
    else {
        return (ability_id, ability_def);
    };
    let Some(&weapon_ranged_ability) = space_mgr
        .item_event_set_abilities
        .get(&(item_id, super::super::super::spawner::EVENT_ITEM_RANGED))
    else {
        return (ability_id, ability_def);
    };
    // Same-id guard: if a weapon ever bound the same id the
    // player already presses (hypothetical content where a
    // pistol-class item binds 592 directly), the redirect is
    // a no-op — skip the rewrite and the re-lookup so the
    // DEBUG log only fires when something actually changed.
    if weapon_ranged_ability == ability_id {
        return (ability_id, ability_def);
    }
    // DEBUG, not INFO — fires on every player shot
    // during sustained fire (auto-cycle 1.5–2 Hz). INFO
    // would balloon the log volume for what is, after
    // the first redirect, expected steady-state
    // behavior. The `combat.use_ability` span (INFO)
    // already records every commit; this DEBUG just
    // discriminates the redirect path inside it.
    tracing::debug!(
        entity_id,
        original_ability_id = ability_id,
        weapon_ability_id = weapon_ranged_ability,
        item_id,
        "useAbility: redirecting archetype-default ranged \
         auto-attack to active weapon's RANGED binding"
    );
    // Depends on PR #494 adding the weapon's RANGED
    // binding to `known_abilities` at equip time. The
    // `is_ability_granted_by_active_weapon` fallback in
    // the validation block below catches the gap if PR
    // #494 hasn't landed yet, but the explicit grant is
    // the cleaner path so this comment doesn't drift.
    let redirected_def = space_mgr.ability_defs.get(&weapon_ranged_ability).cloned();
    (weapon_ranged_ability, redirected_def)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::cell_entity::BandolierItem;

    const SMG_AUTO_ATTACK: i32 = 559;
    const WEAPON_ITEM_ID: i32 = 4242;

    fn make_def(id: i32, name: &str) -> AbilityDef {
        AbilityDef {
            ability_id: id,
            name: name.to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        }
    }

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
        }
        mgr
    }

    fn equip_weapon(mgr: &mut SpaceManager, ranged_ability: i32) {
        if let Some(e) = mgr.get_entity_mut(1) {
            e.bandolier_items.insert(
                e.active_bandolier_slot,
                BandolierItem {
                    instance_id: 0,
                    item_id: WEAPON_ITEM_ID,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 30,
                    cur_ammo_type: 2,
                },
            );
        }
        mgr.item_event_set_abilities.insert(
            (
                WEAPON_ITEM_ID,
                super::super::super::super::spawner::EVENT_ITEM_RANGED,
            ),
            ranged_ability,
        );
    }

    #[test]
    fn non_archetype_ability_passes_through_unchanged() {
        let mut mgr = make_mgr();
        equip_weapon(&mut mgr, SMG_AUTO_ATTACK);
        let def = Some(make_def(123, "SomeOtherAbility"));
        let (id, out_def) = resolve_weapon_redirect(1, 123, def.clone(), &mgr);
        assert_eq!(id, 123, "non-592 id is never redirected");
        assert_eq!(out_def.map(|d| d.ability_id), Some(123));
    }

    #[test]
    fn archetype_default_redirects_to_active_weapon_ranged_binding() {
        let mut mgr = make_mgr();
        equip_weapon(&mut mgr, SMG_AUTO_ATTACK);
        mgr.ability_defs.insert(
            SMG_AUTO_ATTACK,
            make_def(SMG_AUTO_ATTACK, "SMG Auto Attack"),
        );
        let pistol_def = Some(make_def(
            ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK,
            "Pistol Shot",
        ));

        let (id, out_def) =
            resolve_weapon_redirect(1, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK, pistol_def, &mgr);
        assert_eq!(
            id, SMG_AUTO_ATTACK,
            "592 redirects to the weapon's RANGED binding"
        );
        assert_eq!(
            out_def.map(|d| d.ability_id),
            Some(SMG_AUTO_ATTACK),
            "the returned def is re-looked-up for the redirected id"
        );
    }

    #[test]
    fn archetype_default_with_no_weapon_passes_through() {
        // No bandolier item equipped → lookup misses → no redirect.
        let mgr = make_mgr();
        let pistol_def = Some(make_def(
            ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK,
            "Pistol Shot",
        ));
        let (id, out_def) =
            resolve_weapon_redirect(1, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK, pistol_def, &mgr);
        assert_eq!(
            id, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK,
            "no weapon = no redirect"
        );
        assert_eq!(
            out_def.map(|d| d.ability_id),
            Some(ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK)
        );
    }

    #[test]
    fn archetype_default_with_same_binding_is_noop() {
        // Weapon binds the same id the player pressed → same-id guard,
        // no redirect, original def preserved unchanged.
        let mut mgr = make_mgr();
        equip_weapon(&mut mgr, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK);
        let pistol_def = Some(make_def(
            ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK,
            "Pistol Shot",
        ));
        let (id, out_def) =
            resolve_weapon_redirect(1, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK, pistol_def, &mgr);
        assert_eq!(
            id, ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK,
            "same binding = no rewrite"
        );
        assert_eq!(
            out_def.map(|d| d.ability_id),
            Some(ARCHETYPE_DEFAULT_RANGED_AUTO_ATTACK)
        );
    }
}
