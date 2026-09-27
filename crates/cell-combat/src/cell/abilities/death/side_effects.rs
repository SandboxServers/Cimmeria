//! Post-transition death side effects — the death animation, the kill-XP
//! grant, and the player Defeat Window.
//!
//! These used to live inline in `damage_apply`, which is why an
//! effect-script kill produced none of them (see [`super::resolve_death`]).
//! They sit behind the wire burst in [`super::apply_death_transition`]
//! because each one depends on the corpse already carrying `BSF_DEAD`:
//! the animation plays on a dead-state model, the XP grant is the
//! once-per-corpse payout, and the Defeat Window is only meaningful once
//! the client has flipped the dying player into the dead state.

use cimmeria_entity::cell_entity::PlayerIdentity;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::loot_drop::kill_xp;
use super::super::messaging::{send_entity_method, send_entity_method_to_self_and_witnesses};

/// Kismet event id for the death animation. Event set 1025 (Mob) drives
/// it for both NPCs and players today; if they ever diverge, branch on
/// `e.is_player` at the lookup below.
const EVENT_ENTITY_DEATH: i32 = 5001;

/// Broadcast the `onSequence` death animation for `target_eid`.
///
/// Fans to self+witnesses so a spectator sees the entity fall. Silently
/// no-ops when the entity's event set has no `Entity_Death` sequence —
/// content without a death anim is normal, not an error.
pub(super) async fn send_death_sequence(
    target_eid: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(esid) = space_mgr.get_entity(target_eid).map(|_| 1025) else {
        return;
    };
    let Some(&death_seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ENTITY_DEATH)) else {
        return;
    };

    let mut seq_args = Vec::with_capacity(28);
    seq_args.extend_from_slice(&death_seq_id.to_le_bytes()); // KismetEventSetSeqID
    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // SourceID (dying entity)
    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // TargetID (also dying entity — NOT killer, or client plays death anim on killer)
    seq_args.push(1); // PrimaryTarget
    seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
    seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs count
    seq_args.push(0); // ViewType
    seq_args.extend_from_slice(&0i32.to_le_bytes()); // InstanceId

    send_entity_method_to_self_and_witnesses(
        target_eid,
        crate::mercury::method_idx::ON_SEQUENCE,
        seq_args,
        tx,
        space_mgr,
    )
    .await;
    tracing::debug!(
        target: "abilities.sequence",
        event = "entity_death",
        source_id = target_eid,
        target_id = target_eid,
        sequence_id = death_seq_id,
        event_set_id = esid,
        "onSequence broadcast: Entity_Death (death animation)"
    );
}

/// Grant kill XP for a dead NPC to whoever `attacker_id` credits.
///
/// No-op for player targets — PvP pays no XP. The recipient comes from
/// [`SpaceManager::credit_recipient`] (pets PT-06, D-PT02): a player is
/// paid itself, a pet pays its owner scaled by `PetState::transfer_xp`,
/// and any other NPC attacker pays nobody. Before that seam a pet kill
/// sent `GrantXP` to the pet's id (no base session, so the XP was lost)
/// and a mob that killed a pet sent `GrantXP` to the mob's id.
///
/// Telemetry: a pet's kill logs `event = "pet_kill_credited"` on
/// `pets.credit`; every kill that pays nothing logs its `reason` (see
/// [`NoKillXp`]). The `GrantXP` hop is the only thing standing between a
/// kill and the player's level bar, so a send failure is an `error!`, not a
/// silent drop.
pub(super) async fn grant_kill_xp(
    target_eid: u32,
    attacker_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(target) = space_mgr.get_entity(target_eid) else {
        return;
    };
    if target.is_player {
        return;
    }
    let base_xp = kill_xp(target.level);
    let KillXpPayout { recipient, xp } = match kill_xp_payout(space_mgr, attacker_id, base_xp) {
        Ok(payout) => payout,
        Err(no_xp) => {
            log_no_kill_xp(no_xp, target_eid, attacker_id, base_xp, space_mgr);
            return;
        }
    };
    tracing::info!(
        attacker = attacker_id,
        credited = recipient,
        via_pet = recipient != attacker_id,
        target = target_eid,
        mob_level = target.level,
        base_xp,
        xp,
        "Granting kill XP"
    );
    if let Some(pet) = space_mgr
        .get_entity(attacker_id)
        .and_then(|e| e.pet.as_ref())
    {
        // `xp_before` is not logged: the cell holds no XP total. The base's
        // `progression.grant_xp` span (keyed on the owner's `entity_id`) is
        // where the before/after lives. The identity is the summon-time
        // capture, the same one `credit_recipient` just matched.
        let id = space_mgr.pets.summoner_identity(attacker_id);
        tracing::debug!(
            target: "pets.credit",
            event = "pet_kill_credited",
            entity_id = attacker_id,
            pet_id = attacker_id,
            owner_id = recipient,
            account_id = id.account_id,
            player_id = id.player_id,
            victim_id = target_eid,
            victim_template_id = target.template_id,
            victim_level = target.level,
            base_xp,
            xp_granted = xp,
            transfer_xp = pet.transfer_xp,
            "pet kill credited to its owner"
        );
    }
    if let Err(e) = tx
        .send(CellToBaseMsg::GrantXP {
            entity_id: recipient,
            xp_amount: xp,
            // Mob-kill XP is not GM-sourced — no GM feedback line.
            gm_feedback_to: None,
        })
        .await
    {
        tracing::error!(
            attacker = attacker_id, credited = recipient, target = target_eid, xp,
            error = %e,
            "GrantXP send to base failed -- player kill credit lost"
        );
    }
}

