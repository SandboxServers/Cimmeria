//! Installed content must authenticate the retained direct-root layout plan.
use super::in_place_tests::{commit, Fixture};
use super::*;

#[test]
fn installed_content_requires_the_retained_adoption_plan() {
    for corruption in ["missing", "schema", "intent"] {
        let f = Fixture::new(None);
        let provenance = commit(f.preview().unwrap()).unwrap();
        let mut state = f.state.lock().unwrap();
        assert!(state.has_valid_install().unwrap());
        let path = state
            .directory
            .root
            .join(publication::plan_name(provenance.work_id));
        if corruption == "missing" {
            std::fs::remove_file(path).unwrap();
        } else {
            let mut value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            if corruption == "schema" {
                value["schema_version"] = serde_json::json!(1);
            } else {
                value["installation"]["schema_version"] = serde_json::json!(1);
            }
            std::fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        }
        assert!(
            state.installed_content().is_err(),
            "accepted {corruption} plan"
        );
        assert!(
            state.has_valid_install().is_err(),
            "validity accepted {corruption} plan"
        );
    }
}
