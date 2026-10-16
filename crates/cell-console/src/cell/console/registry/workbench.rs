//! Workbench metadata derived from each registered dot-console command.

use super::{Spec, COMMAND_METADATA_VERSION};

/// Presentation metadata for a registered command. A family packet may replace
/// the pending fields only after its per-command evidence has been reviewed.
pub(crate) struct WorkbenchMetadata {
    pub category: &'static str,
    pub availability: &'static str,
    pub disposition_reason: &'static str,
    pub metadata_version: u32,
}

/// Return conservative metadata for a row in the only command registry.
pub(crate) fn workbench_metadata(_spec: &Spec) -> WorkbenchMetadata {
    WorkbenchMetadata {
        category: "Dot console",
        availability: "unsupported",
        disposition_reason: "Typed arguments and verification require reviewed command evidence",
        metadata_version: COMMAND_METADATA_VERSION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::console::registry::COMMANDS;

    #[test]
    fn every_registered_dot_command_has_pending_workbench_metadata() {
        assert_eq!(COMMANDS.len(), 129);
        for spec in COMMANDS {
            let metadata = workbench_metadata(spec);
            assert_eq!(metadata.availability, "unsupported", ".{}", spec.name);
            assert!(!metadata.disposition_reason.is_empty(), ".{}", spec.name);
            assert_eq!(metadata.metadata_version, COMMAND_METADATA_VERSION);
        }
    }
}
