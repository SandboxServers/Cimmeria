//! The toggle bodies behind `gmDebugCombat` (170), `gmDebugCombatVerbose`
//! (171), `gmDebugHeal` (172), `gmDebugAbility` (169) and
//! `gmDebugAbilityOnMob` (176), shared with the crafted-caller cells 2
//! (`toggleCombatDebug`), 3 (`toggleCombatVerboseDebug`) and 6
//! (`toggleHealDebug`); and the two server-side helpers the def declares
//! but the client cannot call, `setAbilityDebugTarget` and
//! `clearAbilityDebug`.
//!
//! Each returns the feedback line the caller shows, or a [`Refused`] with
//! the reason its row logs and the line that explains it. They change
//! state only; sending the line is the caller's job.
//!
//! **Deviation from the legacy python, on purpose.** `SGWGmPlayer.py`'s
//! `gmDebugAbility` made the GM's target cast the ability once with the GM
//! attached as its debug player, and its `gmDebugHeal` fully healed the
//! target. The AB-N1 design (`docs/analysis/ability-mechanics/
//! lab-uat-and-telemetry.md`) keys these commands on the def's own debug
//! state instead (`debugAbilityList`, a heal-debug toggle), the intent of
//! the names; the legacy server is a reconstruction, not CME's code.

use super::settings::DebugSettings;
use crate::cell::space_manager::SpaceManager;

/// A command that changed nothing, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The `reason` its row logs.
    pub reason: &'static str,
    /// The feedback line.
    pub text: String,
}

impl Refused {
    fn new(reason: &'static str, text: impl Into<String>) -> Self {
        Self {
            reason,
            text: text.into(),
        }
    }
}

/// The three on/off toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Combat,
    Verbose,
    Heal,
}

/// Edit `entity_id`'s settings, creating them for a player and dropping
/// them once nothing is on. A missing or non-player caller is refused.
fn edit<T>(
    mgr: &mut SpaceManager,
    entity_id: u32,
    f: impl FnOnce(&mut DebugSettings) -> T,
) -> Result<T, Refused> {
    let Some(caller) = mgr.get_entity(entity_id) else {
        return Err(Refused::new("caller_gone", "caller entity not found"));
    };
    if !caller.is_player {
        return Err(Refused::new("caller_not_player", "only a player can debug"));
    }
    let player_id = caller.player_id;
    let watchers = &mut mgr.combat_debug.watchers;
    let settings = watchers.entry(entity_id).or_default();
    if settings.player_id != player_id {
        // The entity id was reused by another character: start clean.
        *settings = DebugSettings::default();
        settings.player_id = player_id;
    }
    let out = f(settings);
    if settings.is_idle() {
        watchers.remove(&entity_id);
    }
    Ok(out)
}

/// Flip one toggle. Returns whether it is now on, and the feedback line.
pub fn toggle(
    mgr: &mut SpaceManager,
    entity_id: u32,
    which: Toggle,
) -> Result<(bool, String), Refused> {
    let on = edit(mgr, entity_id, |s| {
        let flag = match which {
            Toggle::Combat => &mut s.combat,
            Toggle::Verbose => &mut s.verbose,
            Toggle::Heal => &mut s.heal,
        };
        *flag = !*flag;
        *flag
    })?;
    let text = match (which, on) {
        (Toggle::Combat, true) => {
            "Combat debug on: each hit you land or take prints its roll and damage here"
        }
        (Toggle::Combat, false) => "Combat debug off",
        (Toggle::Verbose, true) => {
            "Verbose combat debug on: each hit also prints its effect plans, NVP damage, \
             ledger entries and pulses"
        }
        (Toggle::Verbose, false) => "Verbose combat debug off",
        (Toggle::Heal, true) => {
            "Heal debug on: each heal or buff you cast or receive prints what it changed here"
        }
        (Toggle::Heal, false) => "Heal debug off",
    };
    Ok((on, text.to_string()))
}

/// `name (id)` of an ability, or `ability id` when the cell has no def.
pub fn ability_label(mgr: &SpaceManager, ability_id: i32) -> String {
    match mgr.ability_defs.get(&ability_id) {
        Some(d) => format!("{} ({ability_id})", d.name),
        None => format!("ability {ability_id}"),
    }
}

