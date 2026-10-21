//! Parity between launcher-core's per-file index builder and the adoption
//! reference. Runs on every OS: `reference_tests` is macOS-gated, so the zip,
//! signing and PE helpers are repeated here rather than imported.
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::PathBuf;

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
/// A minimal PE32 image: enough headers for `client_setup::aslr` to patch.
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
fn file_index_build_matches_adoption_reference_raw_index() {
    let dir = tempfile::tempdir().unwrap();
    let exe = minimal_pe();
    // A bundled cooked-data PAK (moved to SourceCache.en-us by both builders)
    // and a lower-cased EULA script (the builder restores its stock name).
    let seed = archive(&[
        ("Working/Binaries/SGW.exe", exe.as_slice()),
        ("later.txt", b"later entry"),
        (
            "Working/SGWGame/Cache.en-US/CookedInteractionSet.pak",
            b"bundled cooked data",
        ),
        ("Working/SGWGame/Content/UI/Startup/EULA/eula.lua", b"eula"),
    ]);
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
    let paths: Vec<PathBuf> = [&seed, &first, &second, &third]
        .iter()
        .enumerate()
        .map(|(i, bytes)| {
            let p = dir.path().join(format!("{i}.zip"));
            std::fs::write(&p, bytes).unwrap();
            p
        })
        .collect();
    let manifest = release.manifest();
    let patches: Vec<(crate::manifest::PatchEntry, PathBuf)> = manifest
        .patches
        .iter()
        .cloned()
        .zip(paths[1..].iter().cloned())
        .collect();
    let sink = crate::unpack::UnpackSink {
        progress: crate::install_progress::ProgressSink::latest().0,
        label: "File index parity".into(),
        cancel: CancellationToken::new(),
    };
    let index = cimmeria_launcher_core::file_index::build(
        &paths[0],
        &patches,
        manifest,
        &dir.path().join("index"),
        &sink,
    )
    .unwrap();
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
        None,
    )
    .unwrap();
    // The final state of each path: the seed, then each patch's outputs in
    // order. Paths compare lower-cased: the reference keeps the seed's
    // spelling, the builder restores stock case. The stock-case rename itself
    // is pinned by `build_tests`, which compares exact spellings.
    let mut expected: BTreeMap<String, String> = index
        .seed
        .files
        .iter()
        .map(|file| (file.path.to_ascii_lowercase(), file.sha256.clone()))
        .collect();
    for layer in &index.patches {
        for file in &layer.outputs {
            expected.insert(file.path.to_ascii_lowercase(), file.sha256.clone());
        }
    }
    let raw: BTreeMap<String, String> = result
        .raw
        .iter()
        .filter(|(path, _)| path.as_str() != "launcher-installed.json")
        .map(|(path, entry)| (path.to_ascii_lowercase(), hex(&entry.sha256)))
        .collect();
    assert_eq!(expected, raw);
    assert!(expected.contains_key("working/sgwgame/sourcecache.en-us/cookedinteractionset.pak"));

    // The ASLR setup rule hashes the SGW.exe that `client_setup` prepares:
    // the same bytes with ASLR disabled.
    let aslr_copy = dir.path().join("aslr-check.exe");
    std::fs::write(&aslr_copy, &exe).unwrap();
    crate::client_setup::aslr::disable(&aslr_copy).unwrap();
    let prepared = std::fs::read(&aslr_copy).unwrap();
    let [cimmeria_launcher_core::file_index::SetupRule::AslrOff { sha256, .. }, ..] =
        index.setup.as_slice()
    else {
        panic!("{:?}", index.setup);
    };
    assert_eq!(*sha256, hex(&Sha256::digest(&prepared)));
    assert_ne!(*sha256, hex(&Sha256::digest(&exe)));
}
#[test]
fn reference_records_zip_invalid_only_for_zip_failures() {
    let seed = crate::install_worker::fixtures::archive(true);
    let patch = archive(&[("later.txt", b"patch one")]);
    let bad = archive(&[("../escape", b"escape")]);
    let servers = crate::client_setup::login_servers::default_servers();
    let attempt = |seed_bytes: &[u8], release: &VerifiedRelease, patches: Vec<PathBuf>| {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seed.zip");
        std::fs::write(&path, seed_bytes).unwrap();
        let artifacts = Artifacts {
            seed: path,
            patches,
        };
        rejection::scope(|| {
            reference::reconstruct(
                dir.path(),
                release,
                &artifacts,
                &servers,
                &CancellationToken::new(),
                crate::install_progress::ProgressSink::latest().0,
                None,
            )
        })
    };
    // A patch-count mismatch is not a zip failure: nothing is recorded.
    let (result, record) = attempt(&seed, &signed(&seed, &[(&patch, "install_dir")]), vec![]);
    assert!(matches!(result, Err(Error::InvalidArtifact)));
    assert!(record.is_none());
    // A seed that fails the authenticated size and sha256 check is not a zip failure either.
    let (result, record) = attempt(
        &archive(&[("other.txt", b"other")]),
        &signed(&seed, &[]),
        vec![],
    );
    assert!(matches!(result, Err(Error::InvalidArtifact)));
    assert!(record.is_none());
    // A zip that fails `strict_zip` is recorded at the seed.
    let (result, record) = attempt(&bad, &signed(&bad, &[]), vec![]);
    assert!(matches!(result, Err(Error::InvalidArtifact)));
    let record = record.unwrap();
    assert_eq!(record.check, "reference_zip_invalid");
    assert_eq!(record.entry.as_deref(), Some("reference"));
}
