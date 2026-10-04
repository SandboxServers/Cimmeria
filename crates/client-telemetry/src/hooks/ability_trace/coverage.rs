//! The client's half of the ability telemetry coverage gate (AB-C7).
//!
//! These are the methods the client must account for: every
//! client-to-server ability method the stock client can send, and every
//! server-to-client one it receives. `tools/telemetry-coverage/abilities.py`
//! owns the method set; its `--check` fails when these tables differ from
//! the set minus its exceptions. The tests below fail when a declared
//! method does not resolve through the table its hook really uses, so a
//! method cannot drop out of the hooks without failing the build.
//!
//! The script scans the `(index, "name")` tuples, one per line.

/// Client-to-server methods the router hook must report as
/// `client.ability.sent` (cell-method index, `.def` name).
pub(crate) const CLIENT_SENDS: &[(u16, &str)] = &[
    (4, "confirmationResponse"),
    (68, "useAbility"),
    (69, "useAbilityOnGroundTarget"),
    (72, "resetMyAbilities"),
    (77, "trainAbility"),
    (88, "petInvokeAbility"),
    (89, "petAbilityToggle"),
    (169, "gmDebugAbility"),
    (170, "gmDebugCombat"),
    (171, "gmDebugCombatVerbose"),
    (172, "gmDebugHeal"),
    (176, "gmDebugAbilityOnMob"),
];

/// Server-to-client methods the `onEntityMethod` hook must report as
/// `client.ability.recv` (flat SGWPlayer client-method index, `.def` name).
pub(crate) const CLIENT_RECVS: &[(u16, &str)] = &[
    (1, "onSequence"),
    (12, "onTimerUpdate"),
    (14, "onEffectResults"),
    (19, "onStateFieldUpdate"),
    (20, "onStatUpdate"),
    (21, "onStatBaseUpdate"),
    (28, "onPlayerCommunication"),
    (101, "onKnownAbilitiesUpdate"),
    (121, "onErrorCode"),
    (141, "onAbilityTreeInfo"),
];

#[cfg(test)]
mod tests {
    use super::super::decode::spec_for;
    use super::super::recv_methods::{resolve, Receiver, PLAYER_FIRST_EXTENDED_ID};
    use super::*;

    /// The receive hook resolves a message by its wire id
    /// (`recv_methods::resolve`, as `recv::report` calls it): every
    /// declared method must come back from its own wire id for the local
    /// player, direct or extended.
    #[test]
    fn every_declared_recv_resolves_from_its_wire_id() {
        for &(index, name) in CLIENT_RECVS {
            let index = u32::from(index);
            let (msg_id, sub) = if index < PLAYER_FIRST_EXTENDED_ID {
                (index, None)
            } else {
                let rel = index - PLAYER_FIRST_EXTENDED_ID;
                (
                    PLAYER_FIRST_EXTENDED_ID + rel / 0x100,
                    Some((rel % 0x100) as u8),
                )
            };
            let (m, skip) = resolve(msg_id, Receiver::Player, sub)
                .unwrap_or_else(|| panic!("{name} ({index}) does not resolve"));
            assert_eq!(m.name, name);
            assert_eq!(u32::from(m.index), index, "{name}");
            assert_eq!(skip, usize::from(sub.is_some()), "{name}");
        }
    }

    /// The router hook (`inline_hooks::ability::route`) picks a method by
    /// `decode::spec_for(name)`: every declared send must resolve there,
    /// at the same index.
    #[test]
    fn every_declared_send_resolves_in_the_router_allowlist() {
        for &(index, name) in CLIENT_SENDS {
            let spec = spec_for(name)
                .unwrap_or_else(|| panic!("{name} ({index}) is not in decode::ALLOWLIST"));
            assert_eq!(spec.cell_index, index, "{name}");
        }
    }
}
