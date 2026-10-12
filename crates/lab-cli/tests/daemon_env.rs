//! labd.env parsing, editing, masking and backups. Ports the masking, key
//! rule, line rewrite and backup checks of the removed PowerShell test file
//! (the PowerShell check is named in each comment).

mod support;

use cimmeria_lab_cli::daemon::env_file::{self, EnvLine};
use regex::Regex;
use tempfile::TempDir;

fn hex() -> String {
    "ab".repeat(32)
}

fn masked(line: &str) -> String {
    env_file::mask_line(&env_file::parse_line(line))
}

fn lines(text: &str) -> Vec<EnvLine> {
    env_file::parse(text)
}

/// The lines joined with `|`, the way the PowerShell checks compare them.
fn joined(lines: &[EnvLine]) -> String {
    lines
        .iter()
        .map(env_file::render_line)
        .collect::<Vec<_>>()
        .join("|")
}

const SRC: &str = "# head\nFOO=1\nCIMMERIA_LAB_BAR=a=b\n# tail\n";

// ---- masking ----

// 'a TOKEN key is masked'
#[test]
fn mask_token_key() {
    assert_eq!(
        masked("CIMMERIA_LAB_DAEMON_TOKEN=s3cr3t"),
        "CIMMERIA_LAB_DAEMON_TOKEN=<redacted>"
    );
}

// 'a PASSWORD key is masked, spaces around = allowed'
#[test]
fn mask_password_key_with_spaces_around_equals() {
    assert_eq!(masked("MY_PASSWORD = x y"), "MY_PASSWORD=<redacted>");
}

// 'a URL host is shown as <host>, the port stays'
#[test]
fn mask_url_host_keeps_the_port() {
    assert_eq!(
        masked("CIMMERIA_LAB_BIND=http://127.0.0.1:8779/mcp"),
        "CIMMERIA_LAB_BIND=http://<host>:8779/mcp"
    );
}

// New (F-LX1): a bracketed IPv6 host is masked whole, the port stays.
#[test]
fn mask_ipv6_url_host_whole() {
    assert_eq!(
        masked("CIMMERIA_LAB_MCP_URL=http://[::1]:8779/x"),
        "CIMMERIA_LAB_MCP_URL=http://<host>:8779/x"
    );
    assert_eq!(
        masked("X=http://user:pw@[fe80::1]:80/p"),
        "X=http://<host>:80/p"
    );
}

// 'a comment with a TOKEN assignment is masked'
#[test]
fn mask_comment_with_a_token_assignment() {
    assert_eq!(masked("# note TOKEN=abc"), "# note TOKEN=<redacted>");
}

// 'a plain comment is shown as is'
#[test]
fn plain_comment_is_shown_as_is() {
    assert_eq!(masked("# keep this comment"), "# keep this comment");
}

// 'an ordinary value is shown as is'
#[test]
fn ordinary_value_is_shown_as_is() {
    assert_eq!(
        masked("CIMMERIA_LAB_INSTANCES=default,p2"),
        "CIMMERIA_LAB_INSTANCES=default,p2"
    );
}

// 'a 64-hex value is masked under any key'
#[test]
fn hex64_value_is_masked_under_any_key() {
    assert_eq!(
        masked(&format!("CIMMERIA_LAB_X={}", hex())),
        "CIMMERIA_LAB_X=<redacted>"
    );
}

// 'URL userinfo with a / in the password is masked with the host'
#[test]
fn url_userinfo_with_a_slash_is_masked_with_the_host() {
    assert_eq!(
        masked("U=http://user:p/ss@host.example/x"),
        "U=http://<host>/x"
    );
}

// 'URL userinfo is masked, the port stays'
#[test]
fn url_userinfo_is_masked_and_the_port_stays() {
    assert_eq!(
        masked("U=postgres://u:pw@db:5432/d"),
        "U=postgres://<host>:5432/d"
    );
}

// 'a KEY-named value is masked'
#[test]
fn key_named_value_is_masked() {
    assert_eq!(
        masked("API_KEY=0123456789abcdef0123456789abcdef01234567"),
        "API_KEY=<redacted>"
    );
}

// 'a line that is not KEY=VALUE is not shown'
#[test]
fn unparsed_line_is_not_shown() {
    assert_eq!(masked("pasted-secret-on-its-own-line"), "<unparsed line>");
}

// 'the token never appears in env output' (env.ps1 listing, at the library level)
#[test]
fn a_masked_listing_never_holds_the_token() {
    let file = lines("# head\nFOO=1\nCIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real\n# tail\n");
    let listing = file
        .iter()
        .map(env_file::mask_line)
        .collect::<Vec<_>>()
        .join("|");
    assert_eq!(
        listing,
        "# head|FOO=1|CIMMERIA_LAB_DAEMON_TOKEN=<redacted>|# tail"
    );
    assert!(!listing.contains("test-token-not-real"));
}

// New: a bearer token in a comment is masked.
#[test]
fn mask_bearer_text() {
    assert_eq!(
        env_file::hide_text("# curl -H 'bearer abc123def'"),
        "# curl -H '<redacted>"
    );
}

// ---- key rule ----

