use super::*;

#[test]
fn classify_rosetta_timeout_is_timed_out() {
    assert_eq!(classify_rosetta(None), RosettaProbe::TimedOut);
}

#[test]
fn classify_rosetta_spawn_error_is_error() {
    let err = std::io::Error::from(std::io::ErrorKind::NotFound);
    assert_eq!(classify_rosetta(Some(Err(err))), RosettaProbe::Error);
}

#[test]
fn classify_rosetta_exit_zero_is_present() {
    assert_eq!(classify_rosetta(Some(Ok(true))), RosettaProbe::Present);
}

#[test]
fn classify_rosetta_nonzero_exit_is_missing() {
    assert_eq!(classify_rosetta(Some(Ok(false))), RosettaProbe::Missing);
}

#[test]
fn translocated_detects_app_translocation_mount() {
    let exe = Path::new(
        "/private/var/folders/x/y/T/AppTranslocation/ABC/d/Cimmeria Launcher.app/Contents/MacOS/x",
    );
    assert!(translocated(exe));
}

#[test]
fn translocated_is_false_in_applications() {
    let exe = Path::new("/Applications/Cimmeria Launcher.app/Contents/MacOS/x");
    assert!(!translocated(exe));
}

#[test]
fn translocated_needs_a_whole_component() {
    let exe = Path::new("/Users/p/AppTranslocationNotes/Cimmeria Launcher.app/Contents/MacOS/x");
    assert!(!translocated(exe));
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn rosetta_probe_answers_present_or_missing() {
    let probe = rosetta().await;
    assert!(
        matches!(probe, RosettaProbe::Present | RosettaProbe::Missing),
        "{probe:?}"
    );
}
