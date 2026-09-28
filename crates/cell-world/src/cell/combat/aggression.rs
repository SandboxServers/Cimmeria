//! Effective NPC aggression (NA13, D-NA01): the runtime or seeded override,
//! else the faction reaction of the viewer toward the NPC.
//!
//! This is python's `SGWPlayer.getAggressionLevel`
//! (`deprecated/python/cell/SGWPlayer.py`): `aggressionOverride` when the mob
//! has one, else `FACTION_REACTION_TABLE[player.faction][mob.faction]`. Only
//! [`MobAggression::Hostile`] aggroes on sight.

use cimmeria_entity::cell_entity::PetState;
use cimmeria_entity::cell_entity::{CellEntity, MobAggression};

use super::faction_reaction::reaction;

/// Default NPC attack ability ID: "Pistol Shot" (ability 592, ranged DD).
/// Was incorrectly 597 ("Heal Focus") — a self-heal, not an attack. Granted
/// at spawn to an NPC whose template names no ability set, and what the
/// ability selector picks for an NPC that knows no ability. Re-exported as
/// `cell::combat::NPC_DEFAULT_ABILITY`.
pub const NPC_DEFAULT_ABILITY: i32 = 592;

/// Default proximity-aggro radius in world units (D-NA09), used when
/// `entity_templates.aggro_radius` is NULL. Horizontal distance. A starting
/// value: the 2009 radius is unrecovered, so this is tuned at UAT.
pub const DEFAULT_AGGRO_RADIUS: f32 = 18.0;

/// Default assist radius in world units (D-NA04, D-NA09), used when
/// `entity_templates.assist_radius` is NULL. Horizontal distance from the
/// assisting NPC to the neighbour that just engaged. A starting value: the
/// 2009 server had no assist at all, so this is ours to tune at UAT.
pub const DEFAULT_ASSIST_RADIUS: f32 = 10.0;

/// Largest height difference, in world units, at which the Idle scan still
/// considers a player to be on the NPC's floor (D-NA09). Cellblock storeys
/// sit about 5-10 u apart (y 24.7 / 34.6 / 39.6), and the navmesh ray cannot
/// see floors (audit S15), so this band is the only storey guard.
pub const AGGRO_VERTICAL_BAND: f32 = 4.0;

/// The faction a player *reacts as*. Server-side `CellEntity::faction` stays
/// 0 for players (the `faction == 10` hostility checks depend on that), but
/// every client is told faction 3 (`Praxis`) on world entry
/// (`mercury::aoi::PLAYER_FACTION`, python `SGWPlayer.py` `self.faction = 3`).
/// The reaction row must be the one the client itself uses.
pub const PLAYER_REACTION_FACTION: u8 = crate::mercury::aoi::PLAYER_FACTION;

/// Effective aggression of an NPC with `override_level` and `npc_faction`
/// toward a viewer of `viewer_faction`. The override wins; otherwise the
/// reaction table decides.
pub fn effective_aggression(
    override_level: Option<MobAggression>,
    viewer_faction: u8,
    npc_faction: u8,
) -> MobAggression {
    override_level.unwrap_or_else(|| reaction(viewer_faction, npc_faction))
}

/// Effective aggression of `npc` toward players.
pub fn aggression_toward_players(npc: &CellEntity) -> MobAggression {
    effective_aggression(
        npc.aggro.override_level,
        PLAYER_REACTION_FACTION,
        npc.faction,
    )
}

/// Whether `npc` aggroes players on sight. This is the Idle admission test
/// in `npc_ai_tick` and the first gate of the auto-aggro scan.
///
/// A pet is never hostile to players, whatever its faction or override
/// (pets PT-05, D-PT06): it fights for its owner, so it must not run the
/// player proximity scan or be recruited as an assister. The owner's faction
/// already reads neutral; this also covers a content `set_aggression` aimed at
/// a pet.
pub fn is_hostile_to_players(npc: &CellEntity) -> bool {
    !npc.extensions.contains::<PetState>() && aggression_toward_players(npc).is_hostile()
}

/// Whether any faction regards NPCs of `faction` as a target: a row of the
/// reaction table with at least one HOSTILE cell. A cheap precheck that keeps
/// the NPC-target scan off NPCs that could never find anything (faction 1
/// World Object, the friendly and neutral ambient rows, and so on).
pub fn faction_has_npc_enemies(faction: u8) -> bool {
    (0..super::faction_reaction::FACTION_COUNT as u8).any(|t| reaction(faction, t).is_hostile())
}