// 'an upper-case key with digits and _ is valid'
#[test]
fn key_with_upper_case_digits_and_underscore_is_valid() {
    assert!(env_file::is_valid_key("CIMMERIA_LAB_X_1"));
}

// 'a lower-case key is rejected'
#[test]
fn lower_case_key_is_rejected() {
    assert!(!env_file::is_valid_key("lower"));
}

// 'a key starting with a digit is rejected'
#[test]
fn key_starting_with_a_digit_is_rejected() {
    assert!(!env_file::is_valid_key("1ABC"));
}

// 'a key with a dash is rejected'
#[test]
fn key_with_a_dash_is_rejected() {
    assert!(!env_file::is_valid_key("A-B"));
}

// 'an empty key is rejected'
#[test]
fn empty_key_is_rejected() {
    assert!(!env_file::is_valid_key(""));
}

// ---- line rewrite ----

// 'set replaces the key in place'
#[test]
fn set_replaces_the_key_in_place() {
    let mut file = lines(SRC);
    env_file::set(&mut file, "FOO", "2");
    assert_eq!(joined(&file), "# head|FOO=2|CIMMERIA_LAB_BAR=a=b|# tail");
}

// 'set appends an unknown key'
#[test]
fn set_appends_an_unknown_key() {
    let mut file = lines(SRC);
    env_file::set(&mut file, "NEW_ONE", "x");
    assert_eq!(
        joined(&file),
        "# head|FOO=1|CIMMERIA_LAB_BAR=a=b|# tail|NEW_ONE=x"
    );
}

// 'unset removes the key line only'
#[test]
fn unset_removes_the_key_line_only() {
    let mut file = lines(SRC);
    assert!(env_file::unset(&mut file, "FOO"));
    assert_eq!(joined(&file), "# head|CIMMERIA_LAB_BAR=a=b|# tail");
}

// 'set changes the last duplicate, which is the one that counts'
#[test]
fn set_changes_the_last_duplicate() {
    let mut file = lines("A=1\nA=2\n");
    env_file::set(&mut file, "A", "9");
    assert_eq!(joined(&file), "A=1|A=9");
}

// 'unset of the only line gives no lines'
#[test]
fn unset_of_the_only_line_gives_no_lines() {
    let mut file = lines("FOO=1\n");
    assert!(env_file::unset(&mut file, "FOO"));
    assert!(file.is_empty());
}

// 'unset matches a key stored in another case, as the daemon reads it'
#[test]
fn unset_matches_a_key_stored_in_another_case() {
    let mut file = lines("foo=1\nBAR=2\n");
    assert!(env_file::unset(&mut file, "FOO"));
    assert_eq!(joined(&file), "BAR=2");
}

// 'set replaces a key stored in another case instead of adding a second'
#[test]
fn set_replaces_a_key_stored_in_another_case() {
    let mut file = lines("foo=1\n");
    env_file::set(&mut file, "FOO", "2");
    assert_eq!(joined(&file), "FOO=2");
}

// 'the input lines are not changed' becomes: render and parse round-trip
// the file, so an edit that touches one line cannot disturb the others.
#[test]
fn render_then_parse_round_trips_the_lines() {
    let file = lines(SRC);
    assert_eq!(env_file::parse(&env_file::render(&file)), file);
    assert_eq!(
        env_file::render(&file),
        "# head\r\nFOO=1\r\nCIMMERIA_LAB_BAR=a=b\r\n# tail\r\n"
    );
}

// New: the value is everything after the first '=' (it may hold '=').
#[test]
fn value_is_everything_after_the_first_equals() {
    let mut file = lines("# c\n");
    env_file::set(&mut file, "NEW_KEY", "a b=c");
    let reread = env_file::parse(&env_file::render(&file));
    assert_eq!(env_file::get(&reread, "NEW_KEY"), Some("a b=c"));
}

// 'a VALUE starting with - is set with -Value:' (storage side)
#[test]
fn value_starting_with_a_dash_is_stored_whole() {
    let mut file = lines("");
    env_file::set(&mut file, "DASH_KEY", "-Xmx512m");
    assert_eq!(joined(&file), "DASH_KEY=-Xmx512m");
}

// New: get ignores case and the last duplicate wins; an absent key is None.
#[test]
fn get_ignores_case_and_the_last_duplicate_wins() {
    let file = lines("a=1\nA=2\n");
    assert_eq!(env_file::get(&file, "a"), Some("2"));
    assert_eq!(env_file::get(&file, "MISSING"), None);
}

// New: unset of an absent key is false and changes nothing.
#[test]
fn unset_of_an_absent_key_returns_false() {
    let mut file = lines(SRC);
    assert!(!env_file::unset(&mut file, "MISSING_KEY"));
    assert_eq!(joined(&file), "# head|FOO=1|CIMMERIA_LAB_BAR=a=b|# tail");
}

// The file as it is written with one line changed: every other line keeps its
// exact bytes (spacing, a whitespace-only line, CRLF).
const EDIT_SRC: &str = "FOO = 1\r\n  BAR=2  \r\n   \r\n# c\r\nBAZ=3\r\n";

