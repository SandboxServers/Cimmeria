//! `show_tutorial` row → [`Action::ShowTutorial`] (Class Start v6, CS-03),
//! and the load-time check that every tutorial a chain shows or tests is a
//! real `DUIST_DefaultTutorial` dialog.
//!
//! Every param is in `params`; `target_id` and `target_key` stay empty:
//!
//! ```json
//! {"tutorial_id": 5882}
//! ```
//!
//! **A bad row refuses the whole chain**, like `grant_ability`. The tutorial
//! usually rides a milestone chain (a pistol grant, the core abilities, then
//! the tutorial); running the rest with a silently dropped tutorial leaves
//! the player with an ability bar nobody explained.

use std::collections::HashSet;

use crate::actions::Action;
use crate::chain::Chain;
use crate::conditions::Condition;

use super::DbActionRow;

/// The `action_type` this module owns.
pub(super) const ACTION_TYPE: &str = "show_tutorial";

/// Convert one `show_tutorial` row, or say why the chain must be refused.
pub(super) fn convert_show_tutorial(row: &DbActionRow) -> Result<Action, String> {
    if row.target_id.is_some() || row.target_key.is_some() {
        return Err(
            "show_tutorial takes its id from params.tutorial_id; target_id and target_key \
             must be empty"
                .to_string(),
        );
    }
    let Some(raw) = row.params.get("tutorial_id") else {
        return Err("`tutorial_id` is missing".to_string());
    };
    let Some(tutorial_id) = raw
        .as_i64()
        .and_then(|i| i32::try_from(i).ok())
        .filter(|&i| i > 0)
    else {
        return Err(format!(
            "`tutorial_id` {raw} is not a positive integer dialog id"
        ));
    };
    Ok(Action::ShowTutorial { tutorial_id })
}

/// The tutorial ids a chain names: every `show_tutorial` action and every
/// `tutorial_shown` condition.
fn tutorial_ids(chain: &Chain) -> Vec<i32> {
    let shown = chain.actions.iter().filter_map(|a| match a {
        Action::ShowTutorial { tutorial_id } => Some(*tutorial_id),
        _ => None,
    });
    let tested = chain.conditions.iter().filter_map(|c| match c {
        Condition::TutorialShown { tutorial_id, .. } => Some(*tutorial_id),
        _ => None,
    });
    let mut ids: Vec<i32> = shown.chain(tested).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Drop every chain whose `show_tutorial` action or `tutorial_shown`
/// condition names an id that is not in `tutorial_dialogs` (the
/// `resources.dialogs` rows whose `ui_screen_type` is
/// `DUIST_DefaultTutorial`), logging one ERROR per chain.
///
/// A non-tutorial dialog is refused as firmly as an unknown one: a
/// `DUIST_DefaultDialog` shown through this action would be a one-time
/// mission dialog with buttons, and a typo in a `tutorial_shown` condition
/// would gate its chain on a tutorial nobody can ever see (`eq`) or on
/// nothing at all (`neq`).
///
/// `None` means the dialog table could not be read: every chain that names a
/// tutorial is refused (reason `dialog_table_unavailable`), and the rest of
/// the engine still loads. A multi-trigger chain is one logical chain: every
/// expansion of a refused chain id goes, and it is logged once.
pub fn refuse_chains_with_unknown_tutorials(
    chains: Vec<Chain>,
    tutorial_dialogs: Option<&HashSet<i32>>,
) -> Vec<Chain> {
    let mut refused: HashSet<i64> = HashSet::new();
    for chain in &chains {
        if refused.contains(&chain.id) {
            continue;
        }
        let ids = tutorial_ids(chain);
        if ids.is_empty() {
            continue;
        }
        match tutorial_dialogs {
            None => {
                tracing::error!(
                    target: "content",
                    event = "chain_refused",
                    reason = "dialog_table_unavailable",
                    chain_id = chain.id,
                    chain_name = cimmeria_names::book().chain(chain.id),
                    tutorial_ids = ?ids,
                    "show_tutorial / tutorial_shown ids could not be checked \
                     (resources.dialogs unreadable); the chain is not loaded"
                );
                refused.insert(chain.id);
            }
            Some(known) => {
                let unknown: Vec<i32> = ids.into_iter().filter(|id| !known.contains(id)).collect();
                if !unknown.is_empty() {
                    let names = cimmeria_names::book();
                    let unknown_names: Vec<String> = unknown
                        .iter()
                        .map(|&id| match names.dialog(id) {
                            Some(name) => format!("{id}:{name}"),
                            None => id.to_string(),
                        })
                        .collect();
                    tracing::error!(
                        target: "content",
                        event = "chain_refused",
                        reason = "unknown_tutorial",
                        chain_id = chain.id,
                        chain_name = names.chain(chain.id),
                        unknown_tutorial_ids = ?unknown,
                        unknown_tutorial_names = ?unknown_names,
                        "show_tutorial / tutorial_shown names a dialog that is not a \
                         DUIST_DefaultTutorial row in resources.dialogs; the chain is not loaded"
                    );
                    refused.insert(chain.id);
                }
            }
        }
    }
    if refused.is_empty() {
        return chains;
    }
    chains
        .into_iter()
        .filter(|c| !refused.contains(&c.id))
        .collect()
}
