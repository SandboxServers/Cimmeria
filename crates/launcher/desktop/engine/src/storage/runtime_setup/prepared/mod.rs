//! Historical prerequisite evidence independent of the latest operation journal.
use super::*;
const NAME: &str = "runtime-selection.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    schema_version: u32,
    operation_id: Uuid,
    plan_digest: [u8; 32],
}
/// Checked prerequisite evidence, not permission to launch. The launch
/// coordinator must still lock/validate actual resources and graphics policy.
pub struct PreparedRuntime {
    pub plan: Plan,
}
impl DesktopState {
    /// Selecting a fresh attempt invalidates earlier success even when admission
    /// or dispatch subsequently fails. Never silently fall back to an old prefix.
    pub(super) fn select_runtime_attempt(&mut self, plan: &Plan) -> Result<(), IntentError> {
        self.write_runtime(
            NAME,
            &Selection {
                schema_version: 1,
                operation_id: plan.id,
                plan_digest: plan.digest()?,
            },
        )
    }
    pub fn prepared_runtime(&mut self) -> Result<Option<PreparedRuntime>, IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let Some(operation) = self.operations.snapshot().operation.as_ref() else {
            return Ok(None);
        };
        if !operation.state.terminal() {
            return Ok(None);
        }
        let Some(selected): Option<Selection> = read(&self.directory.root.join(NAME))? else {
            return Ok(None);
        };
        if selected.schema_version != 1 || selected.operation_id.is_nil() {
            return Err(StorageError::Corrupt.into());
        }
        if operation.id == selected.operation_id
            && (operation.kind != OperationKind::PrepareRuntime
                || operation.intent_digest != selected.plan_digest)
        {
            return Err(StorageError::Corrupt.into());
        }
        if operation.kind == OperationKind::PrepareRuntime
            && (operation.id != selected.operation_id
                || operation.state != OperationState::Succeeded)
        {
            return Ok(None);
        }
        let Some(installed) = self.installed_content()? else {
            return Ok(None);
        };
        let plan: Plan = read(&self.directory.root.join(plan_name(selected.operation_id)))?
            .ok_or(StorageError::Corrupt)?;
        if !plan.valid()
            || plan.id != selected.operation_id
            || plan.digest()? != selected.plan_digest
        {
            return Err(StorageError::Corrupt.into());
        }
        if plan.installation != installed.intent {
            return Ok(None);
        }
        let Some(record) = read_record(&self.directory.root, &plan)? else {
            return Ok(None);
        };
        if record.phase != Phase::Quiescent || !record.result.as_ref().is_some_and(verified_result)
        {
            return Ok(None);
        }
        if !install_worker::content_valid(
            &installed.intent.destination.join("game"),
            &installed.release,
        ) {
            return Ok(None);
        }
        Ok(Some(PreparedRuntime { plan }))
    }
}
#[cfg(test)]
mod tests;