// New: a set leaves every untouched line byte-identical; only BAZ changes.
#[test]
fn untouched_lines_round_trip_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    std::fs::write(&path, EDIT_SRC).unwrap();

    let mut file = env_file::read(&path).unwrap();
    env_file::set(&mut file, "BAZ", "4");
    env_file::write_with_backup(&path, &file).unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "FOO = 1\r\n  BAR=2  \r\n   \r\n# c\r\nBAZ=4\r\n"
    );
}

// New: an unset leaves every other line byte-identical; only BAZ goes.
#[test]
fn unset_keeps_other_lines_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    std::fs::write(&path, EDIT_SRC).unwrap();

    let mut file = env_file::read(&path).unwrap();
    assert!(env_file::unset(&mut file, "BAZ"));
    env_file::write_with_backup(&path, &file).unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "FOO = 1\r\n  BAR=2  \r\n   \r\n# c\r\n"
    );
}

// ---- reading ----

// New (Read-LabdEnvFile): a BOM, LF, CRLF and a lone CR all parse.
#[test]
fn parse_accepts_a_bom_and_every_line_ending() {
    let file = env_file::parse("\u{feff}A=1\r\nB=2\nC=3\rD=4\n");
    assert_eq!(joined(&file), "A=1|B=2|C=3|D=4");
}

// New: blank, comment and unparsed lines are told apart.
#[test]
fn parse_line_tells_the_line_kinds_apart() {
    assert_eq!(
        env_file::parse_line("   "),
        EnvLine::Blank("   ".to_string())
    );
    assert_eq!(
        env_file::parse_line("  # c"),
        EnvLine::Comment("  # c".to_string())
    );
    assert_eq!(
        env_file::parse_line("=x"),
        EnvLine::Unparsed("=x".to_string())
    );
    assert_eq!(
        env_file::parse_line("  K = v "),
        EnvLine::Pair {
            key: "K".to_string(),
            value: " v".to_string(),
            raw: Some("  K = v ".to_string())
        }
    );
}

// New: a missing file is NotFound from read, empty from read_or_empty.
#[test]
fn missing_file_is_not_found_or_empty() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    let err = env_file::read(&path).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    assert!(env_file::read_or_empty(&path).is_empty());
}

// ---- write and backup ----

// 'env set keeps comments and order' + 'env set writes labd.env.bak-<stamp>'
// + 'the backup holds the old file'
#[test]
fn write_replaces_the_file_and_backs_up_the_old_one() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    std::fs::write(
        &path,
        "# head\r\nFOO=1\r\nCIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real\r\n# tail\r\n",
    )
    .unwrap();
    let before = std::fs::read_to_string(&path).unwrap();

    let mut file = env_file::read(&path).unwrap();
    env_file::set(&mut file, "FOO", "2");
    let backup = env_file::write_with_backup(&path, &file).unwrap().unwrap();

    let after = env_file::read(&path).unwrap();
    assert_eq!(
        joined(&after),
        "# head|FOO=2|CIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real|# tail"
    );
    let name = backup.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        Regex::new(r"^labd\.env\.bak-\d{8}-\d{6}$")
            .unwrap()
            .is_match(&name),
        "backup name {name}"
    );
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), before);
    assert_eq!(backup.parent().unwrap(), tmp.path());
}

// 'each change writes its own backup, even in the same second'
#[test]
fn each_write_in_the_same_second_gets_its_own_backup() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    std::fs::write(&path, "A=1\r\n").unwrap();
    let file = lines("A=2\n");
    let first = env_file::write_with_backup_at(&path, &file, "20261011-010203")
        .unwrap()
        .unwrap();
    let second = env_file::write_with_backup_at(&path, &file, "20261011-010203")
        .unwrap()
        .unwrap();
    let third = env_file::write_with_backup_at(&path, &file, "20261011-010203")
        .unwrap()
        .unwrap();
    let names: Vec<String> = [first, second, third]
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "labd.env.bak-20261011-010203",
            "labd.env.bak-20261011-010203-2",
            "labd.env.bak-20261011-010203-3"
        ]
    );
}

// 'no labd.env.tmp is left behind'
#[test]
fn no_tmp_file_is_left_behind() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    std::fs::write(&path, "A=1\r\n").unwrap();
    env_file::write_with_backup(&path, &lines("A=2\n")).unwrap();
    assert!(!tmp.path().join("labd.env.tmp").exists());
}

// New: no existing file means no backup, and the folder is created.
#[test]
fn writing_a_new_file_makes_no_backup_and_creates_the_folder() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nested").join("labd.env");
    let backup = env_file::write_with_backup(&path, &lines("A=1\n")).unwrap();
    assert_eq!(backup, None);
    assert_eq!(std::fs::read(&path).unwrap(), b"A=1\r\n");
}

// New (labd-lib Write-LabdEnvFile): UTF-8 without a BOM, CRLF.
#[test]
fn written_file_is_utf8_without_bom_and_crlf() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("labd.env");
    env_file::write_with_backup(&path, &lines("# h\u{e9}\nA=1\n")).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes, "# h\u{e9}\r\nA=1\r\n".as_bytes());
    assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
}
