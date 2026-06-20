//! Parse / validate / route one GM `.`-console line.
//!
//! [`handle_console_command`] is the channel entry point called from
//! [`crate::cell::chat`]; [`resolve_target`] enforces the per-command target
//! contract and [`exec`] routes a validated command to its family handler.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::registry::{Spec, Target, COMMANDS};
use super::send_gm_feedback;
use super::{crafting, entity, mission, net, patrol, query, seed, server, spawn, stats};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Parse, validate, and dispatch one GM `.`-console line.
///
/// `text` is the raw chat body including the leading `.`. The caller is already
/// confirmed `access_level >= GameMaster` by the channel gate in
/// [`crate::cell::chat`]. Every accepted command is logged at `info` for the
/// audit trail; validation failures reply to the GM via [`feedback`] and abort.
pub(crate) async fn handle_console_command(
    caller_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let body = text.strip_prefix('.').unwrap_or(text);
    let parts: Vec<&str> = body.split_whitespace().collect();
    let Some(&name) = parts.first() else {
        send_gm_feedback(caller_id, "Empty command. Try .help", tx).await;
        return;
    };
    let args: Vec<&str> = parts[1..].to_vec();

    let Some(spec) = COMMANDS.iter().find(|c| c.name == name) else {
        send_gm_feedback(
            caller_id,
            &format!("Unknown command: .{name} (try .help)"),
            tx,
        )
        .await;
        return;
    };

    if args.len() < spec.min {
        send_gm_feedback(
            caller_id,
            &format!(
                ".{name}: not enough arguments (got {}, need at least {})",
                args.len(),
                spec.min
            ),
            tx,
        )
        .await;
        return;
    }
    if args.len() > spec.max {
        send_gm_feedback(
            caller_id,
            &format!(
                ".{name}: too many arguments (got {}, at most {})",
                args.len(),
                spec.max
            ),
            tx,
        )
        .await;
        return;
    }

    // Resolve + validate the selected target (if the command requires one).
    let target_id = match resolve_target(caller_id, spec, space_mgr) {
        Ok(t) => t,
        Err(msg) => {
            send_gm_feedback(caller_id, &format!(".{name}: {msg}"), tx).await;
            return;
        }
    };

    // Audit trail: log only AFTER the command passes arg-count + target
    // validation, so the "accepted" event marks commands we actually dispatch
    // (not malformed/wrong-target ones rejected above). Record both the caller
    // and their server-side `access_level` so the audit line attributes the
    // command to a privilege level, not just an entity id. Command args may
    // contain names/positions but never secrets, so logging the name + count
    // (not the raw text) is the right privacy/observability balance — mirrors
    // the chat.send span policy.
    let access_level = space_mgr
        .get_entity(caller_id)
        .map(|e| e.access_level)
        .unwrap_or(0);
    tracing::info!(
        entity_id = caller_id,
        access_level,
        command = name,
        argc = args.len(),
        "GM .-console command accepted"
    );

    // Discord gm-channel audit trail. Attribute to the caller's cached name
    // (threaded in via InitPlayerState); fall back to the entity id if the
    // name isn't cached yet. Args are name/position tokens, never secrets —
    // the same privacy balance as the audit log above.
    let gm_name = space_mgr
        .get_entity(caller_id)
        .and_then(|e| e.character_name.clone())
        .unwrap_or_else(|| format!("entity:{caller_id}"));
    cimmeria_discord::emit_gm_command(gm_name, format!(".{name}"), args.join(" "));

    exec(name, caller_id, &args, target_id, tx, space_mgr, engine).await;
}