/// Who a kill's XP goes to, and how much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct KillXpPayout {
    pub(super) recipient: u32,
    pub(super) xp: u64,
}

/// Why a kill paid no XP. `reason()` is the stable log value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum NoKillXp {
    /// An ordinary NPC (not a pet) killed an NPC.
    NpcAttacker,
    /// The attacker is a registered pet whose owner's entity id no longer
    /// belongs to the summoner. [`SpaceManager::credit_recipient`] already
    /// logged it (`credit_refused`, WARN), so nothing is logged again.
    CreditRefused,
    /// The attacker still carries pet state but the registry already
    /// dropped it (the teardown gap before the entity goes).
    PetUnregistered,
    /// The pet's `transfer_xp` is non-finite or not above zero.
    TransferXpInvalid(f32),
    /// The pet's `transfer_xp` is finite but so large that the scaled
    /// payout exceeds [`MAX_KILL_XP`] (bad seed data).
    XpOverflow(f32),
    /// The payout rounds to zero XP (a level-0 victim, or a tiny scale).
    ZeroXp,
}

impl NoKillXp {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::NpcAttacker => "npc_attacker",
            Self::CreditRefused => "credit_refused",
            Self::PetUnregistered => "pet_unregistered",
            Self::TransferXpInvalid(_) => "transfer_xp_invalid",
            Self::XpOverflow(_) => "xp_overflow",
            Self::ZeroXp => "zero_xp",
        }
    }
}

/// The largest single kill payout: `i32::MAX`, for every kill, pet or
/// player, whatever the scale.
///
/// Two inputs can exceed it. `kill_xp` is `10 * level` in `u64`, so a
/// victim with a corrupt huge level pays more than the base can store even
/// at scale 1.0. And a finite but huge `transfer_xp` (`f32::MAX`) makes the
/// `f64` product exceed `u64`, where an `as u64` cast saturates rather than
/// failing: the owner would be granted `u64::MAX` XP. `i32::MAX` is
/// the narrowest place XP is stored: `sgw_player.exp` is `integer` and the
/// wire payload is an `INT32`, so no single payout can be larger than what
/// the base can persist (#889).
pub(super) const MAX_KILL_XP: u64 = i32::MAX as u64;

