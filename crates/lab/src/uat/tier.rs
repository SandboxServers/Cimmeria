//! Native levels ("tiers") and action roles.
//!
//! A tier says *how* an action drove the client, from most to least
//! native. The runner records the tier every action actually ran at, and
//! the grader refuses a PASS when a step action ran below the row's
//! `required_native` (see [`super::grade`]).
//!
//! | Tier | Drive |
//! |---|---|
//! | `N1` | Real input on the client's own UI (keys, clicks, typing) |
//! | `N2` | A slash command typed into chat that stands in for a UI action |
//! | `N3` | The stock UI's Lua binding called through `client_lua_eval` |
//! | `G`  | GM setup typed into chat (setup and teardown only) |
//! | `X`  | A server-side shortcut (`server_console_exec`, DB) |

use serde::{Deserialize, Serialize};

/// How an action drove the client. Ordered most native first, so
/// `a < b` means `a` is more native than `b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Tier {
    N1,
    N2,
    N3,
    G,
    X,
}

impl Tier {
    /// True when `self` is at least as native as `required`.
    pub fn satisfies(self, required: Tier) -> bool {
        self <= required
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::N1 => "N1",
            Tier::N2 => "N2",
            Tier::N3 => "N3",
            Tier::G => "G",
            Tier::X => "X",
        }
    }
}

/// Where an action sits in a row. Only `Step` actions count toward the
/// row's native level; setup may use GM and server shortcuts (recorded
/// and flagged, never graded), and `Anchor` is the runner's own `.bug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Setup,
    Step,
    Teardown,
    Anchor,
}

/// The most native tier a known tool can honestly claim, and its default.
///
/// `None` means "a tool the runner does not know" (a sibling agent's new
/// tool, say): the spec must then declare the tier itself, and the runner
/// records `tier_source = "spec"`. For a known tool a spec may declare a
/// *less* native tier (a typed slash command standing in for a click is
/// N2) but never a more native one: `client_lua_eval` can never be N1.
pub fn tool_floor(tool: &str) -> Option<Tier> {
    Some(match tool {
        // Real input: window messages / DirectInput / typing.
        "client_input_key"
        | "client_input_mouse"
        | "client_ui_click"
        | "client_cursor_move"
        | "client_type_text"
        | "client_input_focus"
        | "client_input_release" => Tier::N1,
        // The flows are orchestration over the same real input.
        "lab_login"
        | "lab_create_character"
        | "lab_delete_character"
        | "lab_ensure_character_slot"
        | "lab_play_character"
        | "lab_finish_dialog"
        | "lab_logout"
        | "lab_client_start"
        | "lab_client_stop"
        | "lab_client_restart" => Tier::N1,
        // Lua and native calls drive the client without an input event.
        "client_lua_eval"
        | "client_console"
        | "client_call_native"
        | "client_mem_write"
        | "client_hook_install"
        | "client_hook_remove" => Tier::N3,
        // Server-side shortcuts.
        "server_console_exec" | "server_content_reload" => Tier::X,
        // Everything else, including planned tools: the capability table.
        _ => return super::tools::lookup(tool).and_then(|c| c.floor),
    })
}

/// True for tools that only read (never drive the client). A read used as
/// an action is recorded but does not affect the native grade.
pub fn is_read_only(tool: &str) -> bool {
    matches!(
        tool,
        "client_ui_state"
            | "client_wait_for"
            | "client_entity_table"
            | "client_module_info"
            | "client_mem_read"
            | "client_events_read"
            | "client_hook_list"
            | "client_input_status"
            | "lab_client_status"
            | "lab_characters"
            | "lab_screenshot"
            | "lab_screenshot_region"
            | "lab_pixel_probe"
            | "lab_crash_report"
            | "lab_timeline"
    ) || tool.starts_with("server_")
        && !matches!(tool, "server_console_exec" | "server_content_reload")
        || super::tools::lookup(tool).is_some_and(|c| c.floor.is_none())
}

/// Map a tool's self-reported native level to a tier. The world tools
/// (#1099) report a word (`real_input`, `slash_command`, `ui_lua`,
/// `server_shortcut`); the combat tools (#1100) report the tier code
/// itself (`N1` .. `X`). `read`, `none` and unknown words drove nothing
/// the grade cares about and map to `None`.
pub fn from_reported(level: &str) -> Option<Tier> {
    match level {
        "real_input" | "N1" => Some(Tier::N1),
        "slash_command" | "N2" => Some(Tier::N2),
        "ui_lua" | "N3" => Some(Tier::N3),
        "G" => Some(Tier::G),
        "server_shortcut" | "X" => Some(Tier::X),
        _ => None,
    }
}

/// A `native_level` value as a tool returns it: a word (#1099) or an
/// object with a `tier` code (#1100). Returns the tier and the word read.
pub fn from_reported_value(v: &serde_json::Value) -> Option<(Tier, String)> {
    let word = v
        .as_str()
        .or_else(|| v.get("tier").and_then(serde_json::Value::as_str))?;
    from_reported(word).map(|t| (t, word.to_string()))
}

