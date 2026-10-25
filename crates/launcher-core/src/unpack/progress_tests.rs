use std::path::Path;

use super::CABINET_STEP;
use crate::install::Progress;
use crate::unpack::cab_portable::fixture::{write_cab_set, CabOptions};
use crate::unpack::test_fixtures::{incompressible, sink, write_stored_rar4};
use crate::unpack::unpack;

#[test]
fn only_the_cabinet_step_reports_expanding() {
    let (sink, mut rx) = sink();
    sink.report(CABINET_STEP, 1, 2, Path::new("a"));
    sink.report("unpacking RAR", 1, 2, Path::new("b"));
    assert!(matches!(rx.try_recv().unwrap(), Progress::Expanding { .. }));
    assert!(matches!(
        rx.try_recv().unwrap(),
        Progress::Extracting { .. }
    ));
}

// The installer RAR: its own files are `Extracting`, the cabinet files
// `Expanding`. Fails if the cabinet code stops passing CABINET_STEP.
#[test]
fn a_cabinet_seed_reports_both_phases() {
    let dir = tempfile::tempdir().unwrap();
    let cab_dir = dir.path().join("cabs");
    std::fs::create_dir_all(&cab_dir).unwrap();
    let sgw = incompressible(7, 150_000);
    let ini: Vec<u8> = b"[Core.System]\r\nPaths=..\\SGWGame\\CookedPC\r\n"
        .iter()
        .copied()
        .cycle()
        .take(40_000)
        .collect();
    let files = vec![
        ("Working\\binaries\\SGW.exe", sgw.clone()),
        ("Working\\SGWGame\\Config\\DefaultEngine.ini", ini.clone()),
        ("Common\\res\\a.def", b"<def/>".to_vec()),
    ];
    let set = write_cab_set(&cab_dir, &files, CabOptions::default());
    assert!(set.cabinets.len() >= 2, "{:?}", set.cabinets);

    let mut entries: Vec<(String, Vec<u8>)> = vec![
        ("SetupQA.exe".into(), b"MZ".to_vec()),
        (
            "Data/Prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe".into(),
            b"inert vendor fixture".to_vec(),
        ),
    ];
    for name in set.cabinets.iter().map(String::as_str).chain(["DATA.INF"]) {
        entries.push((
            format!("Data\\{name}"),
            std::fs::read(cab_dir.join(name)).unwrap(),
        ));
    }
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let archive = dir.path().join("client.rar");
    write_stored_rar4(&archive, &refs);

    let dest = dir.path().join("install");
    let (sink, mut rx) = sink();
    unpack(&archive, &dest, &sink).unwrap();
    let (mut archive_files, mut cabinet_files) = (0, 0);
    while let Ok(event) = rx.try_recv() {
        match event {
            Progress::Extracting { .. } => archive_files += 1,
            Progress::Expanding { .. } => cabinet_files += 1,
            _ => {}
        }
    }
    assert!(archive_files > 0, "no RAR entry was reported");
    assert_eq!(cabinet_files, 3, "one Expanding per cabinet file");
}

#[test]
fn the_rar_listing_pass_reports_checking_without_a_total() {
    let dir = tempfile::tempdir().unwrap();
    let names: Vec<String> = (0..256).map(|i| format!("f{i}.txt")).collect();
    let entries: Vec<(&str, &[u8])> = names
        .iter()
        .map(|name| (name.as_str(), b"x".as_slice()))
        .collect();
    let archive = dir.path().join("many.rar");
    write_stored_rar4(&archive, &entries);
    let (sink, mut rx) = sink();
    unpack(&archive, &dir.path().join("out"), &sink).unwrap();
    let mut index = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let Progress::Checking { read, total } = event {
            index.push((read, total));
        }
    }
    assert_eq!(index, vec![(256, 0)]);
}
