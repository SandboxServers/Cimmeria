//! Crash, foreign-edit and nested-link regression journeys for direct adoption.
use super::in_place_tests::{confirm_replacing, stored_replacement, Fixture};
use super::*;
const EXE: &str = "Working/Binaries/SGW.exe";

use super::light_tests::Light;
impl Light {
    fn folder(&self) -> &Path {
        &self.folder
    }
    fn operation(&self) -> (Uuid, OperationState) {
        let state = self.state.lock().unwrap();
        let op = state.operations().snapshot().operation.clone().unwrap();
        (op.id, op.state)
    }
    fn reopen(mut self) -> Self {
        let path = self.root.path().join("state");
        // Replace the Arc, closing the old store before reopening it.
        let old = self.state;
        drop(old);
        self.state = Arc::new(Mutex::new(DesktopState::open(&path).unwrap()));
        self
    }
}
fn at(wanted: publication::Point, point: publication::Point) -> Result<(), Error> {
    if point == wanted {
        Err(StorageError::Io.into())
    } else {
        Ok(())
    }
}
fn recover_now(f: &Arc<Mutex<DesktopState>>) -> Result<(), Error> {
    let mut state = f.lock().unwrap();
    let id = state.operations().snapshot().operation.as_ref().unwrap().id;
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision)
}
fn abandon_now(f: &Arc<Mutex<DesktopState>>) -> Result<(), Error> {
    let mut state = f.lock().unwrap();
    let id = state.operations().snapshot().operation.as_ref().unwrap().id;
    let revision = state.operations().snapshot().revision;
    abandon(&mut state, id, revision)
}
#[test]
fn recovery_preserves_a_reappeared_target_and_its_parked_original() {
    let f = Fixture::new(None);
    let live = inventory::native(f.folder(), EXE);
    let release = std::fs::read(&live).unwrap();
    std::fs::write(&live, b"original edit").unwrap();
    let mut preview = f.preview().unwrap();
    preview.replacements = vec![stored_replacement(&preview, EXE, &release)];
    let backup = inventory::native(preview.report.backup.as_ref().unwrap(), EXE);
    assert!(confirm_replacing(preview, |p| at(publication::Point::Replaced, p)).is_err());
    std::fs::write(&live, b"new player edit").unwrap();
    let f = f.reopen();
    assert_eq!(recover_now(&f.state), Err(Error::SourceChanged));
    assert_eq!(std::fs::read(&live).unwrap(), b"new player edit");
    assert_eq!(std::fs::read(&backup).unwrap(), b"original edit");
}
#[test]
fn recovery_preserves_a_missing_target_that_appeared_after_admission() {
    let f = Fixture::new(None);
    let live = f.folder().join("later.txt");
    let release = std::fs::read(&live).unwrap();
    std::fs::remove_file(&live).unwrap();
    let mut preview = f.preview().unwrap();
    let mut replacement = stored_replacement(&preview, "later.txt", &release);
    replacement.reason = Classification::Missing;
    preview.replacements = vec![replacement];
    assert!(confirm_replacing(preview, |p| at(publication::Point::Plan, p)).is_err());
    std::fs::write(&live, b"new player file").unwrap();
    let f = f.reopen();
    assert_eq!(recover_now(&f.state), Err(Error::SourceChanged));
    assert_eq!(std::fs::read(&live).unwrap(), b"new player file");
}
fn link_directory(path: &Path, target: &Path) {
    #[cfg(windows)]
    assert!(std::process::Command::new("pwsh").args(["-NoProfile", "-Command", "New-Item -ItemType Junction -Path $env:CIMMERIA_LINK_PATH -Target $env:CIMMERIA_LINK_TARGET -ErrorAction Stop | Out-Null"])
        .env("CIMMERIA_LINK_PATH", path).env("CIMMERIA_LINK_TARGET", target).status().unwrap().success());
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, path).unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recover_and_abandon_refuse_nested_links_before_any_outside_write() {
    for point in [publication::Point::Plan, publication::Point::SetUp] {
        for abandoning in [false, true] {
            let f = Light::new(vec![], &[]).await;
            let preview = f.preview().unwrap();
            assert!(confirm_replacing(preview, |p| at(point, p)).is_err());
            let binaries = f.folder().join("Working/Binaries");
            std::fs::rename(&binaries, f.folder().join("retained-binaries")).unwrap();
            let outside = tempfile::tempdir().unwrap();
            let sentinel = outside.path().join("SGW.exe");
            std::fs::write(&sentinel, b"outside player executable").unwrap();
            let before = inventory::identity(&sentinel).unwrap();
            link_directory(&binaries, outside.path());
            let f = f.reopen();
            let result = if abandoning {
                abandon_now(&f.state)
            } else {
                recover_now(&f.state)
            };
            assert!(result.is_err(), "{point:?} abandoning={abandoning}");
            assert_eq!(
                std::fs::read(&sentinel).unwrap(),
                b"outside player executable"
            );
            assert_eq!(inventory::identity(&sentinel).unwrap(), before);
            assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 1);
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abandon_resumes_after_each_restored_file_and_metadata_deletion() {
    use super::direct_abandon::Point;
    for (fault, occurrence) in [
        (Point::File, 1),
        (Point::File, 2),
        (Point::Setup, 1),
        (Point::Manifest, 1),
        (Point::Expected, 1),
        (Point::Work, 1),
        (Point::Identity, 1),
        (Point::Receipt, 1),
        (Point::Owner, 1),
    ] {
        let f = Light::new(vec![], &[("later.txt", b"original later")]).await;
        let exe_path = inventory::native(f.folder(), EXE);
        let mut original_exe = std::fs::read(&exe_path).unwrap();
        original_exe[100] = 10;
        original_exe[0x186] |= 0x40;
        std::fs::write(&exe_path, &original_exe).unwrap();
        std::fs::write(f.folder().join("later.txt"), b"original later").unwrap();
        let mut preview = f.preview().unwrap();
        let mut patched_exe = original_exe.clone();
        patched_exe[100] = 11;
        preview.replacements = vec![
            stored_replacement(&preview, "later.txt", b"new later"),
            stored_replacement(&preview, EXE, &patched_exe),
        ];
        // An overlay result has a separate parked original, including setup's executable.
        for row in &mut preview.report.files {
            if row.path == EXE || row.path == "later.txt" {
                row.classification = Classification::Modified;
            }
        }
        if preview.report.backup.is_none() {
            preview.report.backup = Some(f.folder().join(format!(
                "cimmeria-adoption-backup-{}",
                preview.report.preview_handle
            )));
        }
        assert!(confirm_replacing(preview, |p| at(publication::Point::SetUp, p)).is_err());
        let f = f.reopen();
        let (id, _) = f.operation();
        let mut state = f.state.lock().unwrap();
        let record = inspect(&state, id).unwrap();
        let mut seen = 0;
        let result = super::direct_abandon::abandon_with(
            &mut state,
            &record.plan,
            record.phase,
            &mut |point| {
                if point == fault {
                    seen += 1;
                    if seen == occurrence {
                        return Err(StorageError::Io.into());
                    }
                }
                Ok(())
            },
        );
        assert!(result.is_err(), "fault {fault:?}/{occurrence} reached");
        drop(state);
        let f = f.reopen();
        assert!(
            interrupted(&f.state.lock().unwrap(), id)
                .unwrap()
                .abandonable
        );
        abandon_now(&f.state).unwrap();
        assert_eq!(f.operation().1, OperationState::Cancelled);
        assert_eq!(
            std::fs::read(&exe_path).unwrap(),
            original_exe,
            "{fault:?}/{occurrence}"
        );
        assert_eq!(
            std::fs::read(f.folder().join("later.txt")).unwrap(),
            b"original later"
        );
        assert!(!f.folder().join(".cimmeria-install.json").exists());
        assert!(!f.folder().join("content-ready.json").exists());
    }
}
