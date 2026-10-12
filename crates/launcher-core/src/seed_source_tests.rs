use super::*;
use crate::install::{install_all, InstallContext};
use crate::manifest::Manifest;
use crate::unpack::test_fixtures::write_stored_rar4;
use std::time::Duration;
use wiremock::matchers::{method, path as url_path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Keeps every progress event a check or copy reports.
#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<Progress>>,
}

impl ProgressReporter for Recorder {
    fn report(&self, value: Progress) {
        self.events.lock().unwrap().push(value);
    }
}

fn recording_sink() -> (ProgressSink, Arc<Recorder>) {
    let recorder = Arc::new(Recorder::default());
    (ProgressSink::Checkpoint(recorder.clone()), recorder)
}

fn sha_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(hasher)
}

fn seed_for(bytes: &[u8]) -> SeedEntry {
    SeedEntry {
        blob: "seed/x.rar".into(),
        size: bytes.len() as u64,
        sha256: sha_of(bytes),
    }
}

fn manifest_for(seed: SeedEntry) -> Manifest {
    Manifest {
        schema: 1,
        min_launcher: None,
        seed,
        patches: vec![],
        file_index: None,
    }
}

/// Lists the names left in `dir`, so a test can see what was written.
fn names_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

/// A minimal PE32 image with the ASLR flag set, so the install's PE patch has
/// a real header to edit.
fn minimal_pe() -> Vec<u8> {
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    exe
}

#[test]
fn a_wrong_size_file_is_refused_without_reading_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"").unwrap();
    let seed = SeedEntry {
        blob: "seed/x.rar".into(),
        size: 1_000_000_000,
        sha256: "00".repeat(32),
    };
    let (sink, recorder) = recording_sink();
    let err = verify_local_seed(&path, &seed, &CancellationToken::new(), &sink).unwrap_err();
    assert_eq!(
        err,
        LocalSeedError::SizeMismatch {
            expected: 1_000_000_000,
            actual: 0,
        }
    );
    assert!(recorder.events.lock().unwrap().is_empty());
}

#[test]
fn a_right_size_wrong_content_file_is_refused_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"12345678").unwrap();
    let seed = seed_for(b"abcdefgh");
    let (sink, _recorder) = recording_sink();
    let err = verify_local_seed(&path, &seed, &CancellationToken::new(), &sink).unwrap_err();
    assert_eq!(err, LocalSeedError::HashMismatch);
    assert_eq!(std::fs::read(&path).unwrap(), b"12345678");
    assert_eq!(names_in(dir.path()), ["player.rar"]);
}

#[test]
fn a_file_replaced_after_the_check_is_refused_by_copy_into() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"aaaa").unwrap();
    let (sink, _recorder) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap();
    // Same size, different bytes: the pick-time hash must not be trusted.
    std::fs::write(&path, b"bbbb").unwrap();
    let dest = dir.path().join(".tmp-seed-test.download");
    let err = verified
        .copy_into(&dest, &CancellationToken::new(), &sink)
        .unwrap_err();
    assert!(
        matches!(err, LocalSeedError::Changed | LocalSeedError::HashMismatch),
        "{err:?}"
    );
    assert!(!dest.exists());
    assert!(!local_copy_path(&dest).exists());
    assert_eq!(std::fs::read(&path).unwrap(), b"bbbb");
}

#[test]
fn copy_into_writes_the_dest_and_keeps_the_players_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"aaaa").unwrap();
    let (sink, _recorder) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap();
    let dest = dir.path().join(".tmp-seed-test.download");
    verified
        .copy_into(&dest, &CancellationToken::new(), &sink)
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"aaaa");
    assert_eq!(std::fs::read(&path).unwrap(), b"aaaa");
    assert!(!local_copy_path(&dest).exists());
}

#[cfg(windows)]
#[test]
fn a_symlink_to_a_valid_archive_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.rar");
    std::fs::write(&target, b"aaaa").unwrap();
    let link = dir.path().join("link.rar");
    if let Err(error) = std::os::windows::fs::symlink_file(&target, &link) {
        println!("skipped: cannot create a symlink here ({error})");
        return;
    }
    let (sink, _recorder) = recording_sink();
    let err =
        verify_local_seed(&link, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap_err();
    assert_eq!(err, LocalSeedError::NotAFile);
}

