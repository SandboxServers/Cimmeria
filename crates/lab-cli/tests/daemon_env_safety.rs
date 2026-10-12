//! labd.env masking of PASS/PWD keys and `KEY= value`, and the writer's refusals:
//! a file that is not UTF-8 is never rewritten, and a failed rename leaves no
//! `.tmp` behind.

use cimmeria_lab_cli::daemon::env_file::{self, EnvLine};
use tempfile::TempDir;

#[test]
fn keys_with_pass_or_pwd_are_masked() {
    for key in [
        "DB_PASS",
        "PASSPHRASE",
        "db_pwd",
        "PWD",
        "ADMIN_PASSWD",
        "X_PASSWORD",
    ] {
        assert_eq!(env_file::mask(key, "hunter2"), "<redacted>", "{key}");
        let line = env_file::parse_line(&format!("{key}=hunter2"));
        assert_eq!(
            env_file::mask_line(&line),
            format!("{key}=<redacted>"),
            "{key}"
        );
        assert_eq!(
            env_file::hide_text(&format!("{key}=hunter2")),
            format!("{key}=<redacted>"),
            "{key} in free text"
        );
    }
}

#[test]
fn a_key_that_is_not_secret_keeps_its_value() {
    assert_eq!(env_file::mask("CIMMERIA_LAB_PORT", "8779"), "8779");
}

#[test]
fn a_space_after_the_equals_sign_does_not_leak_the_value() {
    assert_eq!(
        env_file::hide_text("API_KEY= hunter2"),
        "API_KEY=<redacted>"
    );
    assert_eq!(
        env_file::hide_text("a DB_PASS =  hunter2 b"),
        "a DB_PASS =<redacted> b"
    );
    assert_eq!(env_file::hide_text("TOKEN=\tabc"), "TOKEN=<redacted>");
    // Only the same line: a bare `KEY=` does not eat the next line's word.
    assert_eq!(env_file::hide_text("KEY=\nnext"), "KEY=<redacted>\nnext");
    // A pair read from a file keeps the space in its value, and is masked by its key.
    let line = env_file::parse_line("API_KEY= hunter2");
    assert_eq!(env_file::mask_line(&line), "API_KEY=<redacted>");
}

fn lines(text: &str) -> Vec<EnvLine> {
    env_file::parse(text)
}

#[test]
fn a_file_that_is_not_utf8_is_not_rewritten() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("labd.env");
    let original: &[u8] = b"A=1\r\nNAME=caf\xe9\r\nB=2\r\n";
    std::fs::write(&path, original).unwrap();
    // The listing still reads it, lossily.
    let shown = env_file::read(&path).expect("read is lossy, not an error");
    assert!(shown
        .iter()
        .any(|l| matches!(l, EnvLine::Pair { key, .. } if key == "NAME")));

    let mut edited = lines("A=1\r\nB=3\r\n");
    env_file::set(&mut edited, "C", "9");
    let err = env_file::write_with_backup(&path, &edited).expect_err("must refuse");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("not valid UTF-8"), "{err}");
    assert_eq!(std::fs::read(&path).unwrap(), original, "file untouched");
    let others: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(others, ["labd.env"], "no backup, no .tmp");
}

#[test]
fn a_utf8_file_is_still_rewritten_and_a_missing_one_created() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("labd.env");
    assert_eq!(
        env_file::write_with_backup(&path, &lines("A=caf\u{e9}\r\n")).unwrap(),
        None
    );
    let backup = env_file::write_with_backup(&path, &lines("A=2\r\n")).unwrap();
    assert!(backup.is_some());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "A=2\r\n");
}

/// A read-only destination makes the rename fail on Windows (MoveFileEx will
/// not replace it); the half-written `labd.env.tmp` must not stay.
#[cfg(windows)]
#[test]
fn a_failed_rename_removes_the_tmp_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("labd.env");
    std::fs::write(&path, "A=1\r\n").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();

    let result = env_file::write_with_backup(&path, &lines("A=2\r\n"));

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).unwrap();
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let p = entry.unwrap().path();
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = std::fs::set_permissions(&p, perms);
    }

    assert!(
        result.is_err(),
        "the rename over a read-only file must fail"
    );
    assert!(
        !dir.path().join("labd.env.tmp").exists(),
        "the .tmp file was left behind"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "A=1\r\n");
}
