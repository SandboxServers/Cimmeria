//! `grant_ability` row → [`Action::GrantAbility`] (Class Start v6, CS-01a),
//! and the load-time check that every granted id is a real ability.
//!
//! Every param is in `params`; `target_id` and `target_key` stay empty:
//!
//! ```json
//! {"ability_ids": [592, 594], "source_kind": "tutorial", "source_id": 1559}
//! ```
//!
//! - `ability_ids`: a non-empty list of distinct positive ability ids.
//! - `source_kind`: `tutorial`, `racial_core`, `signature` or `mission`.
//!   `gm` is refused: a GM grant is removed by the GM reset and earns no
//!   branch credit, so a content chain that wrote one would hand out an
//!   ability the next reset takes away.
//! - `source_id`: optional, the mission or chain id the grant comes from.
//! - `archetypes`: the `EArchetype` ordinals (1-8) that may receive the
//!   grant. **Required** for `signature` and `racial_core`, optional for
//!   `tutorial` and `mission` (absent = every archetype). The executor and
//!   the base both check it against the player's real archetype.
//!
//! **A bad row refuses the whole chain**, unlike most verbs, which drop the
//! one row. A grant chain usually does several things for one milestone (a
//! tutorial line, an item, the abilities); running the rest without the
//! grant would leave a player told about an ability they never got.

use std::collections::HashSet;

use crate::actions::{AbilityGrant, AbilityGrantKind, Action};
use crate::chain::Chain;

use super::DbActionRow;

/// The `action_type` this module owns.
pub(super) const ACTION_TYPE: &str = "grant_ability";

/// Convert one `grant_ability` row, or say why the chain must be refused.
pub(super) fn convert_grant_ability(row: &DbActionRow) -> Result<Action, String> {
    let params = &row.params;
    if row.target_id.is_some() || row.target_key.is_some() {
        return Err(
            "grant_ability takes its ids from params.ability_ids; target_id and target_key \
             must be empty"
                .to_string(),
        );
    }

    let Some(raw_ids) = params.get("ability_ids").and_then(|v| v.as_array()) else {
        return Err("`ability_ids` must be a list of ability ids".to_string());
    };
    if raw_ids.is_empty() {
        return Err("`ability_ids` is empty".to_string());
    }
    let mut ability_ids = Vec::with_capacity(raw_ids.len());
    for v in raw_ids {
        let Some(id) = v
            .as_i64()
            .and_then(|i| i32::try_from(i).ok())
            .filter(|&i| i > 0)
        else {
            return Err(format!(
                "`ability_ids` entry {v} is not a positive integer ability id"
            ));
        };
        if ability_ids.contains(&id) {
            return Err(format!("`ability_ids` lists {id} twice"));
        }
        ability_ids.push(id);
    }

    let raw_kind = params
        .get("source_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let source_kind = match AbilityGrantKind::from_str_opt(raw_kind) {
        Some(AbilityGrantKind::Gm) => {
            return Err(
                "`source_kind` \"gm\" is for GM tools only; content grants use tutorial, \
                 racial_core, signature or mission"
                    .to_string(),
            )
        }
        Some(kind) => kind,
        None => {
            return Err(format!(
                "`source_kind` {raw_kind:?} must be one of tutorial, racial_core, signature, \
                 mission"
            ))
        }
    };

    let source_id = match params.get("source_id") {
        None | Some(serde_json::Value::Null) => None,
        Some(v) => match v.as_i64().and_then(|i| i32::try_from(i).ok()) {
            Some(id) => Some(id),
            None => return Err("`source_id` must be an integer mission or chain id".to_string()),
        },
    };

    let archetypes = parse_archetypes(params.get("archetypes"))?;
    if archetypes.is_empty() && requires_archetypes(source_kind) {
        return Err(format!(
            "`archetypes` is required for a {} grant: a class or race grant must name who \
             may receive it, never rely on the trigger's archetype condition alone",
            source_kind.as_str()
        ));
    }

    Ok(Action::GrantAbility(AbilityGrant {
        ability_ids,
        source_kind,
        source_id,
        archetypes,
    }))
}

