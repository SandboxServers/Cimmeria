//! Workbench metadata derived from each registered dot-console command.

use super::{Spec, COMMAND_METADATA_VERSION};

/// Presentation metadata for a registered command. A family packet may replace
/// the pending fields only after its per-command evidence has been reviewed.
pub(crate) struct WorkbenchMetadata {
    pub category: &'static str,
    pub availability: &'static str,
    pub disposition_reason: &'static str,
    pub metadata_version: u32,
    pub args: &'static [super::workbench_world_move::TypedArg],
    pub forms: &'static str,
    pub preconditions: &'static str,
    pub impact: &'static str,
    pub verify: &'static str,
    pub compensation: &'static str,
    pub impact_kind: &'static str,
    pub verify_tools: &'static [&'static str],
    pub comparison_fields: &'static [&'static str],
    pub compensation_kind: &'static str,
}

/// Return conservative metadata for a row in the only command registry.
pub(crate) fn workbench_metadata(spec: &Spec) -> WorkbenchMetadata {
    if let Some(metadata) = super::workbench_world_move::metadata(spec.name) {
        return metadata;
    }
    if let Some(metadata) = super::workbench_world_author::metadata(spec.name) {
        return metadata;
    }
    if let Some(metadata) = super::workbench_progression::metadata(spec.name) {
        return metadata;
    }
    WorkbenchMetadata {
        category: "Dot console",
        availability: "unsupported",
        disposition_reason: "Typed arguments and verification require reviewed command evidence",
        metadata_version: COMMAND_METADATA_VERSION,
        args: &[],
        forms: "",
        preconditions: "",
        impact: "",
        verify: "",
        compensation: "",
        impact_kind: "",
        verify_tools: &[],
        comparison_fields: &[],
        compensation_kind: "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::console::registry::{arg_specs, COMMANDS};

    #[test]
    fn every_registered_dot_command_has_pending_workbench_metadata() {
        assert_eq!(COMMANDS.len(), 130);
        for spec in COMMANDS {
            let metadata = workbench_metadata(spec);
            if super::super::workbench_world_move::metadata(spec.name).is_none()
                && super::super::workbench_world_author::metadata(spec.name).is_none()
                && super::super::workbench_progression::metadata(spec.name).is_none()
            {
                assert_eq!(metadata.availability, "unsupported", ".{}", spec.name);
            }
            if metadata.availability != "runnable" {
                assert!(!metadata.disposition_reason.is_empty(), ".{}", spec.name);
            }
            assert_eq!(metadata.metadata_version, COMMAND_METADATA_VERSION);
        }
    }

    #[test]
    fn despawn_target_argument_name_comes_from_registry_arg_spec() {
        let row = crate::cell::console::command_catalog()
            .into_iter()
            .find(|row| row.name == "despawn")
            .unwrap();
        assert_eq!(row.target_policy, "mob_or_id");
        assert_eq!(row.target_argument_name, Some(arg_specs("despawn")[0].name));
        assert_eq!(row.target_argument_name, Some("entityId"));
    }
}
