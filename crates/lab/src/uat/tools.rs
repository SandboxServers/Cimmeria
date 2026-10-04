//! The capability table: the one place that maps what a spec wants
//! (`@world_click`) to the lab tool that does it (`client_world_click`),
//! with the most native tier that tool can claim.
//!
//! Specs may name a tool directly or by `@capability`. Tools other changes
//! are still building are listed here by their planned names (2026-09-29);
//! when one lands under a different name, change its `tool` here and every
//! spec that uses the alias follows. Until a tool is routed, a row that
//! needs it is BLOCKED naming it.

use super::lab_commands;
use super::spec::{ActionSpec, SectionSpec};
use super::tier::Tier;

/// What `@target_player` resolves to. Not a router tool: the runner
/// expands it into `client_target` naming the other lab client's
/// character, on the client the action runs on (two-player rows).
pub const TARGET_PLAYER_TOOL: &str = "uat_target_player";

/// One capability.
#[derive(Debug, Clone, Copy)]
pub struct Capability {
    /// The name specs use after `@`.
    pub alias: &'static str,
    /// The MCP tool that provides it.
    pub tool: &'static str,
    /// The most native tier the tool can claim; `None` for a read.
    pub floor: Option<Tier>,
}

const fn drive(alias: &'static str, tool: &'static str, floor: Tier) -> Capability {
    Capability {
        alias,
        tool,
        floor: Some(floor),
    }
}

const fn read(alias: &'static str, tool: &'static str) -> Capability {
    Capability {
        alias,
        tool,
        floor: None,
    }
}

/// Every capability a spec may name. The first two blocks are on `main`
/// (#1080, #1099); the rest are planned tools (matrix backlog ids).
pub const CAPABILITIES: &[Capability] = &[
    // On main.
    read("ui_state", "client_ui_state"),
    read("wait_for", "client_wait_for"),
    read("entity_table", "client_entity_table"),
    read("screenshot", "lab_screenshot"),
    read("screenshot_region", "lab_screenshot_region"),
    read("pixel_probe", "lab_pixel_probe"),
    drive("key", "client_input_key", Tier::N1),
    drive("ui_click", "client_ui_click", Tier::N1),
    drive("type_text", "client_type_text", Tier::N1),
    drive("login", "lab_login", Tier::N1),
    drive("play", "lab_play_character", Tier::N1),
    drive("logout", "lab_logout", Tier::N1),
    drive("finish_dialog", "lab_finish_dialog", Tier::N1),
    // World (L4, L5, L15, L17): on main since #1099. Each reports its own
    // `native_level`, which the runner applies over the floor here.
    read("entity_find", "client_entity_find"),
    drive("world_click", "client_world_click", Tier::N1),
    drive("target", "client_target", Tier::N1),
    drive("move_to", "client_move_to", Tier::N1),
    drive("camera", "client_camera", Tier::N1),
    // Two-player rows (AB-L6): `client_target` on the other player's
    // character by name, with real input. Expanded by the runner.
    drive("target_player", TARGET_PLAYER_TOOL, Tier::N1),
    // UI and items (L1, L2, L3, L8, L9, L10).
    read("window_read", "client_window_read"),
    drive("window_click_row", "client_window_click_row", Tier::N1),
    read("chat_log", "client_chat_log"),
    read("inventory", "client_inventory"),
    read("player_state", "client_player_state"),
    drive("item_action", "client_item_action", Tier::N1),
    drive("drag_drop", "client_drag_drop", Tier::N1),
    // Abilities, combat, events (L6, L7, L12, L16): on main since #1100;
    // each reports `native_level: {tier, label}`.
    drive("use_ability", "client_use_ability", Tier::N1),
    read("combat_log", "client_combat_log"),
    // Death needs a real lethal hit: `/gmsethealth 0` never runs the death
    // sequence. The tool reports its own tier per result.
    drive("die_and_respawn", "client_die_and_respawn", Tier::G),
    read("wait_event", "client_wait_event"),
    read("hotbar", "client_hotbar"),
    // Ability lab (AB-L3). The three dot commands from AB-L2 are typed
    // into chat by the runner (`super::lab_commands`), which waits for the
    // command's own feedback line; `@ability_state` is the server read
    // (AB-L1) for `source = "server"` clauses.
    drive(
        "cooldowns_reset",
        lab_commands::COOLDOWNS_RESET_TOOL,
        Tier::G,
    ),
    drive("dummy", lab_commands::DUMMY_TOOL, Tier::G),
    drive("clear_effects", lab_commands::CLEAR_EFFECTS_TOOL, Tier::G),
    read("ability_state", "server_ability_state"),
    // Not assigned yet (L11, L18).
    drive("chat_send", "client_chat_send", Tier::N1),
    drive("cache_files", "client_cache_files", Tier::N1),
];