/// Effective aggression of NPC `viewer` toward NPC `target` (NPC-vs-NPC,
/// #1009): the reaction table read with the NPC as the viewer,
/// `REACTION[viewer.faction][target.faction]`.
///
/// The aggression override is player-facing (python `getAggressionLevel`),
/// so it only ever **narrows** this: a non-HOSTILE override (a chain-disarmed
/// guard, `set_aggression 0`) keeps the NPC out of NPC fights too, while a
/// HOSTILE override does not turn it on every NPC in reach. An NPC armed
/// against players by content fights the NPCs its faction already would.
pub fn npc_aggression_toward(viewer: &CellEntity, target: &CellEntity) -> MobAggression {
    match viewer.aggro.override_level {
        Some(level) if !level.is_hostile() => level,
        _ => reaction(viewer.faction, target.faction),
    }
}

/// Whether `e` is an NPC that takes part in NPC-vs-NPC combat at all: an
/// SGWMob (class 0x04) that is not a player and not a pet. Beings (props,
/// Col Marsh) never enter combat (NA42), and pets fight on their owner's
/// terms (pets PT-05), so neither is a viewer or a target here.
pub fn is_npc_combatant(e: &CellEntity) -> bool {
    !e.is_player
        && e.class_id == crate::mercury::SGWMOB_CLASS_ID
        && !e.extensions.contains::<PetState>()
}

/// Whether NPC `viewer` may take NPC `target` as a target: both are NPC
/// combatants ([`is_npc_combatant`]), they are different entities, and the
/// viewer's aggression toward the target is HOSTILE
/// ([`npc_aggression_toward`]). Liveness, state and geometry are the scan's
/// gates, not this rule's.
pub fn npc_may_target_npc(viewer: &CellEntity, target: &CellEntity) -> bool {
    viewer.entity_id != target.entity_id
        && is_npc_combatant(viewer)
        && is_npc_combatant(target)
        && npc_aggression_toward(viewer, target).is_hostile()
}

/// Whether `npc` looks for NPC targets on its Idle scan: an NPC combatant
/// whose faction has an enemy in the table and that no non-HOSTILE override
/// has disarmed. The Idle admission test's NPC half, beside
/// [`is_hostile_to_players`].
pub fn seeks_npc_targets(npc: &CellEntity) -> bool {
    is_npc_combatant(npc)
        && !npc.aggro.override_level.is_some_and(|l| !l.is_hostile())
        && faction_has_npc_enemies(npc.faction)
}

/// Whether the player `attacker` may damage `target`: THE player hostility
/// rule, the one the #444 single-target gate (`handle_use_ability`, the
/// warmup re-check) enforces, and the one a pet obeys on its owner's behalf
/// (pets PT-05: a pet fights only what its owner could).
///
/// - **An NPC target:** an NPC of [`HOSTILE_FACTION`] that is not a pet.
///   Never a pet (whatever its faction), never a vendor, quest giver or
///   neutral NPC, whatever its aggression override.
/// - **A player target:** only the attacker's opponent in an engaged duel,
///   in the same space, at the two entities the engage recorded
///   ([`DuelRegistry::can_harm`], social systems SS-D2; the entity check is
///   SS-D3's, so the gate and the non-lethal clamp cover the same hits).
///   Every other player, a bystander included, is untouchable. The duel
///   registry is the only authority: the PvP flag the client sees is never
///   read back (D-SS23), so a stuck flag cannot make anyone attackable.
///
/// Every caller inherits this rule: the single-target launch, the warmup
/// re-check, the ground-AoE and cone candidate filters, and a pet acting for
/// its owner.
///
/// [`HOSTILE_FACTION`]: super::faction_reaction::HOSTILE_FACTION
/// [`DuelRegistry::can_harm`]: crate::cell::duel::DuelRegistry::can_harm
pub fn player_may_attack(
    attacker: &CellEntity,
    target: &CellEntity,
    duels: &crate::cell::duel::DuelRegistry,
) -> bool {
    if target.is_player {
        return match (attacker.player_id, target.player_id) {
            (Some(a), Some(t)) => {
                attacker.is_player
                    && attacker.space_id == target.space_id
                    && duels.can_harm_entities(
                        a,
                        attacker.entity_id.0 as u32,
                        t,
                        target.entity_id.0 as u32,
                    )
            }
            _ => false,
        };
    }
    player_may_attack_pve(attacker, target)
}

