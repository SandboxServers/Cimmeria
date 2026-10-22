use super::*;
use std::path::PathBuf;

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

#[test]
fn entries_scrub_home_and_ips_and_keep_three_newest() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_string_lossy().into_owned();
    let redaction = Redaction {
        home: Some(home.clone()),
        user: None,
    };
    for day in ["18", "19", "20"] {
        write(
            dir.path(),
            &format!("launcher.2026-10-{day}.log"),
            "older\n",
        );
    }
    let newest = format!("{home}\\x\\launcher.toml read\nconnect to 198.51.100.7:8081 failed\n");
    write(dir.path(), "launcher.2026-10-21.log", &newest);

    let got = entries(dir.path(), &redaction, LAUNCHER_LOG_CAP);
    let names: Vec<&str> = got.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "launcher/launcher.2026-10-19.log",
            "launcher/launcher.2026-10-20.log",
            "launcher/launcher.2026-10-21.log",
        ]
    );
    let last = String::from_utf8(got[2].1.clone()).unwrap();
    assert!(last.contains("[ip]:8081"), "{last}");
    assert!(!last.contains("198.51.100.7"), "{last}");
    assert!(!last.contains(&home), "{last}");
    // The local file is never changed.
    let local = std::fs::read_to_string(dir.path().join("launcher.2026-10-21.log")).unwrap();
    assert_eq!(local, newest);
}

#[test]
fn entries_drop_the_oldest_lines_over_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let ten: String = (0..10).map(|i| format!("line {i:02}\n")).collect();
    write(dir.path(), "launcher.2026-10-20.log", &ten);
    write(dir.path(), "launcher.2026-10-21.log", &ten);
    // Each line is 8 bytes with its newline: 15 lines fit, 20 do not.
    let cap = 15 * 8;

    let got = entries(dir.path(), &Redaction::default(), cap);
    assert_eq!(got.len(), 2);
    let oldest = String::from_utf8(got[0].1.clone()).unwrap();
    assert_eq!(oldest, "line 05\nline 06\nline 07\nline 08\nline 09\n");
    assert_eq!(got[1].1, ten.as_bytes());
}

#[test]
fn append_adds_launcher_entries_to_the_game_zip() {
    let mut game = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    game.start_file("Binaries/a.log", zip::write::SimpleFileOptions::default())
        .unwrap();
    game.write_all(b"game\n").unwrap();
    let body = game.finish().unwrap().into_inner();
    let added = vec![(
        "launcher/launcher.2026-10-21.log".to_string(),
        b"hello\n".to_vec(),
    )];

    let out = append(body, &added).unwrap();
    let archive = zip::ZipArchive::new(std::io::Cursor::new(out)).unwrap();
    let mut names: Vec<String> = archive
        .file_names()
        .filter_map(|name| name.ok().map(|name| name.to_string()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["Binaries/a.log", "launcher/launcher.2026-10-21.log"]
    );
}

#[test]
fn entries_without_a_folder_are_empty() {
    let dir = tempfile::tempdir().unwrap();
    let missing: PathBuf = dir.path().join("logs");
    assert!(entries(&missing, &Redaction::default(), LAUNCHER_LOG_CAP).is_empty());
}
