//! Read-only console queries — `.help`, the `.search*` family, `.players`
//! (category D), and the entity/combat inspection trio `.info`/`.facing`/
//! `.combatinfo` (category I). Search runs base-side (the cell caches
//! resource ids but not display names); the rest read cell state and reply
//! via the feedback channel.
//!
//! Legacy reference: `deprecated/python/cell/commands/Resource.py`
//! (`searchItem`/`searchMission`/`searchTemplate`), `deprecated/python/cell/commands/Misc.py`
//! (`players`), and `deprecated/python/cell/commands/Entity.py`
//! (`entityInfo`/`facing`/`combatInfo`).

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;
use tokio::sync::mpsc;

use super::registry::arg_specs;
use super::{send_gm_feedback, Spec, COMMANDS};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.help [filter]` — list the console commands (optionally filtered by
/// substring) **sorted by command name**, each with its one-line summary.
/// Mirrors the legacy `help`, which also sorted by name.
///
/// When the filtered result set has `<= 3` commands, also prints each
/// documented argument (see [`arg_specs`]) as `    name (type): desc` when
/// required or `    [name] (type): desc` when optional — mirroring legacy
/// `ConsoleCommands.py::help`'s detail view. Legacy determined
/// required-vs-optional via `index <= cmd.argsMin`, an off-by-one that
/// mis-marks a command's *last* optional argument as required (e.g. its own
/// `command` arg, or `searchitem`'s `name2`); this restores the intended
/// `index < min` semantics instead of reproducing the bug (D02).
pub(super) async fn help(caller_id: u32, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let matches = help_specs(args.first().copied());
    for spec in &matches {
        send_gm_feedback(caller_id, &format!(".{}: {}", spec.name, spec.help), tx).await;
        if matches.len() <= 3 {
            for (i, a) in arg_specs(spec.name).iter().enumerate() {
                let line = if i < spec.min {
                    format!("    {} ({}): {}", a.name, a.ty, a.desc)
                } else {
                    format!("    [{}] ({}): {}", a.name, a.ty, a.desc)
                };
                send_gm_feedback(caller_id, &line, tx).await;
            }
        }
    }
    if matches.is_empty() {
        // Verbatim legacy wording (`ConsoleCommands.py::help`'s
        // `player.feedback('No command found with that name')`), not the
        // "{name}: ..." prefix convention other feedback lines use — kept
        // exact so it's a stable, testable string.
        send_gm_feedback(caller_id, "No command found with that name", tx).await;
    }
}

/// The `.help` result set: commands matching the optional substring `filter`,
/// sorted alphabetically by name. Split out so the sort is unit-testable.
fn help_specs(filter: Option<&str>) -> Vec<&'static Spec> {
    let needle = filter.map(str::to_ascii_lowercase);
    let mut matches: Vec<&'static Spec> = COMMANDS
        .iter()
        .filter(|spec| needle.as_deref().is_none_or(|f| spec.name.contains(f)))
        .collect();
    matches.sort_by_key(|spec| spec.name);
    matches
}

/// Build the search query string from one or two name args (legacy joined a
/// second optional word with a space) and dispatch the base-side search.
async fn search(caller_id: u32, kind: u8, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>) {
    let query = args.join(" ");
    let _ = tx
        .send(CellToBaseMsg::ConsoleSearch {
            entity_id: caller_id,
            kind,
            query,
        })
        .await;
}

/// `.searchitem <name> [name2]` — search item designs by name.
pub(super) async fn search_item(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 0, args, tx).await;
}

/// `.searchmission <name> [name2]` — search mission designs by name.
pub(super) async fn search_mission(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 1, args, tx).await;
}

/// `.searchtemplate <name> [name2]` — search entity templates by name.
pub(super) async fn search_template(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    search(caller_id, 2, args, tx).await;
}

/// `.players` — list the players in the caller's space. Cell-scoped (the cell
/// only knows its own spaces); mirrors the legacy `players` / `gmUsers`.
pub(super) async fn players(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(space_id) = space_mgr.get_entity_space_id(caller_id) else {
        send_gm_feedback(caller_id, "players: you are not in a space.", tx).await;
        return;
    };
    let mut ids: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .collect();
    ids.sort_unstable();
    if ids.is_empty() {
        send_gm_feedback(caller_id, "players: none in your space.", tx).await;
        return;
    }
    let list = ids
        .iter()
        .map(|pid| {
            let char_id = space_mgr.get_entity(*pid).and_then(|e| e.player_id);
            match char_id {
                Some(c) => format!("{pid} (char {c})"),
                None => pid.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    send_gm_feedback(
        caller_id,
        &format!("players ({} in space): {list}", ids.len()),
        tx,
    )
    .await;
}

// ---- .info / .facing / .combatinfo (category I) ---------------------------

/// Legacy `ARCHETYPE_*` names (`deprecated/python/Atrea/enums.py:218-226`).
/// A small, closed, stable table — safe to port directly (unlike faction,
/// where this codebase's `faction: u8` already uses a different, simplified
/// numbering than legacy's 34-entry table; see [`faction_name`]).
fn archetype_name(id: i32) -> &'static str {
    match id {
        0 => "Any",
        1 => "Soldier",
        2 => "Commando",
        3 => "Scientist",
        4 => "Archeologist",
        5 => "Asgard",
        6 => "Goauld",
        7 => "Sholva",
        8 => "Jaffa",
        _ => "unknown",
    }
}

/// Alignment names, matching the lowercase tokens `.alignment` already
/// accepts (`console/entity.rs::set_alignment`) — legacy `ALIGNMENT_*` order
/// (Undefined=0, Praxis=1, SGU=2; legacy defines further Side_01..End values
/// this codebase doesn't use).
fn alignment_name(value: u8) -> &'static str {
    match value {
        0 => "undefined",
        1 => "praxis",
        2 => "sgu",
        _ => "unknown",
    }
}

/// Faction names for this codebase's simplified `faction: u8` scheme
/// (`entity_struct.rs`'s doc comment: "0=neutral, 1=Tau'ri, 3=SGC,
/// 10=hostile") — NOT legacy's full 34-entry `FACTION_*` table, which uses
/// incompatible numbering (legacy `FACTION_SGC = 2`, this codebase's
/// `faction = 3` means SGC). Naming raw ids from the wrong table would
/// misrepresent data, so this mirrors only the scheme this engine actually
/// uses.
fn faction_name(value: u8) -> &'static str {
    match value {
        0 => "neutral",
        1 => "Tau'ri",
        3 => "SGC",
        10 => "hostile",
        _ => "unknown",
    }
}

/// Short human label for `.info`'s header line: player display name → NPC
/// name → `template N` → generic `player`/`entity` fallback.
fn entity_label(e: &CellEntity) -> String {
    if let Some(n) = &e.character_name {
        return n.clone();
    }
    if let Some(n) = &e.npc_name {
        return n.clone();
    }
    if let Some(t) = e.template_id {
        return format!("template {t}");
    }
    if e.is_player {
        "player".into()
    } else {
        "entity".into()
    }
}

/// `.info [entityId]` — detailed entity dump. Mirrors legacy `entityInfo`
/// (`deprecated/python/cell/commands/Entity.py:50-98`): **selection wins**
/// over the explicit `entityId` arg, which is only a fallback when there is
/// no current selection (opposite precedence from the native `gmShowPlayer`
/// in `cell_methods::gm::query`, which treats a nonzero explicit id as
/// overriding the selection — do not conflate the two).
///
/// Fields legacy always/conditionally prints that this codebase's
/// `CellEntity` cannot supply are omitted rather than faked:
/// - **Colors** (`primaryColorId`/`secondaryColorId`/`skinTint`): `CellEntity`
///   carries no tint fields at all (those live base-side, in character
///   appearance building).
/// - **Template name**: the cell caches template *ids* only, never display
///   names (see this module's top doc and `.searchtemplate`'s base
///   round-trip) — the `Template:` line below reports the id alone.
/// - **Flags/Interaction**: `entity_flags`/`interaction_type_flags` exist,
///   but there is no Rust `EEntityFlags`/`EInteractionNotificationType` name
///   table to decode them (unlike the small closed `EArchetype`/`ALIGNMENT_*`
///   sets above) — reported as raw hex rather than a decoded flag list.
pub(super) async fn info(
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let caller_space = space_mgr.get_entity(caller_id).map(|e| e.space_id.0);
    let subject = target_id.or_else(|| {
        args.first()
            .and_then(|s| s.parse::<u32>().ok())
            .filter(|&id| space_mgr.get_entity(id).map(|e| e.space_id.0) == caller_space)
    });
    let Some(e) = subject.and_then(|id| space_mgr.get_entity(id)) else {
        // Verbatim legacy wording (`entityInfo`'s `'Could not find entity'`).
        send_gm_feedback(caller_id, "Could not find entity", tx).await;
        return;
    };
    let subject = subject.expect("subject resolved above");

    let mut lines = vec![format!(
        " ----- ENTITY {} ({subject}) ----- ",
        entity_label(e)
    )];
    if e.entity_flags != 0 {
        lines.push(format!("Flags: {:#x}", e.entity_flags));
    }
    if e.interaction_type_flags != 0 {
        lines.push(format!(
            "Interaction: {:#x}",
            e.interaction_type_flags as u64
        ));
    }
    if let Some(id) = e.event_set_id.filter(|&id| id != 0) {
        lines.push(format!("Kismet event set: {id}"));
    }
    if let Some(id) = e.name_id.filter(|&id| id != 0) {
        lines.push(format!("Name ID: {id}"));
    }
    if let Some(tid) = e.template_id {
        lines.push(format!("Template: {tid}"));
    }
    if let Some(tag) = &e.tag {
        lines.push(format!("Tag: {tag}"));
    }
    if let Some(mesh) = &e.static_mesh {
        lines.push(format!("Static mesh: {mesh}"));
    }
    if let Some(body_set) = &e.body_set {
        lines.push(format!("Body set: {body_set}"));
    }
    if let Some(aid) = e.archetype_id {
        lines.push(format!("Archetype: {} ({aid})", archetype_name(aid)));
    }
    lines.push(format!(
        "Alignment: {} ({})",
        alignment_name(e.alignment),
        e.alignment
    ));
    lines.push(format!(
        "Faction: {} ({})",
        faction_name(e.faction),
        e.faction
    ));
    lines.push(format!("Level: {}", e.level));

    for line in lines {
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

/// CAS_* geometry constants (`deprecated/python/common/Constants.py:92-100`),
/// used by [`facing_angle`]/[`facing_class`]. Legacy's `CAS_FRONT_FACING`/
/// `CAS_REAR_FACING = 0.78539816` is π/4 written out to float precision —
/// expressed here via the stdlib constant rather than retyping the literal.
const CAS_ENTITY_HEIGHT: f32 = 2.5;
const CAS_FRONT_FACING: f32 = std::f32::consts::FRAC_PI_4;
const CAS_REAR_FACING: f32 = std::f32::consts::FRAC_PI_4;
const CAS_ABOVE_TAN: f32 = 1.732;
const CAS_CLOSE_DISTANCE: f32 = 5.0;

/// Bearing angle (radians, `[0, 2π)`) of `target` relative to `caller`'s own
/// facing direction. Mirrors legacy `SGWSpawnableEntity.facing`
/// (`deprecated/python/cell/SGWSpawnableEntity.py:304-321`): `0` means the
/// caller is looking directly at the target; `π` means the target is
/// directly behind the caller.
fn facing_angle(caller_pos: Vector3, caller_dir: Vector3, target_pos: Vector3) -> f32 {
    let dx = target_pos.x - caller_pos.x;
    let dz = target_pos.z - caller_pos.z;
    if dx == 0.0 && dz == 0.0 {
        return 0.0;
    }
    let bearing = dx.atan2(dz);
    let caller_yaw = caller_dir.x.atan2(caller_dir.z);
    let mut dangle = caller_yaw - bearing;
    if dangle < 0.0 {
        dangle += 2.0 * std::f32::consts::PI;
    }
    dangle
}

/// Facing-class bucket (`Above`/`Below`/`Front`/`Flank`/`Rear`). Mirrors
/// legacy `SGWSpawnableEntity.facingType`
/// (`deprecated/python/cell/SGWSpawnableEntity.py:324-352`).
fn facing_class(caller_pos: Vector3, caller_dir: Vector3, target_pos: Vector3) -> &'static str {
    let pdist =
        ((target_pos.x - caller_pos.x).powi(2) + (target_pos.z - caller_pos.z).powi(2)).sqrt();
    let ydist = target_pos.y - caller_pos.y;
    if pdist < CAS_CLOSE_DISTANCE {
        if ydist > CAS_ENTITY_HEIGHT {
            return "Above";
        }
        if ydist < -CAS_ENTITY_HEIGHT {
            return "Below";
        }
    } else if (ydist / pdist).abs() > CAS_ABOVE_TAN {
        return if ydist > 0.0 { "Above" } else { "Below" };
    }
    let facing = facing_angle(caller_pos, caller_dir, target_pos);
    let two_pi = 2.0 * std::f32::consts::PI;
    if facing < CAS_FRONT_FACING || facing > two_pi - CAS_FRONT_FACING {
        "Front"
    } else if std::f32::consts::PI - CAS_REAR_FACING < facing
        && facing < std::f32::consts::PI + CAS_REAR_FACING
    {
        "Rear"
    } else {
        "Flank"
    }
}

/// `.facing` — facing angle/class and distance from the caller to the
/// selected target. Mirrors legacy `facing`
/// (`deprecated/python/cell/commands/Entity.py:137-153`). Target is
/// required (`Target::Spawnable`) — no fallback to self, unlike the
/// no-arg-inspection commands in `cell_methods::gm::query`.
///
/// `target` and both entities are guaranteed valid by the time this runs:
/// `dispatch::resolve_target` already required a resolved, same-space,
/// `Target::Spawnable`-matching entity before dispatching here (and
/// `Target::Spawnable::matches` accepts every entity, so there is no
/// wrong-type case either), and the caller is the same live entity that
/// authored this command. No dead "not found" branches to guard an
/// unreachable state.
pub(super) async fn facing(
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let caller = space_mgr
        .get_entity(caller_id)
        .expect("caller entity exists for the duration of its own command");
    let (caller_pos, caller_dir) = (caller.position, caller.direction);
    let target_pos = space_mgr
        .get_entity(target)
        .expect("dispatch::resolve_target guarantees a resolved Target::Spawnable")
        .position;

    let angle = facing_angle(caller_pos, caller_dir, target_pos);
    let class = facing_class(caller_pos, caller_dir, target_pos);
    let dist = ((target_pos.x - caller_pos.x).powi(2)
        + (target_pos.y - caller_pos.y).powi(2)
        + (target_pos.z - caller_pos.z).powi(2))
    .sqrt();

    send_gm_feedback(
        caller_id,
        &format!(
            "Facing: {angle:.6} rad / {:.6} deg ({class})",
            angle.to_degrees()
        ),
        tx,
    )
    .await;
    send_gm_feedback(caller_id, &format!("Distance: {dist:.6}"), tx).await;
}

/// `.combatinfo` — combat-readiness diagnostic checklist for the targeted
/// mob. Mirrors legacy `combatInfo`
/// (`deprecated/python/cell/commands/Entity.py:682-709`): emits feedback
/// **only for problems found**; silent (no feedback at all) when everything
/// checks out.
///
/// **Scoped down from legacy** — two of legacy's checks have no equivalent
/// concept in this codebase's entity/ability model yet, so they're omitted
/// rather than guessed:
/// - `target.template.weapon is None` — no per-template "weapon" concept
///   exists for NPCs (`weapon_visual`/`bandolier_items` are player-only).
/// - The `ABILITY_TYPE_{Undefined,Buff,Debuff,Heal,DOT,DD}` bucket counts —
///   `AbilityDef` (`crates/entity/src/abilities/defs.rs`) has no type/category
///   field at all; `AbilityManager` only tracks known ability *ids*. Building
///   an ability-type taxonomy is out of scope for a read-only query packet —
///   see `docs/analysis/legacy-command-parity/handoffs/p02.md`.
///
/// `target` is guaranteed valid and `Target::Mob`-matching by the time this
/// runs (`dispatch::resolve_target`) — no dead "not found"/wrong-type
/// branches to guard an unreachable state.
pub(super) async fn combat_info(
    caller_id: u32,
    target: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let e = space_mgr
        .get_entity(target)
        .expect("dispatch::resolve_target guarantees a resolved Target::Mob");

    let mut lines = Vec::new();
    if e.template_id.is_none() {
        lines.push(" - Entity is not spawned from a template".to_string());
    }
    if e.abilities.known_ability_ids().is_empty() {
        lines.push(" - Entity has no ability set".to_string());
    }

    for line in lines {
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_is_sorted_by_name_and_lists_all() {
        let all = help_specs(None);
        let names: Vec<&str> = all.iter().map(|s| s.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, ".help must be sorted alphabetically by name");
        assert_eq!(
            all.len(),
            COMMANDS.len(),
            "unfiltered .help lists every command"
        );
    }

    #[test]
    fn help_filter_matches_substring_only() {
        let paths = help_specs(Some("path"));
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|s| s.name.contains("path")));
        assert!(help_specs(Some("definitely-no-such-command")).is_empty());
    }
}