/// [`player_may_attack`] with no duel: the rule for a caller that must never
/// admit a player target, whoever is dueling. Pets use it (a pet never joins
/// its owner's duel, the default until the owner decides otherwise), and it
/// is also the NPC half of [`player_may_attack`], so the two cannot drift.
/// A hostile-faction NPC that is not a pet; never a player, never a pet.
pub fn player_may_attack_pve(_attacker: &CellEntity, target: &CellEntity) -> bool {
    !target.is_player
        && !target.extensions.contains::<PetState>()
        && target.faction == super::faction_reaction::HOSTILE_FACTION
}

/// Whether an area ability (ground AoE, cone) cast by `attacker` may hit
/// `candidate`: the area-target filter both collectors share.
///
/// A player caster obeys [`player_may_attack`], so a duel partner is a
/// candidate and every other player is not. A pet obeys its owner's no-duel
/// rule, [`player_may_attack_pve`] (pets PT-05). Any other NPC caster hits the
/// NPCs it would take as targets, [`npc_may_target_npc`] (NPC-vs-NPC, #1009):
/// before #1009 it hit every hostile-faction (10) NPC, which made a NID
/// guard's area ability land on its own post and never on the friendlies it
/// was fighting. Players are never candidates of an NPC's area ability
/// ([`area_candidates`]); that is unchanged.
pub fn may_hit_in_area(
    attacker: &CellEntity,
    candidate: &CellEntity,
    duels: &crate::cell::duel::DuelRegistry,
) -> bool {
    if attacker.is_player {
        player_may_attack(attacker, candidate, duels)
    } else if attacker.extensions.contains::<PetState>() {
        player_may_attack_pve(attacker, candidate)
    } else {
        npc_may_target_npc(attacker, candidate)
    }
}

/// The entities an area ability cast by `attacker_id` scans: every NPC, plus
/// the caster's engaged duel opponent when there is one. Players other than
/// that opponent are never scanned, so no filter mistake can reach them.
/// Callers still apply their space, alive and geometry tests, and
/// [`may_hit_in_area`].
pub fn area_candidates(
    space_mgr: &crate::cell::space_manager::SpaceManager,
    attacker_id: u32,
) -> Vec<u32> {
    let mut out = space_mgr.all_npc_entity_ids();
    let opponent = space_mgr
        .get_entity(attacker_id)
        .filter(|a| a.is_player)
        .and_then(|a| a.player_id)
        .and_then(|pid| crate::cell::duel::engaged_opponent_entity(space_mgr, pid));
    out.extend(opponent);
    out
}

/// The override a content or console `level` sets.
///
/// `1..=5` are `EMobAggressionLevel` values, the same numbers the chain
/// seeds always carried (`{"level": 1}` meant "aggressive" before NA13 and
/// is HOSTILE now). `0` was the pre-NA13 "passive" value; it maps to
/// NEUTRAL so a chain that disarms an NPC with it keeps doing so even on a
/// faction-10 template. Anything else is `None`: the caller rejects it.
pub fn override_from_content_level(level: i32) -> Option<MobAggression> {
    if level == 0 {
        return Some(MobAggression::Neutral);
    }
    MobAggression::from_level(level)
}

/// The proximity-aggro radius of `npc`: its template value, else
/// [`DEFAULT_AGGRO_RADIUS`].
pub fn aggro_radius(npc: &CellEntity) -> f32 {
    npc.aggro.radius_override.unwrap_or(DEFAULT_AGGRO_RADIUS)
}