/// Resolve the caller's current target and validate it against `spec.target`.
///
/// - For [`Target::None`] commands, returns the caller's current target if it's
///   set and resolvable (legacy `findEntity(targetId) if targetId else None`),
///   else `None`. A bad/cross-space current target is simply dropped to `None`
///   rather than failing — these commands don't depend on it.
/// - For typed commands, a target is **required**: returns `Err(msg)` (with a
///   GM-facing reason) when there is no current target, it doesn't resolve, it's
///   in another space, or it's the wrong type.
fn resolve_target(
    caller_id: u32,
    spec: &Spec,
    space_mgr: &SpaceManager,
) -> Result<Option<u32>, String> {
    let caller_space = space_mgr.get_entity(caller_id).map(|e| e.space_id.0);
    let current = space_mgr
        .get_entity(caller_id)
        .and_then(|e| e.current_target_id)
        .filter(|&id| id > 0)
        .and_then(|id| u32::try_from(id).ok());

    if spec.target == Target::None {
        // Optional: pass the current target through only if it resolves in the
        // caller's space; otherwise None.
        let resolved =
            current.filter(|&id| space_mgr.get_entity(id).map(|e| e.space_id.0) == caller_space);
        return Ok(resolved);
    }

    let Some(target_id) = current else {
        return Err("a target is required for this command".to_string());
    };
    let Some(target) = space_mgr.get_entity(target_id) else {
        return Err(format!("targeted entity {target_id} is unknown"));
    };
    if Some(target.space_id.0) != caller_space {
        return Err(format!("targeted entity {target_id} is in another space"));
    }
    if !spec.target.matches(target) {
        return Err(format!("expected {} as a target", spec.target.label()));
    }
    Ok(Some(target_id))
}

/// Route a validated command to its family handler. The big match mirrors
/// `gm::dispatch`; each arm receives only the params it needs.
#[allow(clippy::too_many_lines)]
pub(crate) async fn exec(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match name {
        "help" => query::help(caller_id, args, tx).await,
        // authoring commit workflow
        "seedconfirm" => seed::confirm(caller_id, tx, space_mgr).await,
        "seedpending" => seed::pending(caller_id, tx, space_mgr).await,
        "seedcancel" => seed::cancel(caller_id, tx, space_mgr).await,
        // D. search / query
        "searchitem" => query::search_item(caller_id, args, tx, space_mgr).await,
        "searchmission" => query::search_mission(caller_id, args, tx, space_mgr).await,
        "searchtemplate" => query::search_template(caller_id, args, tx, space_mgr).await,
        "players" => query::players(caller_id, tx, space_mgr).await,
        // F. stat dumps
        "primarystats" | "speedstats" | "armorstats" | "qrstats" | "absorbstats"
        | "stealthstats" => stats::show(name, caller_id, target_id, tx, space_mgr).await,
        // A. entity authoring
        "tag" | "name" | "alignment" | "nameid" | "staticmesh" | "bodyset" | "eventset"
        | "interactiontype" | "lookat" | "visible" | "setcombatant" | "unsetcombatant"
        | "addcomponent" | "delcomponent" | "adddialog" | "removedialog" | "dynamicupdate" => {
            entity::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // H. net / debug
        "net_seq" | "net_seqto" | "net_seqfrom" | "net_timer" | "net_mapinfo" | "net_speak"
        | "net_dialog" | "net_challenge" | "debug_velocity" | "debug_controller"
        | "debug_follow" | "threaten" | "aggression" => {
            net::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // E. crafting
        "allcraft" | "learndiscipline" | "forgetdiscipline" => {
            crafting::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // Mission gaps
        "missionfail" => mission::fail(caller_id, args, target_id, tx, space_mgr).await,
        "missionrewards" => mission::rewards(caller_id, args, target_id, tx, space_mgr).await,
        // G. server / maintenance
        "save" | "reloadmap" | "reloadres" | "removerespawner" | "loglevel" | "logclient" => {
            server::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        // B. spawn authoring
        "savespawn" | "delspawn" | "autosavespawn" | "respawnall" | "spawnrandom" => {
            spawn::dispatch(name, caller_id, args, target_id, tx, space_mgr, engine).await
        }
        // C. patrol authoring
        "path_add" | "path_show" | "path_clear" | "path_assign" | "path_unassign"
        | "path_set_seq" | "path_clear_seq" | "path_set_tp" | "path_clear_tp"
        | "path_set_tp_seq" | "path_set_tp_delay" => {
            patrol::dispatch(name, caller_id, args, target_id, tx, space_mgr).await
        }
        other => {
            // Unreachable in practice — every COMMANDS entry has an arm above,
            // pinned by `tests::every_spec_is_dispatched`.
            send_gm_feedback(caller_id, &format!(".{other}: not implemented"), tx).await;
        }
    }
}
