//! The native combat-debug GM commands (AB-N1): `gmDebugAbility` (169,
//! `/gmdebugability <abilityId>`), `gmDebugCombat` (170, `/gmdebugcombat`),
//! `gmDebugCombatVerbose` (171, `/gmdebugcombatverbose`), `gmDebugHeal`
//! (172, `/gmdebugheal`) and `gmDebugAbilityOnMob` (176,
//! `/gmdebugabilityonmob <abilityId>`).
//!
//! The stock client sends these only from an `SGWGmPlayer` avatar
//! (`docs/reverse-engineering/findings/native-combat-debug.md`), and the
//! dispatch gate refuses them for a non-GM anyway. Each flips the caller's
//! debug state in `cimmeria_cell_world::cell::combat_debug` (the
//! `bCombatDebug` family of `SGWAbilityManager.def`), answers with a
//! feedback line on the first press, and writes one `gm_command` row. The
//! debug lines themselves go out from the combat pipeline as casts resolve.
//!
//! - 169 `gmDebugAbility(aAbilityId)`: toggle the ability in
//!   `debugAbilityList`; `0` is `clearAbilityDebug` (empties the list and
//!   the mob list, lines back to the caller).
//! - 170 / 171 / 172: toggle combat, verbose combat, heal debug.
//! - 176 `gmDebugAbilityOnMob(AbilityID)`: the selected mob's casts of that
//!   ability (`0`: all of them) print to the caller.

use tokio::sync::mpsc;

use cimmeria_cell_world::cell::combat_debug::commands::{
    toggle, toggle_ability, toggle_mob, Refused, Toggle,
};

use super::command_log::{log_gm_command, Outcome};
use super::feedback::send_gm_feedback;
use super::{
    read_i32, GM_DEBUG_ABILITY, GM_DEBUG_ABILITY_ON_MOB, GM_DEBUG_COMBAT, GM_DEBUG_COMBAT_VERBOSE,
    GM_DEBUG_HEAL,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Log the row and answer the GM: `cmd: <text>`.
async fn answer(
    entity_id: u32,
    cmd: &'static str,
    index: u16,
    args_text: &str,
    result: Result<String, Refused>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let (outcome, text) = match result {
        Ok(text) => (Outcome::Applied, text),
        Err(r) => (Outcome::Refused(r.reason), r.text),
    };
    log_gm_command(space_mgr, entity_id, cmd, index, args_text, outcome);
    send_gm_feedback(entity_id, &format!("{cmd}: {text}"), tx).await;
}

/// 170, 171, 172: the argument-less toggles.
pub(super) async fn handle_toggle(
    entity_id: u32,
    index: u16,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (cmd, which) = match index {
        GM_DEBUG_COMBAT => ("gmDebugCombat", Toggle::Combat),
        GM_DEBUG_COMBAT_VERBOSE => ("gmDebugCombatVerbose", Toggle::Verbose),
        GM_DEBUG_HEAL => ("gmDebugHeal", Toggle::Heal),
        _ => return false,
    };
    let result = toggle(space_mgr, entity_id, which).map(|(_, text)| text);
    answer(entity_id, cmd, index, "", result, tx, space_mgr).await;
    true
}

/// 169 and 176: the `INT32` ability-id toggles.
pub(super) async fn handle_ability_toggle(
    entity_id: u32,
    index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (cmd, arg) = match index {
        GM_DEBUG_ABILITY => ("gmDebugAbility", "aAbilityId"),
        GM_DEBUG_ABILITY_ON_MOB => ("gmDebugAbilityOnMob", "AbilityID"),
        _ => return false,
    };
    let Some(ability_id) = read_i32(args, 0) else {
        let result = Err(Refused {
            reason: "bad_args",
            text: format!("missing INT32 {arg}"),
        });
        answer(entity_id, cmd, index, "", result, tx, space_mgr).await;
        return true;
    };
    let args_text = format!("{arg}={ability_id}");
    let result = if index == GM_DEBUG_ABILITY {
        toggle_ability(space_mgr, entity_id, ability_id)
    } else {
        toggle_mob(space_mgr, entity_id, ability_id)
    }
    .map(|(_, text)| text);
    answer(entity_id, cmd, index, &args_text, result, tx, space_mgr).await;
    true
}