/// The assist radius of `npc` (NA14): its template value, else
/// [`DEFAULT_ASSIST_RADIUS`].
pub fn assist_radius(npc: &CellEntity) -> f32 {
    npc.aggro
        .assist_radius_override
        .unwrap_or(DEFAULT_ASSIST_RADIUS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NPC_HOSTILE_FACTION: u8 = crate::cell::combat::HOSTILE_FACTION;

    #[test]
    fn override_beats_the_reaction_table() {
        // Faction 10 alone is hostile to players ...
        assert_eq!(
            effective_aggression(None, PLAYER_REACTION_FACTION, NPC_HOSTILE_FACTION),
            MobAggression::Hostile
        );
        // ... but a seeded NEUTRAL keeps a chain-armed guard passive ...
        assert_eq!(
            effective_aggression(
                Some(MobAggression::Neutral),
                PLAYER_REACTION_FACTION,
                NPC_HOSTILE_FACTION
            ),
            MobAggression::Neutral
        );
        // ... and a HOSTILE override arms a mob whose faction would not.
        assert_eq!(
            effective_aggression(Some(MobAggression::Hostile), PLAYER_REACTION_FACTION, 1),
            MobAggression::Hostile
        );
    }

    #[test]
    fn players_react_as_the_wire_faction_not_the_server_zero() {
        assert_eq!(PLAYER_REACTION_FACTION, 3);
        // Row 0 (the server-side player faction) is NEUTRAL toward 10; using
        // it would silently disable faction-derived aggro everywhere.
        assert_eq!(reaction(0, NPC_HOSTILE_FACTION), MobAggression::Neutral);
        let mut npc = CellEntity::new(
            cimmeria_common::EntityId(1),
            cimmeria_common::SpaceId(1),
            cimmeria_common::Vector3::zero(),
        );
        npc.faction = NPC_HOSTILE_FACTION;
        assert!(is_hostile_to_players(&npc));
        npc.faction = 1;
        assert!(!is_hostile_to_players(&npc), "faction 1 is friendly");
        npc.faction = 0;
        assert!(!is_hostile_to_players(&npc), "NULL faction is neutral");
    }

    /// Only level 1 aggroes; the old `> 0` rule read 2-5 as hostile (A5).
    #[test]
    fn non_hostile_overrides_do_not_aggro() {
        for level in [
            MobAggression::Suspicious,
            MobAggression::Neutral,
            MobAggression::Friendly,
            MobAggression::Default,
        ] {
            assert!(!effective_aggression(Some(level), 3, 10).is_hostile());
        }
    }

    #[test]
    fn content_levels_keep_their_pre_na13_meaning() {
        assert_eq!(override_from_content_level(1), Some(MobAggression::Hostile));
        assert_eq!(override_from_content_level(0), Some(MobAggression::Neutral));
        assert_eq!(override_from_content_level(3), Some(MobAggression::Neutral));
        assert_eq!(override_from_content_level(6), None);
        assert_eq!(override_from_content_level(-1), None);
    }

    fn mob(id: i32, faction: u8) -> CellEntity {
        let mut e = CellEntity::new(
            cimmeria_common::EntityId(id),
            cimmeria_common::SpaceId(1),
            cimmeria_common::Vector3::zero(),
        );
        e.class_id = crate::mercury::SGWMOB_CLASS_ID;
        e.faction = faction;
        e
    }

    /// #1009: the table is read with the NPC as the viewer. Praxis (3) and
    /// Straegis (10) are mutually HOSTILE; the old player-only reading never
    /// looked at row 10 or at an NPC in row 3.
    #[test]
    fn npc_viewer_reads_its_own_row_of_the_table() {
        let praxis = mob(1, 3);
        let nid = mob(2, NPC_HOSTILE_FACTION);
        assert_eq!(npc_aggression_toward(&praxis, &nid), MobAggression::Hostile);
        assert_eq!(npc_aggression_toward(&nid, &praxis), MobAggression::Hostile);
        assert!(npc_may_target_npc(&praxis, &nid));
        assert!(npc_may_target_npc(&nid, &praxis));
        // Same faction: row 10, column 10 is FRIENDLY.
        let nid2 = mob(3, NPC_HOSTILE_FACTION);
        assert!(!npc_may_target_npc(&nid, &nid2));
        // Faction 1 (World Object) is FRIENDLY in every row: the Castle
        // friendlies seeded before #1009 are never a target.
        let world_object = mob(4, 1);
        assert!(!npc_may_target_npc(&nid, &world_object));
        assert!(!npc_may_target_npc(&world_object, &nid));
        // Asymmetric rows are honoured as written: Jaffa_Beleth (17) is
        // HOSTILE to Praxis (3), SUSPICIOUS toward Tollan_Ambient (37).
        assert!(npc_may_target_npc(&mob(5, 17), &mob(6, 3)));
        assert!(!npc_may_target_npc(&mob(5, 17), &mob(6, 37)));
    }

    /// A non-HOSTILE override disarms the NPC against NPCs too; a HOSTILE
    /// override does not widen its NPC targets past its faction row.
    #[test]
    fn override_only_narrows_npc_hostility() {
        let mut praxis = mob(1, 3);
        let nid = mob(2, NPC_HOSTILE_FACTION);
        praxis.aggro.override_level = Some(MobAggression::Neutral);
        assert!(!npc_may_target_npc(&praxis, &nid));
        assert!(!seeks_npc_targets(&praxis));
        let mut friendly = mob(3, 1);
        friendly.aggro.override_level = Some(MobAggression::Hostile);
        assert!(!npc_may_target_npc(&friendly, &nid));
        assert!(!seeks_npc_targets(&friendly), "row 1 has no enemy");
        praxis.aggro.override_level = Some(MobAggression::Hostile);
        assert!(npc_may_target_npc(&praxis, &nid));
    }

    /// Players, pets, beings and the entity itself are never NPC targets.
    #[test]
    fn only_npc_combatants_take_part() {
        let nid = mob(1, NPC_HOSTILE_FACTION);
        let mut player = mob(2, 3);
        player.is_player = true;
        assert!(!npc_may_target_npc(&nid, &player));
        let mut pet = mob(3, 3);
        pet.extensions.insert(PetState::new(99, vec![], 0b111, 0));
        assert!(!npc_may_target_npc(&nid, &pet));
        assert!(!npc_may_target_npc(&pet, &nid));
        assert!(!seeks_npc_targets(&pet));
        let mut being = mob(4, 3);
        being.class_id = 0x01;
        assert!(!npc_may_target_npc(&nid, &being));
        assert!(!npc_may_target_npc(&nid, &nid));
    }

    #[test]
    fn factions_with_npc_enemies() {
        assert!(faction_has_npc_enemies(3));
        assert!(faction_has_npc_enemies(NPC_HOSTILE_FACTION));
        assert!(!faction_has_npc_enemies(1), "World Object");
        assert!(!faction_has_npc_enemies(0), "Undefined");
        assert!(!faction_has_npc_enemies(9), "Friendly_Ambient");
        assert!(!faction_has_npc_enemies(200), "outside the table");
    }

    /// The area rule for an NPC caster follows the NPC target rule, not the
    /// old "any faction-10 NPC" (which hit the caster's own post).
    #[test]
    fn npc_area_ability_hits_hostile_npcs_not_its_own_side() {
        let duels = crate::cell::duel::DuelRegistry::default();
        let nid = mob(1, NPC_HOSTILE_FACTION);
        let nid2 = mob(2, NPC_HOSTILE_FACTION);
        let praxis = mob(3, 3);
        assert!(!may_hit_in_area(&nid, &nid2, &duels));
        assert!(may_hit_in_area(&nid, &praxis, &duels));
        assert!(may_hit_in_area(&praxis, &nid, &duels));
        // A pet keeps its owner's rule: any hostile-faction NPC.
        let mut pet = mob(4, 3);
        pet.extensions.insert(PetState::new(99, vec![], 0b111, 0));
        assert!(may_hit_in_area(&pet, &nid, &duels));
        assert!(!may_hit_in_area(&pet, &praxis, &duels));
    }

    #[test]
    fn radius_defaults_to_18_and_honours_the_template() {
        let mut npc = CellEntity::new(
            cimmeria_common::EntityId(1),
            cimmeria_common::SpaceId(1),
            cimmeria_common::Vector3::zero(),
        );
        assert_eq!(aggro_radius(&npc), 18.0);
        npc.aggro.radius_override = Some(30.0);
        assert_eq!(aggro_radius(&npc), 30.0);
    }

    #[test]
    fn assist_radius_defaults_to_10_and_honours_the_template() {
        let mut npc = CellEntity::new(
            cimmeria_common::EntityId(1),
            cimmeria_common::SpaceId(1),
            cimmeria_common::Vector3::zero(),
        );
        assert_eq!(assist_radius(&npc), 10.0);
        npc.aggro.assist_radius_override = Some(4.0);
        assert_eq!(assist_radius(&npc), 4.0);
        assert_eq!(aggro_radius(&npc), 18.0, "the two radii are independent");
    }
}
