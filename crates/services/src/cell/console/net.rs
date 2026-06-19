//! Low-level net / AI debug commands (category H): kismet sequence play,
//! client timers/map-info/challenge, make-an-entity-speak, debug controllers,
//! and the threat/aggression pokes.
//!
//! Most are thin wrappers over an existing client method (serialized here from
//! the wire shapes in `docs/protocol/client-method-dispatch-table.md`) or an
//! existing cell system (threat list, follow AI, dialog display).
//!
//! Legacy reference: `deprecated/python/cell/commands/Net.py` +
//! `deprecated/python/cell/commands/Misc.py` (`debug_*`) +
//! `deprecated/python/cell/commands/Entity.py` (`threaten`/`aggression`).

use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::abilities::send_entity_method_to_self_and_witnesses;
use crate::cell::interactions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::{ON_PLAYER_COMMUNICATION, ON_SEQUENCE};

/// Client method indices from `docs/protocol/client-method-dispatch-table.md`
/// (SGWPlayer ClientMethods), for the handful not in `method_idx`.
const ON_TIMER_UPDATE: u16 = 12;
const ON_MAP_INFO: u16 = 123;
const ON_CLIENT_CHALLENGE: u16 = 142;

/// Feedback channel id (`CHAN_FEEDBACK`); `net_speak`'s default speak channel is
/// say (`0`).
const CHAN_SAY: u8 = 0;

pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "net_seq" => sequence(caller_id, target_id, args, SeqDir::OnTarget, tx, space_mgr).await,
        "net_seqto" => {
            sequence(
                caller_id,
                target_id,
                args,
                SeqDir::CallerToTarget,
                tx,
                space_mgr,
            )
            .await
        }
        "net_seqfrom" => {
            sequence(
                caller_id,
                target_id,
                args,
                SeqDir::TargetToCaller,
                tx,
                space_mgr,
            )
            .await
        }
        "net_timer" => timer(caller_id, args, tx).await,
        "net_mapinfo" => map_info(caller_id, target_id, args, tx, space_mgr).await,
        "net_speak" => speak(caller_id, target_id, args, tx, space_mgr).await,
        "net_dialog" => dialog(caller_id, target_id, args, tx, space_mgr).await,
        "net_challenge" => challenge(caller_id, args, tx).await,
        "debug_velocity" => debug_velocity(caller_id, target_id, args, tx, space_mgr).await,
        "debug_controller" => {
            send_gm_feedback(
                caller_id,
                "debug_controller: no debug movement controller in the Rust server (no-op).",
                tx,
            )
            .await;
        }
        "debug_follow" => debug_follow(caller_id, target_id, tx, space_mgr).await,
        "threaten" => threaten(caller_id, target_id, args, tx, space_mgr).await,
        "aggression" => aggression(caller_id, target_id, args, tx, space_mgr).await,
        _ => {}
    }
}

enum SeqDir {
    /// Source and target are both the selected entity.
    OnTarget,
    /// Source is the caller, target is the selected entity (or caller).
    CallerToTarget,
    /// Source is the selected entity, target is the caller.
    TargetToCaller,
}

/// Build the `onSequence` arg block. Minimal NameValuePair array (empty) and
/// `ImpactTime = 0` — enough to fire the kismet sequence on the client.
fn build_sequence_args(seq_id: i32, source_id: u32, target_id: u32, view_type: i8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(28);
    buf.extend_from_slice(&seq_id.to_le_bytes());
    buf.extend_from_slice(&(source_id as i32).to_le_bytes());
    buf.extend_from_slice(&(target_id as i32).to_le_bytes());
    buf.push(1); // PrimaryTarget = true
    buf.extend_from_slice(&0f32.to_le_bytes()); // ImpactTime
    buf.extend_from_slice(&0u32.to_le_bytes()); // NameValuePair count = 0
    buf.push(view_type as u8); // ViewType
    buf.extend_from_slice(&0i32.to_le_bytes()); // InstanceId
    buf
}

/// `.net_seq` / `.net_seqto` / `.net_seqfrom` — play a kismet sequence
/// (`onSequence`, client method 1) on a source→target pair, fanned to the
/// broadcaster's own client + witnesses.
async fn sequence(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    dir: SeqDir,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(seq_id) = super::parse_i32(caller_id, args, 0, "sequenceId", tx).await else {
        return;
    };
    let view_type = match args.get(1) {
        Some(s) => match s.parse::<i8>() {
            Ok(v) => v,
            Err(_) => {
                send_gm_feedback(caller_id, "viewType must be an integer.", tx).await;
                return;
            }
        },
        None => 0,
    };
    let target = target_id.unwrap_or(caller_id);
    let (source, dest, broadcaster) = match dir {
        SeqDir::OnTarget => (target, target, target),
        SeqDir::CallerToTarget => (caller_id, target, caller_id),
        SeqDir::TargetToCaller => (target, caller_id, target),
    };
    let args_buf = build_sequence_args(seq_id, source, dest, view_type);
    send_entity_method_to_self_and_witnesses(broadcaster, ON_SEQUENCE, args_buf, tx, space_mgr)
        .await;
    send_gm_feedback(
        caller_id,
        &format!("net_seq {seq_id} (view {view_type}) {source} -> {dest}"),
        tx,
    )
    .await;
}

