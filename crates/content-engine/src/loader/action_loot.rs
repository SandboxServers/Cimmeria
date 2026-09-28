//! `open_loot` row → [`Action::OpenLoot`] (Decision (@Cadacious, 2026-09-28)).
//!
//! ```text
//! target_id  = loot_table_id   (NULL: reopen this player's pending roll only)
//! target_key = unused
//! params     = {"once_per_character": true, "container_key": "Castle_PreRomneyChest"}
//! ```
//!
//! Both params are optional. `once_per_character` defaults to `false`;
//! `container_key` defaults to the container's spawn tag at run time.
//!
//! A malformed param drops the row with a `warn!` naming the chain: a chest
//! that silently re-rolls every press, or never rolls, is worse than one the
//! loader refuses where an author will see it.

use tracing::warn;

use crate::actions::Action;

use super::DbActionRow;

/// `sgw_player.looted_containers` holds `text`; keep keys short and
/// printable so a key is readable in a DB dump and a log line.
const MAX_KEY_CHARS: usize = 64;

/// Convert one `open_loot` `content_actions` row.
pub(super) fn convert_open_loot(row: &DbActionRow) -> Option<Action> {
    let params = &row.params;
    let drop_row = |why: &str| {
        warn!(
            chain_id = row.chain_id,
            ?params,
            reason = "malformed_open_loot",
            "open_loot: {why}; dropping the action row"
        );
        None
    };

    if row.target_id.is_some_and(|t| t <= 0) {
        return drop_row("`target_id` (loot_table_id) must be positive or NULL");
    }
    let once_per_character = match params.get("once_per_character") {
        None => false,
        Some(v) => match v.as_bool() {
            Some(b) => b,
            None => return drop_row("`once_per_character` must be a boolean"),
        },
    };
    let container_key = match params.get("container_key") {
        None => None,
        Some(v) => match v.as_str().map(str::trim) {
            Some(k)
                if !k.is_empty()
                    && k.chars().count() <= MAX_KEY_CHARS
                    && k.chars().all(|c| c.is_ascii_graphic()) =>
            {
                Some(k.to_string())
            }
            _ => {
                return drop_row(
                    "`container_key` must be 1-64 printable ASCII characters with no spaces",
                )
            }
        },
    };
    Some(Action::OpenLoot {
        loot_table_id: row.target_id,
        once_per_character,
        container_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(target_id: Option<i32>, params: serde_json::Value) -> DbActionRow {
        DbActionRow {
            chain_id: 1274,
            action_type: "open_loot".to_string(),
            target_id,
            target_key: None,
            params,
            delay_ms: 0,
            sort_order: 0,
        }
    }

    #[test]
    fn converts_a_once_per_character_open() {
        let a = convert_open_loot(&row(
            Some(8),
            serde_json::json!({"once_per_character": true, "container_key": "Castle_PreRomneyChest"}),
        ));
        assert_eq!(
            a,
            Some(Action::OpenLoot {
                loot_table_id: Some(8),
                once_per_character: true,
                container_key: Some("Castle_PreRomneyChest".to_string()),
            })
        );
    }

    #[test]
    fn defaults_to_a_repeatable_open_keyed_by_tag() {
        assert_eq!(
            convert_open_loot(&row(Some(3), serde_json::json!({}))),
            Some(Action::OpenLoot {
                loot_table_id: Some(3),
                once_per_character: false,
                container_key: None,
            })
        );
        assert_eq!(
            convert_open_loot(&row(None, serde_json::json!({}))),
            Some(Action::OpenLoot {
                loot_table_id: None,
                once_per_character: false,
                container_key: None,
            })
        );
    }

    #[test]
    fn malformed_rows_are_dropped() {
        for (t, p) in [
            (Some(0), serde_json::json!({})),
            (Some(8), serde_json::json!({"once_per_character": "yes"})),
            (Some(8), serde_json::json!({"container_key": ""})),
            (Some(8), serde_json::json!({"container_key": "has space"})),
        ] {
            assert_eq!(convert_open_loot(&row(t, p.clone())), None, "{t:?} {p}");
        }
    }
}
