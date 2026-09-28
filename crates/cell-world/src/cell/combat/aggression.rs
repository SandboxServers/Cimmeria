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
/// candidate and every other player is not. An NPC caster keeps the
/// collectors' historical rule, a hostile-faction non-player, unchanged by
/// duels (SS-D2 scope: NPC-vs-player behaviour does not move).
pub fn may_hit_in_area(
    attacker: &CellEntity,
    candidate: &CellEntity,
    duels: &crate::cell::duel::DuelRegistry,
) -> bool {
    if attacker.is_player {
        player_may_attack(attacker, candidate, duels)
    } else {
        !candidate.is_player && candidate.faction == super::faction_reaction::HOSTILE_FACTION
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