/// The tier an action ran at, and where that number came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Resolved {
    pub tier: Tier,
    /// `"tool"` (the tool's own floor), `"spec"` (declared), `"role"`
    /// (a chat line's default for its role), or `"read"`.
    pub source: &'static str,
}

/// Resolve the tier for a tool action. `declared` is the spec's `tier`.
/// Errors when the spec claims a known tool is more native than it is,
/// or when an unknown driving tool carries no declared tier.
pub fn resolve_tool(tool: &str, declared: Option<Tier>) -> Result<Resolved, String> {
    match (tool_floor(tool), declared) {
        (Some(floor), Some(d)) if d < floor => Err(format!(
            "tier {} is more native than {tool} can be (at best {})",
            d.as_str(),
            floor.as_str()
        )),
        (Some(_), Some(d)) => Ok(Resolved {
            tier: d,
            source: "spec",
        }),
        (Some(floor), None) => Ok(Resolved {
            tier: floor,
            source: "tool",
        }),
        (None, Some(d)) => Ok(Resolved {
            tier: d,
            source: "spec",
        }),
        (None, None) if is_read_only(tool) => Ok(Resolved {
            tier: Tier::N1,
            source: "read",
        }),
        (None, None) => Err(format!(
            "{tool} is not a tool the runner knows: declare its tier in the spec"
        )),
    }
}

/// A chat line's tier: typed with real keys, so N1 as a step (the step
/// *is* typing that command) unless the spec says it stands in for a UI
/// action (N2); in setup and teardown a typed GM command is G.
pub fn resolve_chat(role: Role, declared: Option<Tier>) -> Result<Resolved, String> {
    match declared {
        Some(Tier::N3) | Some(Tier::X) => Err("a typed chat line is N1, N2 or G".to_string()),
        Some(d) => Ok(Resolved {
            tier: d,
            source: "spec",
        }),
        None => Ok(Resolved {
            tier: match role {
                Role::Step => Tier::N1,
                Role::Setup | Role::Teardown | Role::Anchor => Tier::G,
            },
            source: "role",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_is_most_native_first() {
        assert!(Tier::N1.satisfies(Tier::N2));
        assert!(Tier::N2.satisfies(Tier::N2));
        assert!(!Tier::N3.satisfies(Tier::N2));
        assert!(!Tier::X.satisfies(Tier::G));
    }

    #[test]
    fn a_spec_cannot_upgrade_lua_to_real_input() {
        assert!(resolve_tool("client_lua_eval", Some(Tier::N1)).is_err());
        assert_eq!(
            resolve_tool("client_lua_eval", None).unwrap().tier,
            Tier::N3
        );
        // Downgrading is fine: a typed slash command standing in for a click.
        assert_eq!(
            resolve_tool("client_input_key", Some(Tier::N2))
                .unwrap()
                .tier,
            Tier::N2
        );
    }

    #[test]
    fn unknown_driving_tools_need_a_declared_tier() {
        assert!(resolve_tool("client_future_tool", None).is_err());
        let r = resolve_tool("client_future_tool", Some(Tier::N1)).unwrap();
        // A planned tool in the capability table knows its own floor.
        assert_eq!(
            resolve_tool("client_world_click", None).unwrap().tier,
            Tier::N1
        );
        assert!(resolve_tool("client_item_action", Some(Tier::N1)).is_ok());
        assert_eq!(
            resolve_tool("client_inventory", None).unwrap().source,
            "read"
        );
        assert_eq!((r.tier, r.source), (Tier::N1, "spec"));
        // Server reads are evidence, not drives.
        assert_eq!(
            resolve_tool("server_entity_get", None).unwrap().source,
            "read"
        );
        assert_eq!(
            resolve_tool("server_console_exec", None).unwrap().tier,
            Tier::X
        );
    }

    #[test]
    fn reported_levels_in_both_shapes() {
        use serde_json::json;
        assert_eq!(
            from_reported_value(&json!("ui_lua")),
            Some((Tier::N3, "ui_lua".into()))
        );
        assert_eq!(
            from_reported_value(&json!({ "tier": "G", "label": "gm setup" })),
            Some((Tier::G, "G".into()))
        );
        assert_eq!(from_reported_value(&json!({ "tier": "read" })), None);
        assert_eq!(
            from_reported_value(&json!("real_input")).unwrap().0,
            Tier::N1
        );
    }

    #[test]
    fn chat_lines_default_by_role() {
        assert_eq!(resolve_chat(Role::Step, None).unwrap().tier, Tier::N1);
        assert_eq!(resolve_chat(Role::Setup, None).unwrap().tier, Tier::G);
        assert_eq!(
            resolve_chat(Role::Step, Some(Tier::N2)).unwrap().tier,
            Tier::N2
        );
        assert!(resolve_chat(Role::Step, Some(Tier::X)).is_err());
    }
}
