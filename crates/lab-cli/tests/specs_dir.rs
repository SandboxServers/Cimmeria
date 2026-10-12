//! F-LX7: a relative `--specs-dir` is made absolute before any daemon call.

use std::path::{Path, PathBuf};

use cimmeria_lab_cli::daemon::paths::{absolute_in, absolute_specs_dir};

#[test]
fn a_relative_dir_is_joined_to_the_given_directory() {
    let cwd = Path::new("work");
    assert_eq!(
        PathBuf::from(absolute_in("uat-specs", cwd)),
        cwd.join("uat-specs")
    );
}

#[test]
fn rooted_paths_are_kept_on_every_platform() {
    for dir in [
        r"D:\specs",
        "D:/specs",
        r"\\server\share\specs",
        r"C:\a b\c",
    ] {
        assert_eq!(absolute_in(dir, Path::new("work")), dir);
    }
}

#[test]
fn an_empty_dir_stays_empty() {
    assert_eq!(absolute_in("", Path::new("work")), "");
}

#[test]
fn the_process_form_uses_the_current_directory() {
    let want = std::env::current_dir()
        .expect("cwd")
        .join("uat-specs")
        .display()
        .to_string();
    assert_eq!(absolute_specs_dir("uat-specs"), want);
}
