//! Adoption never offers or admits a separately copied installation.
use super::fixture::*;
use super::*;
use cimmeria_launcher_engine::adoption::AdoptionMode;

#[test]
fn adoption_offers_only_the_selected_folder_in_place() {
    let f = Fixture::new(serde_json::json!({}));
    assert_eq!(
        f.host.adoption_status().unwrap().modes,
        vec![AdoptionMode::Light]
    );
}

#[test]
fn a_copy_request_is_refused_before_a_picker_or_any_files_change() {
    let f = Fixture::new(serde_json::json!({}));
    let folder = f.source.game_directory.clone();
    let before = tree(&folder);
    assert_eq!(
        f.host.adoption_mode_allowed(AdoptionMode::Copy),
        Err(AdoptionError::UnsupportedMode)
    );
    assert_eq!(
        f.host
            .begin_folder_adoption(
                folder.clone(),
                Some(f.location()),
                AdoptionMode::Copy,
                f.release()
            )
            .unwrap_err(),
        AdoptionError::UnsupportedMode
    );
    assert_eq!(tree(&folder), before);
    assert!(!f.destination().exists());
}