/// The capability a tool name or `@alias` refers to.
pub fn lookup(name: &str) -> Option<&'static Capability> {
    match name.strip_prefix('@') {
        Some(alias) => CAPABILITIES.iter().find(|c| c.alias == alias),
        None => CAPABILITIES.iter().find(|c| c.tool == name),
    }
}

/// `@alias` → its tool name; any other name unchanged.
pub fn resolve(name: &str) -> Result<String, String> {
    if name.starts_with('@') {
        lookup(name)
            .map(|c| c.tool.to_string())
            .ok_or_else(|| format!("unknown capability {name}"))
    } else {
        Ok(name.to_string())
    }
}

fn resolve_action(a: &mut ActionSpec, errs: &mut Vec<String>) {
    if let Some(t) = &a.tool {
        match resolve(t) {
            Ok(r) => a.tool = Some(r),
            Err(e) => errs.push(e),
        }
    }
    for f in &mut a.fallback {
        resolve_action(f, errs);
    }
}

/// Replace every `@alias` in a section with its tool name.
pub fn resolve_section(spec: &mut SectionSpec) -> Result<(), String> {
    let mut errs = Vec::new();
    for row in &mut spec.rows {
        for a in row
            .setup
            .iter_mut()
            .chain(&mut row.steps)
            .chain(&mut row.teardown)
        {
            resolve_action(a, &mut errs);
        }
        for c in &mut row.expect {
            if let Some(t) = &c.tool {
                match resolve(t) {
                    Ok(r) => c.tool = Some(r),
                    Err(e) => errs.push(format!("{}/{}: {e}", row.id, c.id)),
                }
            }
        }
        for e in &mut row.evidence {
            match resolve(&e.tool) {
                Ok(r) => e.tool = r,
                Err(x) => errs.push(format!("{}/evidence {}: {x}", row.id, e.name)),
            }
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_tools_are_unique() {
        let mut a = std::collections::HashSet::new();
        let mut t = std::collections::HashSet::new();
        for c in CAPABILITIES {
            assert!(a.insert(c.alias), "duplicate alias {}", c.alias);
            assert!(t.insert(c.tool), "duplicate tool {}", c.tool);
        }
    }

    #[test]
    fn an_alias_resolves_and_an_unknown_one_is_an_error() {
        assert_eq!(resolve("@world_click").unwrap(), "client_world_click");
        assert_eq!(resolve("client_ui_state").unwrap(), "client_ui_state");
        assert!(resolve("@nope").is_err());
        assert_eq!(lookup("client_item_action").unwrap().floor, Some(Tier::N1));
        assert_eq!(lookup("@inventory").unwrap().floor, None);
        assert_eq!(resolve("@dummy").unwrap(), "uat_dummy");
        assert_eq!(lookup("@cooldowns_reset").unwrap().floor, Some(Tier::G));
        assert_eq!(resolve("@ability_state").unwrap(), "server_ability_state");
    }
}
