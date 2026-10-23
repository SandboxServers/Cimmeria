//! Applied content identity comes from the retained signed release and live ledger.
use super::*;
use crate::{catalog::VerifiedRelease, manifest::PatchRoot, state::InstalledState};

impl DesktopState {
    /// Best-effort observation only: never turns successful persistence into failure.
    /// Call after terminal publication, or successful Play admission, never staging.
    pub(crate) fn log_applied_content(&self, phase: &'static str) {
        let Ok(Some((installed, _))) = self.installed_for_launch() else {
            return;
        };
        let path = InstalledState::path(&installed.intent.destination.join("game"));
        let Ok(Some(ledger)) = read::<InstalledState>(&path) else {
            return;
        };
        emit(&installed.release, &ledger, phase);
    }
}

fn emit(release: &VerifiedRelease, ledger: &InstalledState, phase: &'static str) {
    let manifest_sha256: String = release
        .digest()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut overlay_ids = Vec::new();
    let mut incomplete_overlay_ids = Vec::new();
    for patch in &release.manifest().patches {
        if patch.root != PatchRoot::SgwGame || !ledger.has_applied_patch(patch) {
            continue;
        }
        if ledger.is_fully_applied(&patch.state_key()) {
            overlay_ids.push(patch.id.as_str());
        } else {
            incomplete_overlay_ids.push(patch.id.as_str());
        }
    }
    tracing::info!(target: "launcher.install", event = "applied_content_identity", phase,
        %manifest_sha256, overlay_ids = ?overlay_ids,
        incomplete_overlay_ids = ?incomplete_overlay_ids,
        "installed content identity");
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn logs_only_ledger_applied_overlays_with_root_suffix_and_partial_separation() {
        let body = serde_json::to_vec(&serde_json::json!({"schema":1,
            "seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},
            "patches":[
                {"id":"complete","blob":"a.zip","size":1,"sha256":"b".repeat(64),"root":"sgw_game"},
                {"id":"partial","blob":"b.zip","size":1,"sha256":"c".repeat(64),"after":"complete","root":"sgw_game"},
                {"id":"offered","blob":"c.zip","size":1,"sha256":"d".repeat(64),"after":"partial","root":"sgw_game"}
            ]})).unwrap();
        let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
        let signature: String = signature
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let release = crate::catalog::verify_release(&body, signature.as_bytes()).unwrap();
        let ledger = InstalledState {
            applied_patches: vec![
                "complete@sgw_game".into(),
                "partial@sgw_game".into(),
                "offered".into(),
            ],
            incomplete_patches: vec![crate::state::IncompletePatch {
                state_key: "partial@sgw_game".into(),
                skipped: vec![],
            }],
            ..Default::default()
        };
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let (_, events) = crate::test_capture::capture(|| {
            tracing::callsite::rebuild_interest_cache();
            emit(&release, &ledger, "applied");
        });
        let event = events
            .iter()
            .find(|event| event.field("event") == Some("applied_content_identity"))
            .unwrap();
        assert_eq!(
            event.field("manifest_sha256"),
            Some(
                release
                    .digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .as_str()
            )
        );
        assert_eq!(event.field("overlay_ids"), Some("[\"complete\"]"));
        assert_eq!(event.field("incomplete_overlay_ids"), Some("[\"partial\"]"));
    }
}
