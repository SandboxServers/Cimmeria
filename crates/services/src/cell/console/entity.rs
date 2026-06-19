//! Live entity / content authoring (category A): edit a spawned entity's
//! authored properties in memory and (where a clean wire path exists) re-push to
//! the AoI.
//!
//! Most of these mutate `CellEntity` fields and are **in-memory only** — the
//! change is visible immediately to anyone whose AoI re-enters the entity, but
//! is not persisted (pair with `.savespawn` for that). Appearance-affecting
//! edits on a player re-broadcast `BeingAppearance` via `RefreshAppearance`;
//! NPC appearance edits surface on the next AoI entry. Commands document their
//! own persistence in the feedback line.
//!
//! Legacy reference: `deprecated/python/cell/commands/Entity.py` +
//! `deprecated/python/cell/commands/Player.py` (`addDialog`/`removeDialog`/
//! `dynamicUpdate`).

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Route a category-A entity-authoring command to its body. `target_id` is the
/// validated selected entity (always `Some` here — every command in this family
/// requires a `Spawnable`/`Being` target).
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, &format!(".{name}: a target is required."), tx).await;
        return;
    };
    // `target_id` was resolved at parse time; confirm the entity still exists so
    // a handler doesn't silently no-op on `get_entity_mut` yet report success.
    if space_mgr.get_entity(target).is_none() {
        send_gm_feedback(
            caller_id,
            &format!(".{name}: target [{target}] no longer exists."),
            tx,
        )
        .await;
        return;
    }
    match name {
        "tag" => set_tag(caller_id, target, args, tx, space_mgr).await,
        "name" => set_name(caller_id, target, args, tx, space_mgr).await,
        "alignment" => set_alignment(caller_id, target, args, tx, space_mgr).await,
        "nameid" => set_i32_field(caller_id, target, args, tx, space_mgr, "nameid").await,
        "staticmesh" => set_str_field(caller_id, target, args, tx, space_mgr, "staticmesh").await,
        "bodyset" => set_str_field(caller_id, target, args, tx, space_mgr, "bodyset").await,
        "eventset" => set_i32_field(caller_id, target, args, tx, space_mgr, "eventset").await,
        "interactiontype" => set_interaction_type(caller_id, target, args, tx, space_mgr).await,
        "lookat" => look_at(caller_id, target, tx, space_mgr).await,
        "visible" => set_visible(caller_id, target, args, tx, space_mgr).await,
        "setcombatant" => set_combatant(caller_id, target, args, true, tx, space_mgr).await,
        "unsetcombatant" => set_combatant(caller_id, target, args, false, tx, space_mgr).await,
        "addcomponent" => component(caller_id, target, args, true, tx, space_mgr).await,
        "delcomponent" => component(caller_id, target, args, false, tx, space_mgr).await,
        "adddialog" => dialog(caller_id, target, args, true, tx, space_mgr).await,
        "removedialog" => dialog(caller_id, target, args, false, tx, space_mgr).await,
        "dynamicupdate" => dynamic_update(caller_id, target, tx, space_mgr).await,
        _ => {}
    }
}

/// `.tag <tag|none>` — set the content tag (`'none'` clears). The tag is what
/// content-engine chains match on via `find_entity_by_tag`.
async fn set_tag(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let tag = args[0];
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.tag = if tag == "none" {
            None
        } else {
            Some(tag.to_string())
        };
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "tag [{target}] = {} (in-memory; .savespawn to persist)",
            tag
        ),
        tx,
    )
    .await;
}

/// `.name <words...>` — set the NPC display name.
async fn set_name(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let name = args.join(" ");
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.npc_name = Some(name.clone());
    }
    send_gm_feedback(
        caller_id,
        &format!("name [{target}] = '{name}' (in-memory; shown on next AoI entry)"),
        tx,
    )
    .await;
}

