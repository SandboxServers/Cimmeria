//! Which tools need the lease.
//!
//! The rule: a tool that changes the client, drives its input or UI, runs
//! client code chosen by the caller, or advances a shared read cursor needs
//! the lease. A tool that only looks stays open, so any session can watch
//! while one drives.
//!
//! Cursor reads: `client_events_read`, `client_wait_event`,
//! `client_chat_log` and `client_combat_log` move named cursors in the one
//! event store, and the chat and combat logs install their client-side
//! capture on first use. Two sessions reading through the same cursor
//! would steal each other's events, so these are the driver's and need the
//! lease. Observers use screenshots, the UI readers, `lab_timeline` and
//! SigNoz instead.
//!
//! `client_wait_for` is guarded because its predicate is caller-chosen Lua.
//!
//! Hidden writes (review 2026-10-04): `client_entity_find` pins shared unit
//! slots and the single projection request slot, and `client_inventory`
//! stores named snapshots; both are leased. The other open tools run fixed
//! read-only Lua (their only global writes are the idempotent `__jenc` /
//! `__jcall` helper definitions), memory reads or window captures.
//!
//! Anything not in [`OPEN`] needs the lease, so a new tool is guarded until
//! someone decides otherwise; `every_routed_tool_is_classified` makes that
//! decision explicit.

/// How a tool is gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Read-only: no lease needed.
    Open,
    /// Needs `lease_id`; a valid one renews the lease.
    Lease,
}

/// Tools that only look. Everything else needs the lease.
pub const OPEN: &[&str] = &[
    // The lease tools themselves (renew/release check their own id).
    "lab_lease_acquire",
    "lab_lease_renew",
    "lab_lease_release",
    "lab_lease_status",
    // Supervisor reads.
    "lab_client_status",
    "lab_crash_report",
    "lab_timeline",
    "lab_uat_report",
    // Pixels.
    "lab_screenshot",
    "lab_screenshot_region",
    "lab_pixel_probe",
    // Fixed client reads (no caller code, no cursor).
    "client_module_info",
    "client_mem_read",
    "client_hook_list",
    "client_input_status",
    "client_entity_table",
    "client_ui_state",
    "client_window_read",
    "client_player_state",
    "client_hotbar",
    "lab_characters",
];

/// Tools that need the lease. Not consulted by [`gate`] (anything not
/// [`OPEN`] is guarded); listed so the classification test can prove every
/// routed tool was decided on, and so the docs have one list to cite.
#[cfg_attr(not(test), allow(dead_code))]
pub const LEASED: &[&str] = &[
    // Process lifecycle.
    "lab_client_start",
    "lab_client_stop",
    "lab_client_restart",
    // Caller-chosen code and memory.
    "client_lua_eval",
    "client_wait_for",
    "client_mem_write",
    "client_call_native",
    "client_console",
    "client_hook_install",
    "client_hook_remove",
    // Hidden shared writes: entity_find pins the shared private unit slots
    // and the one LabWorld projection request (supervisor/world/find.rs,
    // world/lua.rs), so an observer's call can rename or time out the
    // driver's lookup; inventory's `snapshot` writes the supervisor's
    // shared snapshot table.
    "client_entity_find",
    "client_inventory",
    // Shared cursors.
    "client_events_read",
    "client_wait_event",
    "client_chat_log",
    "client_combat_log",
    // Input.
    "client_input_key",
    "client_input_mouse",
    "client_input_focus",
    "client_input_release",
    "client_cursor_move",
    "client_type_text",
    "client_ui_click",
    "client_window_click",
    "client_drag_drop",
    // World and combat actions.
    "client_world_click",
    "client_target",
    "client_move_to",
    "client_camera",
    "client_use_ability",
    "client_item_action",
    "client_die_and_respawn",
    // Flows.
    "lab_login",
    "lab_logout",
    "lab_play_character",
    "lab_create_character",
    "lab_delete_character",
    "lab_ensure_character_slot",
    "lab_finish_dialog",
    // UAT: a run drives everything; an attest writes the evidence bundle.
    "lab_uat_run",
    "lab_uat_attest",
];

/// The gate for `tool`. Unknown names need the lease (fail closed).
pub fn gate(tool: &str) -> Gate {
    if OPEN.contains(&tool) {
        Gate::Open
    } else {
        Gate::Lease
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_tools_need_the_lease() {
        assert_eq!(gate("client_some_future_tool"), Gate::Lease);
        assert_eq!(gate("lab_client_status"), Gate::Open);
        assert_eq!(gate("client_lua_eval"), Gate::Lease);
    }

    /// Regression guard (review 2026-10-04): tools with hidden shared
    /// writes are not open.
    #[test]
    fn tools_with_hidden_writes_need_the_lease() {
        assert_eq!(gate("client_entity_find"), Gate::Lease);
        assert_eq!(gate("client_inventory"), Gate::Lease);
    }

    #[test]
    fn no_tool_is_both_open_and_leased() {
        for t in OPEN {
            assert!(!LEASED.contains(t), "{t} is in both lists");
        }
    }
}
