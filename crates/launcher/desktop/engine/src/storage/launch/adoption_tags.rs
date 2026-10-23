//! The adoption tags of a launch: how the launched copy was adopted and how many
//! of its patches are still incomplete.
use super::*;

/// The adoption tags of the installed copy about to launch. A copy that cannot be
/// read as installed launches untagged (`none`, 0), and the read failure is logged
/// without its text.
pub(super) fn adoption_tags(owner: &DesktopState) -> (&'static str, u64) {
    match owner.installed_for_launch() {
        Ok(Some((installed, provenance))) => {
            let game = installed.intent.content_root();
            launch_tags(provenance.as_ref(), &game)
        }
        Ok(None) => ("none", 0),
        Err(_) => {
            tracing::warn!(
                target: "launcher.launch",
                event = "launch_tags_unavailable",
                "launch: the adoption tags could not be read"
            );
            ("none", 0)
        }
    }
}

/// The adoption tags of an admitted launch: the provenance's check (`light`,
/// `full`, or `none` for a downloaded install) and how many patches the copy's
/// own ledger lists as incomplete. An unreadable ledger counts as none.
pub(super) fn launch_tags(
    provenance: Option<&crate::storage::adoption::Provenance>,
    game: &std::path::Path,
) -> (&'static str, u64) {
    let check = match provenance.map(|p| p.check) {
        Some(crate::storage::adoption::FileCheck::Light) => "light",
        Some(crate::storage::adoption::FileCheck::Full) => "full",
        None => "none",
    };
    let incomplete = crate::state::InstalledState::load(game)
        .incomplete_patches
        .len();
    (check, u64::try_from(incomplete).unwrap_or(0))
}