/// `.net_timer <id> <type> [totalTime] [secondaryId]` — start a client timer
/// (`onTimerUpdate`, client method 12) on the caller.
async fn timer(caller_id: u32, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let Some(id) = super::parse_i32(caller_id, args, 0, "timer id", tx).await else {
        return;
    };
    let Some(ty) = super::parse_i32(caller_id, args, 1, "timer type", tx).await else {
        return;
    };
    // `Type` is a UINT8 on the wire — reject out-of-range so the byte isn't
    // silently truncated into a different timer type.
    let Ok(ty) = u8::try_from(ty) else {
        send_gm_feedback(caller_id, "net_timer: type must be 0-255.", tx).await;
        return;
    };
    let total_time = match args.get(2) {
        Some(s) => s
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
            .unwrap_or(1.0),
        None => 1.0,
    };
    let mut buf = Vec::with_capacity(17);
    buf.extend_from_slice(&id.to_le_bytes());
    buf.push(ty);
    buf.extend_from_slice(&(caller_id as i32).to_le_bytes()); // SourceID
    buf.extend_from_slice(&total_time.to_le_bytes());
    buf.extend_from_slice(&total_time.to_le_bytes()); // BigWorldTimeComplete (relative)
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: caller_id,
            method_index: ON_TIMER_UPDATE,
            args: buf,
        })
        .await;
    send_gm_feedback(
        caller_id,
        &format!("net_timer {id} (type {ty}, {total_time}s)"),
        tx,
    )
    .await;
}

/// `.net_mapinfo <sysId> <keyId> <lifetime> [delete] [sysTypeId]` — send
/// `onMapInfo` (client method 123) to the target, located at the caller.
async fn map_info(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(sys_id) = super::parse_i32(caller_id, args, 0, "sysId", tx).await else {
        return;
    };
    let Some(key_id) = super::parse_i32(caller_id, args, 1, "keyId", tx).await else {
        return;
    };
    let Some(lifetime) = super::parse_i32(caller_id, args, 2, "lifetime", tx).await else {
        return;
    };
    let delete = match args.get(3) {
        Some(s) => *s == "1" || s.eq_ignore_ascii_case("true"),
        None => false,
    };
    let sys_type_id = args.get(4).and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let recipient = target_id.unwrap_or(caller_id);
    let (pos, world_id) = space_mgr
        .get_entity(caller_id)
        .map(|e| (e.position, e.space_id.0))
        .unwrap_or((cimmeria_common::Vector3::zero(), 0));

    let mut buf = Vec::with_capacity(33);
    buf.extend_from_slice(&(sys_type_id as u32).to_le_bytes());
    buf.extend_from_slice(&(sys_id as u32).to_le_bytes());
    buf.extend_from_slice(&(key_id as u32).to_le_bytes());
    buf.extend_from_slice(&world_id.to_le_bytes());
    buf.extend_from_slice(&pos.x.to_le_bytes());
    buf.extend_from_slice(&pos.y.to_le_bytes());
    buf.extend_from_slice(&pos.z.to_le_bytes());
    buf.extend_from_slice(&(lifetime as u32).to_le_bytes());
    buf.push(u8::from(delete));
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: recipient,
            method_index: ON_MAP_INFO,
            args: buf,
        })
        .await;
    send_gm_feedback(
        caller_id,
        &format!("net_mapinfo sys={sys_id} key={key_id} life={lifetime} del={delete}"),
        tx,
    )
    .await;
}

/// `.net_speak <message> [channel]` — make the target speak a message on a chat
/// channel, fanned to its witnesses.
async fn speak(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "net_speak: a target is required.", tx).await;
        return;
    };
    // First arg is the message; an optional trailing channel name. The legacy
    // took a single message word — we accept the message as one token (use
    // quotes-free single words; multi-word handled by joining all-but-channel
    // is ambiguous, so keep it simple: message is arg[0], channel is arg[1]).
    let Some(message) = args.first().copied() else {
        send_gm_feedback(caller_id, "net_speak: a message is required.", tx).await;
        return;
    };
    let channel = match args.get(1).map(|s| s.to_ascii_lowercase()) {
        None => CHAN_SAY,
        Some(c) => match c.as_str() {
            "say" => 0,
            "emote" => 1,
            "yell" => 2,
            "team" => 3,
            "squad" => 4,
            other => {
                send_gm_feedback(
                    caller_id,
                    &format!("net_speak: unknown channel '{other}'"),
                    tx,
                )
                .await;
                return;
            }
        },
    };
    let speaker = space_mgr
        .get_entity(target)
        .and_then(|e| e.npc_name.clone())
        .unwrap_or_else(|| format!("Entity {target}"));

    let mut buf = Vec::new();
    crate::mercury::write_wstring(&mut buf, &speaker);
    buf.push(0); // SpeakerFlags
    buf.push(channel);
    crate::mercury::write_wstring(&mut buf, message);
    send_entity_method_to_self_and_witnesses(target, ON_PLAYER_COMMUNICATION, buf, tx, space_mgr)
        .await;
    send_gm_feedback(
        caller_id,
        &format!("net_speak [{target}]: \"{message}\""),
        tx,
    )
    .await;
}

