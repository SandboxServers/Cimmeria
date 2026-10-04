//! The server receipt row for every client-to-server ability method
//! (ability-mechanics AB-C7, the "server recv row" column of
//! `docs/analysis/ability-mechanics/telemetry-coverage.md`).
//!
//! [`ABILITY_RECEIPTS`] is read by `tools/telemetry-coverage/abilities.py`
//! (it scans the `generic(index, "method")` calls and the `index:` /
//! `method:` fields of the `AbilityReceipt { .. }` literals), so keep the
//! entries in those two shapes. A method whose handler already
//! writes a richer receipt row of its own (`useAbility`,
//! `useAbilityOnGroundTarget`, both in
//! `cimmeria-cell-methods` `player/combat/mod.rs`) names that row's event;
//! every other method gets [`GENERIC_EVENT`], written here by the router
//! before the GM gate, so a call the gate refuses still has its receipt.

use super::super::space_manager::SpaceManager;

/// The event of the router's own receipt row.
pub const GENERIC_EVENT: &str = "ability_method_recv";

/// One client-to-server ability method and the event of its receipt row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityReceipt {
    /// Flat cell-method index (`docs/protocol/cell-method-dispatch-table.md`).
    pub index: u16,
    /// The `.def` method name.
    pub method: &'static str,
    /// The `event =` of the row that records its receipt.
    pub event: &'static str,
}

const fn generic(index: u16, method: &'static str) -> AbilityReceipt {
    AbilityReceipt {
        index,
        method,
        event: GENERIC_EVENT,
    }
}

/// Every client-to-server method of the AB-C7 ability set, with its
/// receipt row. The set itself is defined in
/// `tools/telemetry-coverage/abilities.py`; its `--check` fails when this
/// table and the script disagree.
pub const ABILITY_RECEIPTS: &[AbilityReceipt] = &[
    generic(2, "toggleCombatDebug"),
    generic(3, "toggleCombatVerboseDebug"),
    generic(4, "confirmationResponse"),
    AbilityReceipt {
        index: 68,
        method: "useAbility",
        event: "use_ability_recv",
    },
    AbilityReceipt {
        index: 69,
        method: "useAbilityOnGroundTarget",
        event: "use_ability_on_ground_recv",
    },
    generic(72, "resetMyAbilities"),
    generic(77, "trainAbility"),
    generic(88, "petInvokeAbility"),
    generic(89, "petAbilityToggle"),
    generic(169, "gmDebugAbility"),
    generic(170, "gmDebugCombat"),
    generic(171, "gmDebugCombatVerbose"),
    generic(172, "gmDebugHeal"),
    generic(176, "gmDebugAbilityOnMob"),
];

/// The receipt entry for `index`, if it is an ability method.
pub fn receipt(index: u16) -> Option<&'static AbilityReceipt> {
    ABILITY_RECEIPTS.iter().find(|r| r.index == index)
}

/// Write the router's receipt row for `method_index` when it is an ability
/// method without a handler-owned row. Stage `recv`, target `abilities`,
/// DEBUG (D-AU5). `mercury_seq` is the inbound packet's seq, the join to
/// the client's `client.ability.sent_seq` (AB-C1); the arguments are not
/// decoded here (the handlers own their layouts), only counted.
pub fn log_receipt(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    packet_seq: Option<u32>,
    space_mgr: &SpaceManager,
) {
    let Some(r) = receipt(method_index) else {
        return;
    };
    if r.event != GENERIC_EVENT {
        return;
    }
    let who = space_mgr.player_identity(entity_id);
    tracing::debug!(
        target: "abilities",
        event = GENERIC_EVENT,
        stage = "recv",
        method = r.method,
        method_index,
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        args_len = args.len(),
        mercury_seq = packet_seq,
        "ability method received"
    );
}