/// An install with a verified local archive and a server that fails every
/// seed request: the archive is used and the player's file stays in place.
#[tokio::test]
async fn install_all_uses_a_local_seed_without_downloading() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(url_path("/seed/x.rar"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let player = root.path().join("player.rar");
    let exe = minimal_pe();
    write_stored_rar4(&player, &[("SGW.exe", exe.as_slice())]);
    let bytes = std::fs::read(&player).unwrap();
    let manifest = manifest_for(seed_for(&bytes));
    let (sink, _recorder) = recording_sink();
    let verified = verify_local_seed(&player, &manifest.seed, &CancellationToken::new(), &sink)
        .expect("the fixture matches its own seed");
    let install_dir = root.path().join("game");
    let http = reqwest::Client::new();
    let servers = crate::client_setup::login_servers::default_servers();
    let seed_source = SeedSourceHandle::local(verified);
    let (result, _report) = install_all(InstallContext {
        manifest_url: &format!("{}/manifest.json", server.uri()),
        install_dir: &install_dir,
        manifest: &manifest,
        login_servers: &servers,
        cancel: CancellationToken::new(),
        progress: ProgressSink::latest().0,
        http: &http,
        seed_source: &seed_source,
    })
    .await;
    result.expect("the local archive installs");
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(std::fs::read(&player).unwrap(), bytes);
    assert_installed_pe(&install_dir.join("SGW.exe"), &exe);
}

#[test]
fn clear_returns_to_download_and_only_later_steps_see_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"aaaa").unwrap();
    let (sink, _recorder) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap();
    let handle = SeedSourceHandle::download();
    handle.use_local(verified.clone());
    // A running step fired by the switch, and one that already read current().
    let running = handle.switched();
    let read_before = handle.current();
    assert!(running.is_cancelled());
    handle.clear();
    assert_eq!(read_before, Some(verified));
    assert!(handle.current().is_none());
    assert!(!handle.switched().is_cancelled());
}

#[test]
fn a_download_running_across_clear_still_sees_a_later_switch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"aaaa").unwrap();
    let (sink, _recorder) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap();
    let handle = SeedSourceHandle::download();
    // The download started before the clear holds the token it was given.
    let running = handle.switched();
    handle.clear();
    assert!(!running.is_cancelled());
    handle.use_local(verified.clone());
    assert!(running.is_cancelled());
    assert_eq!(handle.current(), Some(verified));
}

#[test]
fn current_for_ignores_an_archive_for_another_seed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    std::fs::write(&path, b"aaaa").unwrap();
    let (sink, _recorder) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(b"aaaa"), &CancellationToken::new(), &sink).unwrap();
    let handle = SeedSourceHandle::local(verified.clone());
    assert_eq!(handle.current_for(&verified.sha256), Some(verified.clone()));
    assert_eq!(
        handle.current_for(&verified.sha256.to_ascii_uppercase()),
        Some(verified.clone())
    );
    assert!(handle.current_for(&sha_of(b"bbbb")).is_none());
    // A resumed download for another seed is re-armed and keeps going.
    assert!(handle.resume_for(&sha_of(b"bbbb")).is_none());
    assert!(!handle.switched().is_cancelled());
}

/// Setup clears the ASLR bit (0x40 at 0x186) in the installed image; every
/// other byte must match the packed one.
fn assert_installed_pe(installed: &Path, packed: &[u8]) {
    let got = std::fs::read(installed).unwrap();
    assert_eq!(got.len(), packed.len());
    assert_eq!(got[..0x186], packed[..0x186]);
    assert_eq!(got[0x188..], packed[0x188..]);
}

/// A seed download slowed by a responder delay is switched to a local
/// archive part-way: the install succeeds, no partial file is left, and the
/// server sees no request after the switch.
#[tokio::test]
async fn switching_a_running_download_to_a_local_seed_completes_the_install() {
    let root = tempfile::tempdir().unwrap();
    let player = root.path().join("player.rar");
    let exe = minimal_pe();
    write_stored_rar4(&player, &[("SGW.exe", exe.as_slice())]);
    let bytes = std::fs::read(&player).unwrap();
    let manifest = manifest_for(seed_for(&bytes));
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(url_path("/seed/x.rar"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(bytes.clone())
                .set_delay(Duration::from_secs(30)),
        )
        .mount(&server)
        .await;
    let (sink, _recorder) = recording_sink();
    let verified = verify_local_seed(&player, &manifest.seed, &CancellationToken::new(), &sink)
        .expect("the fixture matches its own seed");
    let install_dir = root.path().join("game");
    let http = reqwest::Client::new();
    let servers = crate::client_setup::login_servers::default_servers();
    let seed_source = SeedSourceHandle::download();
    let manifest_url = format!("{}/manifest.json", server.uri());
    let install = install_all(InstallContext {
        manifest_url: &manifest_url,
        install_dir: &install_dir,
        manifest: &manifest,
        login_servers: &servers,
        cancel: CancellationToken::new(),
        progress: ProgressSink::latest().0,
        http: &http,
        seed_source: &seed_source,
    });
    // Switch once the download request has reached the server.
    let switch = async {
        while server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        seed_source.use_local(verified);
    };
    let (result, _report) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(install, switch).0
    })
    .await
    .expect("the switch stops the slow download");
    result.expect("the local archive installs after the switch");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(names_in(&install_dir)
        .iter()
        .all(|name| !name.starts_with(".tmp-seed-")));
    assert_eq!(std::fs::read(&player).unwrap(), bytes);
    assert_installed_pe(&install_dir.join("SGW.exe"), &exe);
}
