//! Abandon reviewed per-file changes, or release legacy interrupted ownership
//! without resuming or reversing the old installation-relocation algorithm.
use super::in_place_check;
use super::publication::{name, read_large};
use super::*;

fn phase(state: &DesktopState, plan: &Plan) -> Option<Phase> {
    let record =
        read_large::<Record>(&state.directory.root.join(name(plan.provenance.work_id))).ok()?;
    (record.plan == *plan).then_some(record.phase)
}

pub(super) fn abandonable(state: &DesktopState, plan: &Plan) -> bool {
    if plan.schema_version == 1 {
        return phase(state, plan).is_some();
    }
    if plan.schema_version != 2 {
        return false;
    }
    match phase(state, plan) {
        Some(Phase::Staged) => true,
        Some(Phase::Promoted) => {
            matches!(plan.provenance.check, FileCheck::Light)
                && in_place_check::has_setup_originals(&plan.stage())
        }
        _ => false,
    }
}

pub(super) fn abandon(state: &mut DesktopState, plan: &Plan) -> Result<(), Error> {
    if !abandonable(state, plan) {
        return Err(ContractError::InvalidTransition.into());
    }
    super::direct::abandon(
        state,
        plan,
        phase(state, plan).ok_or(StorageError::Corrupt)?,
    )
}
