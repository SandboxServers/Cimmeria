use super::*;
use std::time::{Duration, SystemTime};

fn age(path: &Path, seconds_ago: u64) {
    let when = SystemTime::now() - Duration::from_secs(seconds_ago);
    std::fs::File::open(path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(when))
        .unwrap();
}

#[test]
fn folder_keeps_the_new_operation_and_the_four_newest_older_ones() {
    let root = tempfile::tempdir().unwrap();
    let games = root.path().join("logs/game");
    let mut older = Vec::new();
    for hours in 1..=7u64 {
        let path = games.join(Uuid::new_v4().to_string());
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("wine.log"), b"old").unwrap();
        age(&path, hours * 3600);
        older.push(path);
    }
    let operation = Uuid::new_v4();
    let created = folder(root.path(), operation).unwrap();
    assert_eq!(created, games.join(operation.to_string()));
    assert!(created.is_dir());
    let mut left: Vec<PathBuf> = std::fs::read_dir(&games)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    left.sort();
    let mut expected: Vec<PathBuf> = older[..4].to_vec();
    expected.push(created);
    expected.sort();
    assert_eq!(left, expected);
}

#[test]
fn folder_keeps_the_new_operation_even_when_older_folders_look_newer() {
    let root = tempfile::tempdir().unwrap();
    let games = root.path().join("logs/game");
    let operation = Uuid::new_v4();
    let first = folder(root.path(), operation).unwrap();
    age(&first, 7200);
    for _ in 0..6 {
        std::fs::create_dir_all(games.join(Uuid::new_v4().to_string())).unwrap();
    }
    folder(root.path(), operation).unwrap();
    assert!(games.join(operation.to_string()).is_dir());
    assert_eq!(std::fs::read_dir(&games).unwrap().count(), KEEP);
}
