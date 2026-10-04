//! `.dummy [hostile|friendly|clear] [templateId]` — a lab target that holds
//! still and never fights back (ability-mechanics AB-L2, D-AU6).
//!
//! `.dummy` (hostile) and `.dummy friendly` place one template NPC three
//! metres in front of the caller, facing them, with:
//!
//! - the [`LabDummy`] mark, which keeps it out of the AI tick: no attack, no
//!   chase, no leash, however much threat it takes;
//! - [`LAB_DUMMY_HEALTH`] Health, current and max;
//! - the template's own Defense and Accuracy, read back in the feedback line
//!   (and in `.effects` / `server_ability_state`) so a QR expectation can be
//!   computed;
//! - an aggression override (`hostile` or `friendly`), no respawn and no
//!   `spawnlist` row.
//!
//! It despawns [`LAB_DUMMY_LIFETIME`] after placement ([`lab_dummy_tick`]) or
//! when its owner logs out ([`despawn_lab_dummies_of`]). `.dummy clear`
//! removes the caller's own dummies and no one else's (colo rule: a GM
//! touches only what their own lab characters spawned), and a GM may have at
//! most [`LAB_DUMMY_MAX_PER_OWNER`] standing.

use std::time::Instant;

use cimmeria_entity::cell_entity::MobAggression;
use cimmeria_entity::stats::{ACCURACY, DEFENSE, HEALTH};
use tokio::sync::mpsc;

use crate::cell::console::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{
    DespawnOutcome, LabDummy, SpaceManager, LAB_DUMMY_HEALTH, LAB_DUMMY_LIFETIME,
    LAB_DUMMY_MAX_PER_OWNER,
};

/// The template a dummy uses when none is named: 34, "SGC Jaffa", a plain
/// humanoid body with no ability set.
pub(crate) const DEFAULT_DUMMY_TEMPLATE: i32 = 34;

/// How far in front of the caller a dummy is placed, in metres.
const PLACE_DISTANCE: f32 = 3.0;

/// The `tag` every dummy carries, so `.info` and the bookmark rows name it.
pub(crate) const DUMMY_TAG: &str = "lab_dummy";

const USAGE: &str = ".dummy: usage .dummy [hostile|friendly] [templateId] | .dummy clear";

/// Why a dummy went away: the `reason` of its `lab_dummy_despawned` row.
#[derive(Clone, Copy)]
enum Gone {
    Cleared,
    Expired,
    OwnerLogout,
}

impl Gone {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cleared => "cleared",
            Self::Expired => "expired",
            Self::OwnerLogout => "owner_logout",
        }
    }
}

pub(super) async fn run(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (disposition, template_arg) = match args {
        ["clear"] => return clear(caller_id, tx, space_mgr).await,
        [] => (MobAggression::Hostile, None),
        [word] | [word, _] if parse_disposition(word).is_some() => (
            parse_disposition(word).unwrap_or(MobAggression::Hostile),
            args.get(1),
        ),
        _ => return send_gm_feedback(caller_id, USAGE, tx).await,
    };
    let template_id = match template_arg {
        None => DEFAULT_DUMMY_TEMPLATE,
        Some(t) => match t.parse::<i32>() {
            Ok(id) if id > 0 => id,
            _ => {
                let line = format!(".dummy: templateId must be a positive integer (got {t})");
                return send_gm_feedback(caller_id, &line, tx).await;
            }
        },
    };
    place(caller_id, disposition, template_id, tx, space_mgr).await;
}

fn parse_disposition(word: &str) -> Option<MobAggression> {
    match word.to_ascii_lowercase().as_str() {
        "hostile" => Some(MobAggression::Hostile),
        "friendly" => Some(MobAggression::Friendly),
        _ => None,
    }
}

