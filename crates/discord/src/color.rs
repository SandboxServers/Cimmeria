//! Embed color palette. Each [`crate::event::Severity`] maps to a fixed
//! integer color value (Discord embed `color` field is a 24-bit RGB int).
//!
//! Colors picked from a standard ops-dashboard palette so the channel is
//! glanceable: green for "things working," yellow for "watch this," red for
//! "go look," blue-grey for "FYI," purple for "GM did a thing."

use crate::event::Severity;

/// `0x2ECC71` — emerald green. Server up, level up.
pub const GREEN: u32 = 0x002E_CC71;
/// `0xF1C40F` — amber. Warnings, disconnects, latency, performance alerts.
pub const YELLOW: u32 = 0x00F1_C40F;
/// `0xE74C3C` — alizarin red. Errors, panics, auth fails, DB faults.
pub const RED: u32 = 0x00E7_4C3C;
/// `0x95A5A6` — slate. Informational (logins, world entry, chat).
pub const GREY: u32 = 0x0095_A5A6;
/// `0x9B59B6` — amethyst. GM-issued actions — visually distinct from
/// player actions to catch unauthorised GM use at a glance.
pub const PURPLE: u32 = 0x009B_59B6;

pub const fn for_severity(severity: Severity) -> u32 {
    match severity {
        Severity::Good => GREEN,
        Severity::Info => GREY,
        Severity::Warn => YELLOW,
        Severity::Error => RED,
        Severity::Privileged => PURPLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin that every `Severity` maps to a documented color and that the
    /// colors all fit in a 24-bit RGB value (high byte = 0).
    #[test]
    fn every_severity_maps_to_24bit_color() {
        for sev in [
            Severity::Good,
            Severity::Info,
            Severity::Warn,
            Severity::Error,
            Severity::Privileged,
        ] {
            let c = for_severity(sev);
            assert!(c <= 0x00FF_FFFF, "color must fit in 24 bits");
        }
    }

    /// Pin the palette so a refactor that reorders or renames doesn't
    /// silently change Discord rendering.
    #[test]
    fn palette_is_stable() {
        assert_eq!(for_severity(Severity::Good), GREEN);
        assert_eq!(for_severity(Severity::Info), GREY);
        assert_eq!(for_severity(Severity::Warn), YELLOW);
        assert_eq!(for_severity(Severity::Error), RED);
        assert_eq!(for_severity(Severity::Privileged), PURPLE);
    }
}