/// Resolve `attacker_id` to the credited player and scale `base_xp` by the
/// pet's `transfer_xp` when the attacker is a pet (1.0 per D-PT02; a
/// player attacker always gets the full amount).
///
/// `Err` when nobody is credited, or when the scale leaves nothing to pay:
/// a template authored with `transfer_xp = 0`, or a non-finite or negative
/// value, which fails closed to zero rather than minting XP. Any payout
/// above [`MAX_KILL_XP`] also fails closed (`XpOverflow`) on every path,
/// the 1.0 fast path included: a scaled payout is checked on the `f64`
/// product before the integer cast, and every payout on the integer after.
pub(super) fn kill_xp_payout(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    base_xp: u64,
) -> Result<KillXpPayout, NoKillXp> {
    let pet_scale = space_mgr
        .get_entity(attacker_id)
        .and_then(|e| e.pet.as_ref())
        .map(|p| p.transfer_xp);
    let Some(recipient) = space_mgr.credit_recipient(attacker_id) else {
        return Err(if space_mgr.pets.is_pet(attacker_id) {
            NoKillXp::CreditRefused
        } else if pet_scale.is_some() {
            NoKillXp::PetUnregistered
        } else {
            NoKillXp::NpcAttacker
        });
    };
    let scale = pet_scale.unwrap_or(1.0);
    let xp = if scale == 1.0 {
        base_xp
    } else if scale.is_finite() && scale > 0.0 {
        let scaled = (base_xp as f64 * f64::from(scale)).round();
        if scaled > MAX_KILL_XP as f64 {
            return Err(NoKillXp::XpOverflow(scale));
        }
        scaled as u64
    } else {
        return Err(NoKillXp::TransferXpInvalid(scale));
    };
    // The 1.0 fast path skips the `f64` check above; `kill_xp` of a huge
    // victim level can exceed the cap on its own (#889).
    if xp > MAX_KILL_XP {
        return Err(NoKillXp::XpOverflow(scale));
    }
    if xp == 0 {
        return Err(NoKillXp::ZeroXp);
    }
    Ok(KillXpPayout { recipient, xp })
}

/// Log a kill that paid no XP, with its `reason`.
///
/// Levels follow the negative-logging convention: DEBUG for outcomes play
/// produces every minute (a mob kills a pet, an NPC fight, an orphaned
/// pet's last hit), WARN only for `transfer_xp_invalid` and `xp_overflow`,
/// data faults no client can cause. Rows about a pet go on `pets.credit`
/// with the owner's identity; rows with no pet (an NPC fight, a player
/// kill that overflows) stay on the module target.
/// `CreditRefused` logs nothing: `credit_recipient` already wrote its WARN.
fn log_no_kill_xp(
    no_xp: NoKillXp,
    target_eid: u32,
    attacker_id: u32,
    base_xp: u64,
    space_mgr: &SpaceManager,
) {
    if no_xp == NoKillXp::CreditRefused {
        return;
    }
    let reason = no_xp.reason();
    let victim = space_mgr.get_entity(target_eid);
    let victim_pet_owner = victim.and_then(|v| v.pet.as_ref()).map(|p| p.owner_id);
    let attacker_pet_owner = space_mgr
        .get_entity(attacker_id)
        .and_then(|e| e.pet.as_ref())
        .map(|p| p.owner_id);
    // The owner whose support question this row answers: the pet killer's
    // owner, else the dead pet's owner.
    let owner_id = attacker_pet_owner.or(victim_pet_owner);

    // A mob finishing a pet is the same `NpcAttacker` outcome, named for
    // the support question it answers.
    let reason = if no_xp == NoKillXp::NpcAttacker && victim_pet_owner.is_some() {
        "npc_killed_pet"
    } else {
        reason
    };
    let pet_id = if attacker_pet_owner.is_some() {
        Some(attacker_id)
    } else if victim_pet_owner.is_some() {
        Some(target_eid)
    } else {
        None
    };
    // Only the summon-time capture names the player: it survives owner-id
    // reuse. Once the registry has dropped the pet (`forget_pet` removes
    // the capture) the identity is unknown and the fields are omitted. Never
    // fall back to whoever holds `owner_id` now: after a reuse that is a
    // different player, and the row would blame them (#889).
    let id = match pet_id {
        Some(pet) if space_mgr.pets.is_pet(pet) => space_mgr.pets.summoner_identity(pet),
        _ => PlayerIdentity::UNKNOWN,
    };
    let (account_id, player_id) = (id.account_id, id.player_id);

    let bad_data = match no_xp {
        NoKillXp::TransferXpInvalid(t) | NoKillXp::XpOverflow(t) => Some(t),
        _ => None,
    };
    if let (Some(transfer_xp), Some(_)) = (bad_data, attacker_pet_owner) {
        tracing::warn!(
            target: "pets.credit",
            event = "kill_xp_not_granted",
            reason,
            entity_id = pet_id,
            pet_id,
            owner_id,
            account_id,
            player_id,
            attacker = attacker_id,
            victim_id = target_eid,
            base_xp,
            transfer_xp,
            max_kill_xp = MAX_KILL_XP,
            "pet kill paid no XP: transfer_xp is out of range (not positive and finite, or the payout overflows)"
        );
    } else if bad_data.is_some() {
        // A player's (scale 1.0) kill whose `kill_xp` alone overflows: a
        // corrupt victim level. No pet, so the module target.
        tracing::warn!(
            event = "kill_xp_not_granted",
            reason,
            attacker = attacker_id,
            victim_id = target_eid,
            base_xp,
            max_kill_xp = MAX_KILL_XP,
            "Kill XP not granted: the payout exceeds the XP ceiling"
        );
    } else if owner_id.is_some() {
        tracing::debug!(
            target: "pets.credit",
            event = "kill_xp_not_granted",
            reason,
            entity_id = pet_id,
            pet_id,
            owner_id,
            account_id,
            player_id,
            attacker = attacker_id,
            victim_id = target_eid,
            base_xp,
            "kill involving a pet paid no XP"
        );
    } else {
        tracing::debug!(
            event = "kill_xp_not_granted",
            reason,
            attacker = attacker_id,
            victim_id = target_eid,
            base_xp,
            "Kill XP not granted"
        );
    }
}