/// `gmDebugAbility(aAbilityId)` / `toggleAbilityDebugging`: add the ability
/// to `debugAbilityList`, or take it off. `0` is `clearAbilityDebug`.
/// Returns whether it is now listed, and the feedback line.
pub fn toggle_ability(
    mgr: &mut SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> Result<(bool, String), Refused> {
    if ability_id == 0 {
        return clear_ability_debug(mgr, entity_id).map(|t| (false, t));
    }
    if !mgr.ability_defs.contains_key(&ability_id) {
        return Err(Refused::new(
            "unknown_ability",
            format!("no ability {ability_id} in this cell's ability table"),
        ));
    }
    let label = ability_label(mgr, ability_id);
    let on = edit(mgr, entity_id, |s| {
        if let Some(i) = s.abilities.iter().position(|&a| a == ability_id) {
            s.abilities.remove(i);
            false
        } else {
            s.abilities.push(ability_id);
            true
        }
    })?;
    let text = if on {
        format!(
            "Ability debug on for {label}: its casts by you or on you print here, \
             even with combat debug off"
        )
    } else {
        format!("Ability debug off for {label}")
    };
    Ok((on, text))
}

/// `gmDebugAbilityOnMob(AbilityID)`: print the selected mob's casts of
/// `ability_id` (0: all of its casts) to the caller, or stop. The mob is
/// the caller's selection, an NPC in the caller's space.
pub fn toggle_mob(
    mgr: &mut SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> Result<(bool, String), Refused> {
    let Some(caller) = mgr.get_entity(entity_id) else {
        return Err(Refused::new("caller_gone", "caller entity not found"));
    };
    let caller_space = caller.space_id;
    let Some(mob_id) = caller
        .current_target_id
        .and_then(|t| u32::try_from(t).ok())
        .filter(|&t| t != 0 && t != entity_id)
    else {
        return Err(Refused::new("no_target", "select a mob first"));
    };
    let Some(mob) = mgr.get_entity(mob_id) else {
        return Err(Refused::new(
            "target_gone",
            format!("the selected entity {mob_id} is gone"),
        ));
    };
    if mob.is_player {
        return Err(Refused::new(
            "target_is_player",
            "the selection is a player; use /gmdebugcombat or /gmdebugability for players",
        ));
    }
    if mob.space_id != caller_space {
        return Err(Refused::new(
            "target_other_space",
            "the selected mob is in another space",
        ));
    }
    if ability_id != 0 && !mgr.ability_defs.contains_key(&ability_id) {
        return Err(Refused::new(
            "unknown_ability",
            format!("no ability {ability_id} in this cell's ability table"),
        ));
    }
    let mob_label = super::format::entity_label(mgr, mob_id);
    let what = if ability_id == 0 {
        "all its abilities".to_string()
    } else {
        ability_label(mgr, ability_id)
    };
    let on = edit(mgr, entity_id, |s| {
        let key = (mob_id, ability_id);
        if let Some(i) = s.mobs.iter().position(|&m| m == key) {
            s.mobs.remove(i);
            false
        } else {
            s.mobs.push(key);
            true
        }
    })?;
    let text = if on {
        format!("Mob debug on for {mob_label}, {what}: its casts print here")
    } else {
        format!("Mob debug off for {mob_label}, {what}")
    };
    Ok((on, text))
}

/// `setAbilityDebugTarget(INT32)`: send the caller's debug lines to player
/// `target_id` instead (the caller itself for 0 or its own id). The target
/// must be a player in the caller's space. No client event reaches this
/// method; it is a server-side helper.
pub fn set_ability_debug_target(
    mgr: &mut SpaceManager,
    entity_id: u32,
    target_id: u32,
) -> Result<String, Refused> {
    let target = (target_id != 0 && target_id != entity_id).then_some(target_id);
    if let Some(t) = target {
        let caller_space = mgr.get_entity(entity_id).map(|e| e.space_id);
        match mgr.get_entity(t) {
            Some(e) if e.is_player && Some(e.space_id) == caller_space => {}
            Some(e) if !e.is_player => {
                return Err(Refused::new(
                    "target_not_player",
                    "debug lines can only go to a player",
                ))
            }
            Some(_) => {
                return Err(Refused::new(
                    "target_other_space",
                    "that player is in another space",
                ))
            }
            None => return Err(Refused::new("target_gone", format!("no entity {t}"))),
        }
    }
    let label = target.map(|t| super::format::entity_label(mgr, t));
    edit(mgr, entity_id, |s| s.target = target)?;
    Ok(match label {
        Some(l) => format!("Debug lines now go to {l}"),
        None => "Debug lines now go to you".to_string(),
    })
}

/// `clearAbilityDebug()`: empty `debugAbilityList` and the mob list, and
/// send the lines to the caller again. The on/off toggles stay as they are.
pub fn clear_ability_debug(mgr: &mut SpaceManager, entity_id: u32) -> Result<String, Refused> {
    edit(mgr, entity_id, |s| {
        s.abilities.clear();
        s.mobs.clear();
        s.target = None;
    })?;
    Ok("Ability debug cleared: no listed abilities or mobs, lines go to you".to_string())
}

/// Cells 2 (`toggleCombatDebug`), 3 (`toggleCombatVerboseDebug`) and 6
/// (`toggleHealDebug`): the same toggle as 170, 171 and 172, for a crafted
/// caller (the stock client has no event bound to them). The dispatch gate
/// has already refused a non-GM. Answers with a feedback line and writes
/// one `gm_command` row, as the GM handlers do.
pub async fn toggle_from_cell_method(
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    mgr: &mut SpaceManager,
    entity_id: u32,
    method_index: u16,
    cmd: &'static str,
    which: Toggle,
) {
    let result = toggle(mgr, entity_id, which);
    let who = mgr.player_identity(entity_id);
    let (outcome, reason, text) = match result {
        Ok((_, text)) => ("applied", None, text),
        Err(r) => ("refused", Some(r.reason), r.text),
    };
    tracing::info!(
        target: "abilities",
        event = "gm_command",
        cmd,
        method_index,
        entity_id,
        account_id = who.account_id,
        player_id = who.player_id,
        decision_outcome = outcome,
        reason,
        "combat debug toggle from a cell method"
    );
    super::deliver::send_feedback_line(tx, mgr, entity_id, &format!("{cmd}: {text}")).await;
}
