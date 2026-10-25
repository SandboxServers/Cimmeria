use super::dll_search_directory;

fn plain(input: &str) -> String {
    dll_search_directory(input).into_owned()
}

#[test]
fn verbatim_drive_path_becomes_plain() {
    // Fails if the function returns its input (the F-24 bug).
    assert_eq!(
        plain(r"\\?\Z:\home\deck\Stargate Worlds\game\Working\binaries"),
        r"Z:\home\deck\Stargate Worlds\game\Working\binaries"
    );
}

#[test]
fn bare_drive_root_becomes_plain() {
    assert_eq!(plain(r"\\?\C:\"), r"C:\");
}

#[test]
fn other_forms_pass_through_unchanged() {
    for input in [
        r"C:\Games\SGW",
        r"\\?\UNC\server\share\SGW",
        r"\\?\Volume{0b1e}\SGW",
        r"\\?\C:\a\b.",
        r"\\?\C:\a\b ",
        r"\\?\C:\a\..\b",
        r"\\?\C:\a\.\b",
        r"\\?\C:\a\\b",
        r"\\?\C:\a\",
        r"\\?\C:/a",
        r"\\?\C:",
        r"\\?\1:\a",
    ] {
        assert_eq!(plain(input), input, "{input}");
    }
}

#[test]
fn length_limit_is_260_units() {
    // Fails if the length check is dropped.
    let build = |units: usize| {
        let tail = "a".repeat(units - 3);
        format!(r"C:\{tail}")
    };
    let short = build(259);
    let long = build(260);
    assert_eq!(plain(&format!(r"\\?\{short}")), short);
    let verbatim_long = format!(r"\\?\{long}");
    assert_eq!(plain(&verbatim_long), verbatim_long);
}
