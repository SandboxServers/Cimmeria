//! Which tools claim an effect (C3). A tool in [`JUDGED`] must carry evidence,
//! one in [`NO_CLAIM`] or `policy::OPEN` is passed through, and any other
//! routed tool is unjudged until a packet lists it.

use crate::lease::policy;

/// How the gate treats a tool's result. The names are fixed by contract C3.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    /// The result must carry evidence.
    Judged,
    /// The result is a claim, not a fact, until the tool is judged.
    Unjudged,
    /// Reads, lease, UAT and lifecycle tools: the gate leaves them alone.
    NoClaim,
}

/// Tools whose results carry evidence. Grows packet by packet.
pub const JUDGED: &[&str] = &[
    "client_world_click",
    "client_target",
    "client_move_to",
    "client_camera",
    "lab_recover",
    "client_item_action",
    "client_drag_drop",
    "client_vendor_buy",
    "client_world_map_click",
];

/// Tools that claim no effect: reads, lease, UAT and lifecycle tools.
pub const NO_CLAIM: &[&str] = &[
    // Caller-chosen code, memory and hooks.
    "client_lua_eval",
    "client_wait_for",
    "client_mem_write",
    "client_call_native",
    "client_console",
    "client_hook_install",
    "client_hook_remove",
    // Reads of shared cursors and tables.
    "client_events_read",
    "client_entity_find",
    "client_inventory",
    "client_chat_log",
    "client_combat_log",
    "client_wait_event",
    "client_batch",
    "client_hover_probe",
    // Process lifecycle.
    "lab_client_start",
    "lab_client_stop",
    "lab_client_restart",
    // UAT and recording.
    "lab_uat_run",
    "lab_uat_calibrate",
    "lab_uat_attest",
    "lab_record_start",
    "lab_route_record",
    "lab_route_stop",
];

/// The claim a tool makes. Open tools and [`NO_CLAIM`] make none, [`JUDGED`]
/// tools make one that needs evidence, and every other tool is unjudged.
pub fn claim(tool: &str) -> Claim {
    if policy::OPEN.contains(&tool) || NO_CLAIM.contains(&tool) {
        Claim::NoClaim
    } else if JUDGED.contains(&tool) {
        Claim::Judged
    } else {
        Claim::Unjudged
    }
}

#[cfg(test)]
#[path = "claims_tests.rs"]
mod tests;