/// `.net_dialog <dialogId>` — open a dialog with the target NPC for the caller.
async fn dialog(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "net_dialog: a target is required.", tx).await;
        return;
    };
    let Some(dialog_id) = super::parse_i32(caller_id, args, 0, "dialogId", tx).await else {
        return;
    };
    interactions::send_dialog_display(caller_id, target as i32, dialog_id, tx, space_mgr).await;
    send_gm_feedback(
        caller_id,
        &format!("net_dialog {dialog_id} with [{target}]"),
        tx,
    )
    .await;
}

/// `.net_challenge <challenge> <type> <object> <id1> <id2>` — send
/// `onClientChallenge` (client method 142) to the caller.
async fn challenge(caller_id: u32, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let Some(challenge) = super::parse_i32(caller_id, args, 0, "challenge", tx).await else {
        return;
    };
    let Some(ty) = super::parse_i32(caller_id, args, 1, "type", tx).await else {
        return;
    };
    let Some(object) = args.get(2).copied() else {
        send_gm_feedback(caller_id, "net_challenge: object arg is required.", tx).await;
        return;
    };
    let Some(id1) = super::parse_i32(caller_id, args, 3, "id1", tx).await else {
        return;
    };
    let Some(id2) = super::parse_i32(caller_id, args, 4, "id2", tx).await else {
        return;
    };
    let mut buf = Vec::new();
    buf.extend_from_slice(&challenge.to_le_bytes());
    buf.extend_from_slice(&ty.to_le_bytes());
    crate::mercury::write_wstring(&mut buf, object);
    buf.extend_from_slice(&id1.to_le_bytes());
    buf.extend_from_slice(&id2.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: caller_id,
            method_index: ON_CLIENT_CHALLENGE,
            args: buf,
        })
        .await;
    send_gm_feedback(
        caller_id,
        &format!("net_challenge {challenge} type {ty}"),
        tx,
    )
    .await;
}

/// `.debug_velocity <x> <y> <z>` — set the target's velocity (in-memory).
async fn debug_velocity(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(
            caller_id,
            "debug_velocity: select a target entity first.",
            tx,
        )
        .await;
        return;
    };
    let Some(x) = super::parse_f32(caller_id, args, 0, "velocityX", tx).await else {
        return;
    };
    let Some(y) = super::parse_f32(caller_id, args, 1, "velocityY", tx).await else {
        return;
    };
    let Some(z) = super::parse_f32(caller_id, args, 2, "velocityZ", tx).await else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.velocity = [x, y, z];
    }
    send_gm_feedback(
        caller_id,
        &format!("debug_velocity [{target}] = ({x}, {y}, {z})"),
        tx,
    )
    .await;
}

/// `.debug_follow` — toggle a follow action on the target (it follows the
/// caller). Re-running cancels the follow.
async fn debug_follow(
    caller_id: u32,
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        return;
    };
    let now_following = if let Some(e) = space_mgr.get_entity_mut(target) {
        if e.follow_target_id == Some(caller_id) {
            e.follow_target_id = None;
            e.ai_state = AiState::Idle;
            false
        } else {
            e.follow_target_id = Some(caller_id);
            e.ai_state = AiState::Follow;
            true
        }
    } else {
        return;
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "debug_follow [{target}]: {}",
            if now_following {
                "following you"
            } else {
                "cancelled"
            }
        ),
        tx,
    )
    .await;
}

/// `.threaten <amount>` — generate threat on the targeted mob from the caller.
async fn threaten(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        return;
    };
    let Some(amount) = super::parse_f32(caller_id, args, 0, "threat", tx).await else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        *e.threat_list.entry(caller_id).or_insert(0.0) += amount;
    }
    send_gm_feedback(caller_id, &format!("threaten [{target}] += {amount}"), tx).await;
}

/// `.aggression <level>` — set the targeted mob's aggression level.
async fn aggression(
    caller_id: u32,
    target_id: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        return;
    };
    let Some(level) = super::parse_i32(caller_id, args, 0, "aggression", tx).await else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.aggression = level;
    }
    send_gm_feedback(caller_id, &format!("aggression [{target}] = {level}"), tx).await;
}