async fn place(
    caller_id: u32,
    disposition: MobAggression,
    template_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let standing = space_mgr.lab_dummies_of(caller_id).len();
    if standing >= LAB_DUMMY_MAX_PER_OWNER {
        let line = format!(
            ".dummy: you already have {standing} dummies (the most is {LAB_DUMMY_MAX_PER_OWNER}); .dummy clear removes them"
        );
        return send_gm_feedback(caller_id, &line, tx).await;
    }
    let Some((space_id, world_name, position, heading)) = placement(caller_id, space_mgr) else {
        return send_gm_feedback(caller_id, ".dummy: you are not in a space", tx).await;
    };
    let dummy_id = match space_mgr.spawn_npc_from_template(
        template_id,
        space_id,
        &world_name,
        position,
        heading,
        DUMMY_TAG,
        true,
        Some(disposition),
    ) {
        Ok(id) => id,
        Err(e) => {
            tracing::debug!(
                target: "abilities.gm",
                event = "lab_dummy_refused",
                reason = "spawn_failed",
                entity_id = caller_id,
                template_id,
                error = %e,
                "GM .dummy refused: the template did not spawn"
            );
            return send_gm_feedback(caller_id, &format!(".dummy: {e}"), tx).await;
        }
    };
    let owner_identity = space_mgr.player_identity(caller_id);
    let Some(dummy) = space_mgr.get_entity_mut(dummy_id) else {
        return;
    };
    if let Some(h) = dummy.stats.get_mut(HEALTH) {
        h.update(0, LAB_DUMMY_HEALTH, LAB_DUMMY_HEALTH);
    }
    dummy.extensions.insert(LabDummy {
        owner_id: caller_id,
        owner_identity,
        disposition,
        expires_at: Instant::now() + LAB_DUMMY_LIFETIME,
    });
    let stat = |id: i32| dummy.stats.get(id).map_or(0, |s| s.cur);
    let (defense, accuracy) = (stat(DEFENSE), stat(ACCURACY));
    let name = dummy.npc_name.clone().unwrap_or_default();
    tracing::info!(
        target: "abilities.gm",
        event = "lab_dummy_spawned",
        entity_id = caller_id,
        account_id = owner_identity.account_id,
        player_id = owner_identity.player_id,
        dummy_id,
        template_id,
        disposition = disposition.label(),
        health = LAB_DUMMY_HEALTH,
        defense,
        accuracy,
        space_id,
        lifetime_secs = LAB_DUMMY_LIFETIME.as_secs(),
        "GM placed a lab dummy"
    );
    let line = format!(
        "dummy [{dummy_id}] placed: {} {name} (template {template_id}), Health {LAB_DUMMY_HEALTH}, Defense {defense}, Accuracy {accuracy}; it never attacks; gone in {} min, when you log out, or on .dummy clear",
        disposition.label(),
        LAB_DUMMY_LIFETIME.as_secs() / 60
    );
    send_gm_feedback(caller_id, &line, tx).await;
}

/// The caller's space and world, a point [`PLACE_DISTANCE`] in front of them,
/// and the heading that faces back at them. Facing follows the NPC movement
/// convention: yaw is `direction.y`, forward is `(sin yaw, 0, cos yaw)`.
fn placement(caller_id: u32, space_mgr: &SpaceManager) -> Option<(u32, String, [f32; 3], f32)> {
    let space_id = space_mgr.get_entity_space_id(caller_id)?;
    let world_name = space_mgr.spaces.get(&space_id)?.world_name.clone();
    let e = space_mgr.get_entity(caller_id)?;
    let yaw = e.direction.y;
    let yaw = if yaw.is_finite() { yaw } else { 0.0 };
    let position = [
        e.position.x + PLACE_DISTANCE * yaw.sin(),
        e.position.y,
        e.position.z + PLACE_DISTANCE * yaw.cos(),
    ];
    let heading = (yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
    Some((space_id, world_name, position, heading))
}

async fn clear(caller_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let mine = space_mgr.lab_dummies_of(caller_id);
    let mut removed = 0;
    for id in &mine {
        removed += usize::from(despawn(*id, Gone::Cleared, tx, space_mgr).await);
    }
    let line = if mine.is_empty() {
        "dummy clear: you have no dummies standing".to_string()
    } else {
        format!("dummy clear: {removed} of your dummies removed")
    };
    send_gm_feedback(caller_id, &line, tx).await;
}

/// Despawn the dummies whose lifetime ran out, telling each online owner.
/// Called from the cell loop about once a second.
pub async fn lab_dummy_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    for id in space_mgr.expired_lab_dummies(Instant::now()) {
        let owner = space_mgr
            .get_entity(id)
            .and_then(|e| e.extensions.get::<LabDummy>())
            .map(|d| d.owner_id);
        if despawn(id, Gone::Expired, tx, space_mgr).await {
            if let Some(owner) = owner.filter(|&o| space_mgr.get_entity(o).is_some()) {
                let line = format!("dummy [{id}] despawned: its 10 minutes are up");
                send_gm_feedback(owner, &line, tx).await;
            }
        }
    }
}

/// Despawn every dummy `owner_id` placed. Called when the owner logs out,
/// before their entity is torn down.
pub async fn despawn_lab_dummies_of(
    owner_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for id in space_mgr.lab_dummies_of(owner_id) {
        despawn(id, Gone::OwnerLogout, tx, space_mgr).await;
    }
}

/// Despawn one dummy and log it. Returns whether it was removed.
async fn despawn(
    dummy_id: u32,
    why: Gone,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(mark) = space_mgr
        .get_entity(dummy_id)
        .and_then(|e| e.extensions.get::<LabDummy>())
        .copied()
    else {
        return false;
    };
    let outcome = space_mgr.despawn_npc(dummy_id, tx).await;
    let (removed, witnesses) = match outcome {
        DespawnOutcome::Despawned { witnesses_notified } => (true, witnesses_notified),
        DespawnOutcome::NotFound | DespawnOutcome::RefusedPlayer => (false, 0),
    };
    tracing::info!(
        target: "abilities.gm",
        event = "lab_dummy_despawned",
        reason = why.as_str(),
        entity_id = mark.owner_id,
        account_id = mark.owner_identity.account_id,
        player_id = mark.owner_identity.player_id,
        dummy_id,
        removed,
        witnesses_notified = witnesses,
        "lab dummy despawned"
    );
    removed
}
