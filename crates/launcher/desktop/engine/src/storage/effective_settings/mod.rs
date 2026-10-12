//! Effective launch settings of a verified adopted copy. Fresh installs follow
//! the "Load client patches" preference; adoption consumes what the user
//! reviewed, and only while every independent record of it still agrees.
use super::*;
use crate::storage::adoption::Provenance;

#[cfg(all(target_os = "macos", any(test, feature = "test-support")))]
pub mod fixtures;

/// Patch policy of the installed copy, decided natively. Never renderer input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaunchBinding {
    pub client_patches_enabled: bool,
}

/// What [`DesktopState::injection_policy`] decides. A DLL the policy wants is
/// still loaded only when the native bundle carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InjectionPolicy {
    pub client_patches: bool,
    pub client_telemetry: bool,
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
    // A folder adoption has no saved import to bind, so its plan never matches here.
    if plan.imported.as_ref() != Some(&imported)
        || provenance.import_digest.as_ref() != Some(&imported.confirmation)
        || provenance.legacy_install_id != Some(imported.identity.install_id)
        || plan.report.requested_config.as_ref() != Some(&imported.config)
        || plan.report.imported_identity.as_ref() != Some(&imported.identity)
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

    /// Which DLLs Play puts into `SGW.exe`, before the bundle is consulted.
    /// Play and the "Changes to your client" list both read this, so the list
    /// cannot name a DLL Play would not load, or leave one out.
    pub fn injection_policy(&self) -> Result<InjectionPolicy, StorageError> {
        let binding = self.effective_launch_binding()?;
        Ok(InjectionPolicy {
            // An adopted copy launches with the patch choice the player reviewed
            // at adoption, whatever "Load client patches" says now: its binding
            // is re-derived from the import record and admission enforces it, so
            // the preference cannot override it. Everything else follows the
            // preference.
            client_patches: match binding {
                Some(binding) => binding.client_patches_enabled,
                None => self.preferences.client_patches_enabled,
            },
            // The telemetry DLL goes only to an installation this launcher made,
            // for a player who opted in here. An adopted copy's imported
            // telemetry choice is still not honoured (see `adoption`), and an
            // unreadable choice means no telemetry rather than no Play.
            client_telemetry: binding.is_none()
                && self.game_telemetry().is_ok_and(|choice| choice.opted_in),
        })
    }

    /// The resources this installation may launch with, from the native bundle.
    /// Ok(None) means the policy injects patches and the bundle has no verified
    /// patch artifact. Patches off, by preference or by adoption, never needs it.
    pub fn resolve_play_resources(
        &self,
        bundled: launch::Resources,
    ) -> Result<Option<launch::Resources>, StorageError> {
        let policy = self.injection_policy()?;
        let bundled = launch::Resources {
            client_telemetry: bundled.client_telemetry.filter(|_| policy.client_telemetry),
            ..bundled
        };
        let enabled = policy.client_patches;
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
    if binding.client_patches_enabled != resources.client_patches.is_some()
        || resources.client_telemetry.is_some()
    {
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
