use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::io::{Cursor, Write};

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn signed(seed: &[u8], patches: &[(&[u8], &str)]) -> VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":seed.len(),"sha256":hex(&Sha256::digest(seed))},"patches": patches.iter().enumerate().map(|(i,(p,root))| serde_json::json!({"id":format!("p{i}"),"blob":format!("p{i}.zip"),"size":p.len(),"sha256":hex(&Sha256::digest(p)),"root":root})).collect::<Vec<_>>() })).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    crate::catalog::verify_release(&body, hex(&signature.to_bytes()).as_bytes()).unwrap()
}
#[test]
fn reference_reconstructs_signed_patch_order_and_sgw_game_root() {
    let dir = tempfile::tempdir().unwrap();
    let seed = crate::install_worker::fixtures::archive(true);
    let first = archive(&[
        ("later.txt", b"patch one"),
        ("Working/SGWGame/anchor", b"game directory"),
    ]);
    let second = archive(&[("later.txt", b"patch two")]);
    let third = archive(&[("Content/fixture.lua", b"rooted lua")]);
    let release = signed(
        &seed,
        &[
            (&first, "install_dir"),
            (&second, "install_dir"),
            (&third, "sgw_game"),
        ],
    );
    let paths: Vec<_> = [&seed, &first, &second, &third]
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let p = dir.path().join(format!("{i}.zip"));
            std::fs::write(&p, bytes).unwrap();
            p
        })
        .collect();
    let result = reference::reconstruct(
        dir.path(),
        &release,
        &Artifacts {
            seed: paths[0].clone(),
            patches: paths[1..].to_vec(),
        },
        &crate::client_setup::login_servers::default_servers(),
        &CancellationToken::new(),
        crate::install_progress::ProgressSink::latest().0,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(result.prepared.join("later.txt")).unwrap(),
        b"patch two"
    );
    assert_eq!(
        std::fs::read(result.prepared.join("Working/SGWGame/Content/fixture.lua")).unwrap(),
        b"rooted lua"
    );
    assert!(!result.prepared.join("Content/fixture.lua").exists());
    let mut wrong_order = paths[1..].to_vec();
    wrong_order.swap(0, 1);
    assert!(matches!(
        reference::reconstruct(
            dir.path(),
            &release,
            &Artifacts {
                seed: paths[0].clone(),
                patches: wrong_order
            },
            &crate::client_setup::login_servers::default_servers(),
            &CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0
        ),
        Err(Error::InvalidArtifact)
    ));
}
#[test]
fn authenticated_case_collisions_and_unsafe_zip_names_are_rejected() {
    for entries in [
        vec![
            ("same", b"first".as_slice()),
            ("SAME", b"second".as_slice()),
        ],
        vec![("../escape", b"escape".as_slice())],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let seed = archive(&entries);
        let release = signed(&seed, &[]);
        let path = dir.path().join("seed.zip");
        std::fs::write(&path, &seed).unwrap();
        assert!(matches!(
            reference::reconstruct(
                dir.path(),
                &release,
                &Artifacts {
                    seed: path,
                    patches: vec![]
                },
                &crate::client_setup::login_servers::default_servers(),
                &CancellationToken::new(),
                crate::install_progress::ProgressSink::latest().0
            ),
            Err(Error::InvalidArtifact)
        ));
        assert!(!dir.path().join("escape").exists());
    }
}
#[test]
fn missing_executable_is_reviewed_and_replaced_from_reference() {
    let f = tests::Fixture::new();
    let imported = f.state.lock().unwrap().legacy_import().unwrap().unwrap();
    let exe = crate::install_layout::sgw_exe(&imported.source.game_directory);
    std::fs::remove_file(&exe).unwrap();
    let preview = f.preview().unwrap();
    assert!(preview
        .report
        .files
        .iter()
        .any(|d| d.path.ends_with("SGW.exe") && d.classification == Classification::Missing));
    let handle = preview.report.preview_handle;
    confirm(
        preview,
        Uuid::new_v4(),
        handle,
        tests::choices(),
        CancellationToken::new(),
    )
    .unwrap();
    assert!(!exe.exists());
    assert!(f
        .destination()
        .join("game/Working/Binaries/SGW.exe")
        .is_file());
}