/// `.alignment <undefined|praxis|sgu>` — set the alignment id. Names map to the
/// legacy `ALIGNMENT_*` order (Undefined=0, Praxis=1, SGU=2).
async fn set_alignment(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let value = match args[0].to_ascii_lowercase().as_str() {
        "undefined" => 0u8,
        "praxis" => 1,
        "sgu" => 2,
        other => {
            send_gm_feedback(
                caller_id,
                &format!("alignment: invalid '{other}' (use undefined|praxis|sgu)"),
                tx,
            )
            .await;
            return;
        }
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.alignment = value;
    }
    send_gm_feedback(
        caller_id,
        &format!("alignment [{target}] = {} ({value})", args[0]),
        tx,
    )
    .await;
}

/// `.nameid` / `.eventset` — set an `i32` template field by command name.
async fn set_i32_field(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    field: &str,
) {
    let Some(value) = super::parse_i32(caller_id, args, 0, field, tx).await else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        match field {
            "nameid" => e.name_id = Some(value),
            "eventset" => e.event_set_id = Some(value),
            _ => {}
        }
    }
    send_gm_feedback(
        caller_id,
        &format!("{field} [{target}] = {value} (in-memory; .savespawn to persist)"),
        tx,
    )
    .await;
}

/// `.staticmesh` / `.bodyset` — set a string template field by command name.
async fn set_str_field(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    field: &str,
) {
    let value = args[0].to_string();
    if let Some(e) = space_mgr.get_entity_mut(target) {
        match field {
            "staticmesh" => e.static_mesh = Some(value.clone()),
            "bodyset" => e.body_set = Some(value.clone()),
            _ => {}
        }
    }
    send_gm_feedback(
        caller_id,
        &format!("{field} [{target}] = '{value}' (in-memory; shown on next AoI entry)"),
        tx,
    )
    .await;
}

/// `.interactiontype <flags>` — set the raw INT_* interaction-type bitfield.
async fn set_interaction_type(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(value) = super::parse_i32(caller_id, args, 0, "interactiontype", tx).await else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        e.interaction_type_flags = i64::from(value);
    }
    send_gm_feedback(
        caller_id,
        &format!("interactiontype [{target}] = {value} (in-memory)"),
        tx,
    )
    .await;
}

/// `.lookat` — rotate the target to face the caller (heading write only).
async fn look_at(
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(caller_pos) = space_mgr.get_entity(caller_id).map(|e| e.position) else {
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        let dx = caller_pos.x - e.position.x;
        let dz = caller_pos.z - e.position.z;
        let len = (dx * dx + dz * dz).sqrt();
        if len > f32::EPSILON {
            e.direction = cimmeria_common::Vector3::new(dx / len, 0.0, dz / len);
        }
    }
    send_gm_feedback(
        caller_id,
        &format!("lookat [{target}]: facing you (in-memory; shown on next AoI entry)"),
        tx,
    )
    .await;
}

/// `.visible <1|0>` — show/hide the target. Hiding fans `EntityInvisible` to the
/// target's witnesses; showing requests an AoI re-entry (note in feedback).
async fn set_visible(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(visible) = super::parse_bool(caller_id, args, 0, "visible", tx).await else {
        return;
    };
    if visible {
        send_gm_feedback(
            caller_id,
            &format!("visible [{target}] = true (re-appears on next AoI refresh)"),
            tx,
        )
        .await;
        return;
    }
    let witnesses = space_mgr.get_witnesses_of(target);
    for witness_id in &witnesses {
        let _ = tx
            .send(CellToBaseMsg::EntityInvisible {
                witness_id: *witness_id,
                entity_id: target,
            })
            .await;
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "visible [{target}] = false (hidden from {} witness(es))",
            witnesses.len()
        ),
        tx,
    )
    .await;
}