/// Send `onBeginAidWait` so a dead player sees the Defeat Window with the
/// respawner list for their world.
///
/// Reference: python/cell/SGWPlayer.py:1278 —
/// `self.client.onBeginAidWait(100, respawnerList)`.
pub(super) async fn send_begin_aid_wait(
    target_eid: u32,
    attacker_id: u32,
    ability_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Look up respawners for the player's current world
    let world_name = space_mgr.get_entity_world_name(target_eid);
    let matching_respawners: Vec<_> = if let Some(ref wn) = world_name {
        space_mgr
            .respawners
            .iter()
            .filter(|r| r.world_name == *wn)
            .collect()
    } else {
        vec![]
    };

    let (px, py, pz) = space_mgr
        .get_entity(target_eid)
        .map_or((0.0, 0.0, 0.0), |p| {
            (p.position.x, p.position.y, p.position.z)
        });
    let killer_name = space_mgr
        .get_entity(attacker_id)
        .and_then(|k| k.npc_name.clone().or_else(|| k.character_name.clone()))
        .unwrap_or_default();
    let id = space_mgr.player_identity(target_eid);
    tracing::info!(
        target: "player.death",
        entity_id = target_eid,
        account_id = id.account_id,
        player_id = id.player_id,
        killer = attacker_id,
        killer_name = %killer_name,
        ability_id = ability_id.unwrap_or(-1),
        world = ?world_name,
        x = px,
        y = py,
        z = pz,
        "player death"
    );

    let mut aid_args = Vec::with_capacity(64);
    // INT32: TimeToAid (seconds until auto-respawn)
    aid_args.extend_from_slice(&30i32.to_le_bytes());

    if matching_respawners.is_empty() {
        // Fallback: single entry with chardef spawn position
        aid_args.extend_from_slice(&1u32.to_le_bytes()); // array count
        aid_args.extend_from_slice(&0i32.to_le_bytes()); // respawnerID = 0 (default)
        crate::mercury::write_wstring(&mut aid_args, "Respawn Point");
    } else {
        aid_args.extend_from_slice(&(matching_respawners.len() as u32).to_le_bytes());
        for resp in &matching_respawners {
            aid_args.extend_from_slice(&resp.respawner_id.to_le_bytes());
            crate::mercury::write_wstring(&mut aid_args, &resp.name);
        }
    }

    send_entity_method(
        target_eid,
        crate::mercury::method_idx::ON_BEGIN_AID_WAIT,
        aid_args,
        tx,
        space_mgr,
    )
    .await;
    tracing::info!(
        target = target_eid,
        world = ?world_name,
        respawner_count = if matching_respawners.is_empty() { 1 } else { matching_respawners.len() },
        respawner_ids = ?matching_respawners.iter().map(|r| r.respawner_id).collect::<Vec<_>>(),
        filter = "world_name_only",
        "Sent onBeginAidWait (Defeat Window)"
    );
    crate::cell::player_journal::note(
        target_eid,
        crate::cell::player_journal::kinds::DEATH,
        format!("world={world_name:?}"),
    );
}