/// `signature` and `racial_core` grants belong to one class or race, so the
/// row must say which (CS-01a review F2): a chain gated only by its
/// trigger's `archetype` condition fails open where the trigger does not set
/// one (`archetype neq N` reads a missing value as -1 and passes).
pub(super) fn requires_archetypes(kind: AbilityGrantKind) -> bool {
    matches!(
        kind,
        AbilityGrantKind::Signature | AbilityGrantKind::RacialCore
    )
}

/// The optional `archetypes` param: a non-empty list of distinct
/// `EArchetype` ordinals 1-8 (0, "Any", would gate nothing). Absent or
/// null is an empty list, which means "every archetype".
fn parse_archetypes(value: Option<&serde_json::Value>) -> Result<Vec<i32>, String> {
    let raw = match value {
        None | Some(serde_json::Value::Null) => return Ok(Vec::new()),
        Some(v) => v
            .as_array()
            .ok_or_else(|| "`archetypes` must be a list of EArchetype ordinals".to_string())?,
    };
    if raw.is_empty() {
        return Err("`archetypes` is empty; omit it to allow every archetype".to_string());
    }
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        let Some(id) = v
            .as_i64()
            .and_then(|i| i32::try_from(i).ok())
            .filter(|&i| i >= 1 && cimmeria_names::archetype_name(i).is_some())
        else {
            return Err(format!(
                "`archetypes` entry {v} is not an EArchetype ordinal from 1 to 8"
            ));
        };
        if out.contains(&id) {
            return Err(format!("`archetypes` lists {id} twice"));
        }
        out.push(id);
    }
    Ok(out)
}

/// Drop every chain with a `GrantAbility` naming an id that is not in
/// `known_abilities` (`resources.abilities`), logging one ERROR per chain.
/// `None` means the ability table could not be read: then every chain with
/// a `GrantAbility` is refused (reason `ability_table_unavailable`) and the
/// rest of the engine still loads.
///
/// The row converter cannot see the ability table, so the cell's loader
/// runs this once the chains are built. A multi-trigger chain is one logical
/// chain: every expansion of a refused chain id goes, and it is logged once.
pub fn refuse_chains_with_unknown_abilities(
    chains: Vec<Chain>,
    known_abilities: Option<&HashSet<i32>>,
) -> Vec<Chain> {
    let mut refused: HashSet<i64> = HashSet::new();
    for chain in &chains {
        if refused.contains(&chain.id) {
            continue;
        }
        let granted: Vec<i32> = chain
            .actions
            .iter()
            .filter_map(|a| match a {
                Action::GrantAbility(g) => Some(&g.ability_ids),
                _ => None,
            })
            .flatten()
            .copied()
            .collect();
        if granted.is_empty() {
            continue;
        }
        match known_abilities {
            None => {
                tracing::error!(
                    target: "content",
                    event = "chain_refused",
                    reason = "ability_table_unavailable",
                    chain_id = chain.id,
                    chain_name = cimmeria_names::book().chain(chain.id),
                    ability_ids = ?granted,
                    "grant_ability ids could not be checked (resources.abilities unreadable); \
                     the chain is not loaded"
                );
                refused.insert(chain.id);
            }
            Some(known) => {
                let unknown: Vec<i32> = granted
                    .into_iter()
                    .filter(|id| !known.contains(id))
                    .collect();
                if !unknown.is_empty() {
                    tracing::error!(
                        target: "content",
                        event = "chain_refused",
                        reason = "grant_ability_unknown_ability",
                        chain_id = chain.id,
                        chain_name = cimmeria_names::book().chain(chain.id),
                        unknown_ability_ids = ?unknown,
                        "grant_ability names an ability with no resources.abilities row; \
                         the chain is not loaded"
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
