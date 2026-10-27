use super::*;

#[cfg(unix)]
#[test]
fn maps_an_absolute_path_with_a_space_to_a_z_drive_path() {
    assert_eq!(
        guest(Path::new("/home/deck/a b/x.exe")).unwrap(),
        "Z:\\home\\deck\\a b\\x.exe"
    );
}

#[test]
fn refuses_relative_paths() {
    assert!(guest(Path::new("games/x.exe")).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_windows_invalid_names() {
    assert!(guest(Path::new("/home/deck/CON")).is_err());
    assert!(guest(Path::new("/home/deck/lpt1.txt")).is_err());
    assert!(guest(Path::new("/home/deck/a:b")).is_err());
    assert!(guest(Path::new("/home/deck/trailing.")).is_err());
}

#[cfg(unix)]
#[test]
fn refuses_non_utf8_paths() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let path = Path::new(OsStr::from_bytes(b"/home/deck/\xff/x.exe"));
    assert!(guest(path).is_err());
}
