//! Effective launch settings of a verified adopted copy. Fresh installs keep the
//! bundled patch contract; adoption consumes what the user reviewed, and only
//! while every independent record of it still agrees.
use super::*;
use crate::storage::adoption::Provenance;

#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
pub mod fixtures;

/// Patch policy of the installed copy, decided natively. Never renderer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchBinding {
    pub client_patches_enabled: bool,
}

/// The reviewed settings of a published adoption. The plan alone is not trusted:
/// the installed index names the import digest, and the retained import record
/// re-derives the same settings and digest from the exact legacy JSON.
pub(super) fn launch_binding(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<LaunchBinding, StorageError> {
    let plan = super::adoption::published_plan(state, intent, provenance)?;
    let imported = state
        .legacy_import()
        .map_err(|error| match error {
            migration::MigrationError::Storage(error) => error,
            _ => StorageError::Corrupt,
        })?
        .ok_or(StorageError::Corrupt)?;
    if imported != plan.imported
        || imported.confirmation != provenance.import_digest
        || imported.identity.install_id != provenance.legacy_install_id
        || plan.report.requested_config != imported.config
        || plan.report.imported_identity != imported.identity
    {
        return Err(StorageError::Corrupt);
    }
    let config = &imported.config;
    // Adoption already refuses both; a record claiming otherwise was not reviewed.
    if config.manifest_url != crate::catalog::URL || config.client_patches.dll_override.is_some() {
        return Err(StorageError::UnsupportedSchema);
    }
    // Game telemetry is unavailable here. An opted-in import launches only with
    // that reviewed acceptance; identity, consent and auth URL stay as imported.
    if !plan.choices.old_game_closed
        || (config.telemetry.opted_in && !plan.choices.accept_unavailable_game_telemetry)
    {
        return Err(StorageError::Corrupt);
    }
    let reviewed = config.login_servers.iter().map(|s| (&s.name, &s.url));
    if !reviewed.eq(intent.login_servers.iter().map(|s| (&s.name, &s.url))) {
        return Err(StorageError::Corrupt);
    }
    Ok(LaunchBinding {
        client_patches_enabled: config.client_patches.enabled,
    })
}

impl DesktopState {
    /// None for fresh installs, and when nothing is installed.
    pub fn effective_launch_binding(&self) -> Result<Option<LaunchBinding>, StorageError> {
        match self.installed_for_launch()? {
            Some((installed, Some(provenance))) => {
                launch_binding(self, &installed.intent, &provenance).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// The resources this installation may launch with, from the native bundle.
    /// Ok(None) means the policy injects patches and the bundle has no verified
    /// patch artifact. A copy adopted with patches off never needs that artifact.
    pub fn resolve_play_resources(
        &self,
        bundled: launch::Resources,
    ) -> Result<Option<launch::Resources>, StorageError> {
        let enabled = self
            .effective_launch_binding()?
            .is_none_or(|binding| binding.client_patches_enabled);
        Ok(match (enabled, bundled.client_patches.is_some()) {
            (true, true) => Some(bundled),
            (true, false) => None,
            (false, _) => Some(launch::Resources {
                client_patches: None,
                ..bundled
            }),
        })
    }
}

/// Admission-side enforcement: the caller's resources must already agree with the
/// binding, so a host that skipped resolution cannot inject or drop the patch.
pub(super) fn verify_launch_resources(
    binding: LaunchBinding,
    resources: &launch::Resources,
) -> Result<(), IntentError> {
    if binding.client_patches_enabled != resources.client_patches.is_some() {
        return Err(ContractError::IdentityConflict.into());
    }
    resources.verify()
}

#[cfg(all(test, target_os = "macos"))]
mod adopted_tests;
#[cfg(test)]
mod tests;
#[cfg(all(test, target_os = "macos"))]
mod wine_tests;