/// `.setcombatant` / `.unsetcombatant <bit>` — set/clear a state-field bit index
/// (0-31). The legacy took a `PLAYER_STATE_*` name; we take the bit index since
/// the cell has no name table for combatant flags.
async fn set_combatant(
    caller_id: u32,
    target: u32,
    args: &[&str],
    set: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(bit) = super::parse_i32(caller_id, args, 0, "flag bit", tx).await else {
        return;
    };
    if !(0..32).contains(&bit) {
        send_gm_feedback(caller_id, "combatant flag must be a bit index 0-31.", tx).await;
        return;
    }
    let mask = 1u32 << bit;
    if let Some(e) = space_mgr.get_entity_mut(target) {
        if set {
            e.state_field |= mask;
        } else {
            e.state_field &= !mask;
        }
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "{} [{target}] bit {bit} (state_field now in-memory)",
            if set {
                "setcombatant"
            } else {
                "unsetcombatant"
            }
        ),
        tx,
    )
    .await;
}

/// `.addcomponent` / `.delcomponent <component>` — add/remove a body component.
/// For players, re-broadcasts `BeingAppearance`; NPC edits surface on next AoI
/// entry.
async fn component(
    caller_id: u32,
    target: u32,
    args: &[&str],
    add: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let component = args[0].to_string();
    let mut player_refresh: Option<(i32, bool)> = None;
    if let Some(e) = space_mgr.get_entity_mut(target) {
        if add {
            if !e.components.contains(&component) {
                e.components.push(component.clone());
            }
        } else {
            e.components.retain(|c| c != &component);
        }
        if e.is_player {
            if let Some(pid) = e.player_id {
                player_refresh = Some((pid, e.weapon_holstered));
            }
        }
    }
    if let Some((player_id, holstered)) = player_refresh {
        let _ = tx
            .send(CellToBaseMsg::RefreshAppearance {
                entity_id: target,
                player_id,
                holstered,
            })
            .await;
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "{} [{target}] '{component}'",
            if add { "addcomponent" } else { "delcomponent" }
        ),
        tx,
    )
    .await;
}

/// `.adddialog` / `.removedialog <templateId> <setMapId>` — attach/detach a
/// dialog choice for a player target, resolving the dialog via
/// `dialog_set_maps[setMapId]`.
async fn dialog(
    caller_id: u32,
    target: u32,
    args: &[&str],
    add: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(template_id) = super::parse_i32(caller_id, args, 0, "templateId", tx).await else {
        return;
    };
    let Some(set_map_id) = super::parse_i32(caller_id, args, 1, "setMapId", tx).await else {
        return;
    };
    let Some((dialog_id, interaction_flags)) = space_mgr
        .dialog_set_maps
        .get(&set_map_id)
        .map(|e| (e.dialog_id, e.interaction_flags))
    else {
        send_gm_feedback(
            caller_id,
            &format!("dialog: unknown dialog_set_map id {set_map_id}"),
            tx,
        )
        .await;
        return;
    };
    if let Some(e) = space_mgr.get_entity_mut(target) {
        let list = e.available_interactions.entry(template_id).or_default();
        let tuple = (set_map_id, dialog_id, interaction_flags);
        if add {
            if !list.contains(&tuple) {
                list.push(tuple);
            }
        } else {
            list.retain(|t| t.0 != set_map_id);
        }
    }
    send_gm_feedback(
        caller_id,
        &format!(
            "{} [{target}] template {template_id} setMap {set_map_id} (dialog {})",
            if add { "adddialog" } else { "removedialog" },
            dialog_id
        ),
        tx,
    )
    .await;
}

/// `.dynamicupdate` — re-push the target's dynamic properties to its witnesses.
/// For a player target this re-broadcasts `BeingAppearance`; otherwise it's a
/// best-effort note (NPC dynamic props refresh on next AoI entry).
async fn dynamic_update(
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let refresh = space_mgr
        .get_entity(target)
        .filter(|e| e.is_player)
        .and_then(|e| e.player_id.map(|pid| (pid, e.weapon_holstered)));
    if let Some((player_id, holstered)) = refresh {
        let _ = tx
            .send(CellToBaseMsg::RefreshAppearance {
                entity_id: target,
                player_id,
                holstered,
            })
            .await;
    }
    send_gm_feedback(
        caller_id,
        &format!("dynamicupdate [{target}]: re-broadcast requested"),
        tx,
    )
    .await;
}
