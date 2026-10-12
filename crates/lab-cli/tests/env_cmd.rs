//! `lab env` as the binary, on a temp lab home. Ports the command runs of
//! tools/lab/cli/test-logs-env.ps1 (the PowerShell check is named above each
//! test). The pure masking, key and line-rewrite checks of that file are
//! ported by LX-04 in tests/daemon_env.rs.

use std::path::PathBuf;
use std::process::Command;

mod support;

const START: &str =
    "# head\r\nFOO=1\r\nCIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real\r\n# tail\r\n";
const USAGE: &str =
    "usage: lab env [get KEY | set KEY VALUE | unset KEY]  (a VALUE starting with '-': -Value:<value>)";
const RESTART: &str = "restart the daemon for this to apply: lab restart";

struct Lab {
    home: tempfile::TempDir,
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

impl Lab {
    fn new() -> Self {
        Self {
            home: support::isolated_lab_home(),
        }
    }

    fn file(&self) -> PathBuf {
        self.home.path().join("labd.env")
    }

    fn seed(&self, text: &str) {
        std::fs::write(self.file(), text).expect("seed labd.env");
    }

    fn read(&self) -> String {
        std::fs::read_to_string(self.file()).expect("read labd.env")
    }

    /// File names in the lab home that start with `labd.env.bak-`, sorted.
    fn backups(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.home.path())
            .expect("read lab home")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.starts_with("labd.env.bak-"))
            .collect();
        names.sort();
        names
    }

    /// Runs `lab env <args>`.
    fn run(&self, args: &[&str]) -> Run {
        let output = Command::new(env!("CARGO_BIN_EXE_lab"))
            .arg("env")
            .args(args)
            .env("CIMMERIA_LAB_HOME", self.home.path())
            .output()
            .expect("the lab binary should start");
        Run {
            code: output.status.code().expect("an exit code"),
            out: String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            err: String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n"),
        }
    }
}

// PowerShell: 'env lists every line with the token masked'
#[test]
fn env_lists_every_line_with_the_token_masked() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&[]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(
        run.out,
        "# head\nFOO=1\nCIMMERIA_LAB_DAEMON_TOKEN=<redacted>\n# tail\n"
    );
}

// PowerShell: 'env get masks a token key'
#[test]
fn env_get_masks_a_token_key() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["get", "CIMMERIA_LAB_DAEMON_TOKEN"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(run.out, "<redacted>\n");
}

// PowerShell: 'env get with a bad key exits 2'
#[test]
fn env_get_with_a_bad_key_exits_2() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["get", "bad_key"]);
    assert_eq!(run.code, 2);
    assert_eq!(run.out, "");
    assert_eq!(
        run.err,
        format!("KEY must match ^[A-Z][A-Z0-9_]*$\n{USAGE}\n")
    );
}

// PowerShell: 'env set with a bad key exits 2 and changes nothing'
#[test]
fn env_set_with_a_bad_key_exits_2_and_changes_nothing() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["set", "bad_key", "x"]);
    assert_eq!(run.code, 2);
    assert_eq!(lab.read(), START);
    assert!(lab.backups().is_empty());
}

// PowerShell: 'env set without a value exits 2'
#[test]
fn env_set_without_a_value_exits_2() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["set", "FOO"]);
    assert_eq!(run.code, 2);
    assert_eq!(run.err, format!("set needs KEY and VALUE\n{USAGE}\n"));
    assert_eq!(lab.read(), START);
}

// PowerShell: 'env set FOO 2 exits 0', 'env set keeps comments and order',
// 'env set writes labd.env.bak-<stamp>', 'the backup holds the old file',
// 'no labd.env.tmp is left behind'
#[test]
fn env_set_keeps_comments_and_order_and_backs_up_the_old_file() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["set", "FOO", "2"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(
        lab.read(),
        "# head\r\nFOO=2\r\nCIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real\r\n# tail\r\n"
    );
    let backups = lab.backups();
    assert_eq!(backups.len(), 1, "{backups:?}");
    let pattern = regex::Regex::new(r"^labd\.env\.bak-\d{8}-\d{6}$").expect("regex");
    assert!(pattern.is_match(&backups[0]), "{}", backups[0]);
    let backup_path = lab.home.path().join(&backups[0]);
    assert_eq!(
        std::fs::read_to_string(&backup_path).expect("backup"),
        START
    );
    assert_eq!(
        run.out,
        format!("backup: {}\n{RESTART}\n", backup_path.display())
    );
    assert!(!lab.home.path().join("labd.env.tmp").exists());
}

// PowerShell: 'env set appends an unknown key, unset removes FOO', 'env unset FOO
// exits 0', 'each change writes its own backup, even in the same second'
#[test]
fn env_set_appends_unset_removes_and_every_change_has_its_own_backup() {
    let lab = Lab::new();
    lab.seed(START);
    assert_eq!(lab.run(&["set", "FOO", "2"]).code, 0);
    assert_eq!(lab.run(&["set", "NEW_KEY", "a b=c"]).code, 0);
    assert_eq!(lab.run(&["unset", "FOO"]).code, 0);
    assert_eq!(
        lab.read(),
        "# head\r\nCIMMERIA_LAB_DAEMON_TOKEN=test-token-not-real\r\n# tail\r\nNEW_KEY=a b=c\r\n"
    );
    assert_eq!(lab.backups().len(), 3, "{:?}", lab.backups());
}

// PowerShell: 'unset of an absent key exits 0 and writes nothing'
#[test]
fn env_unset_of_an_absent_key_exits_0_and_writes_nothing() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["unset", "MISSING_KEY"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(run.out, "MISSING_KEY is not set; labd.env unchanged\n");
    assert!(lab.backups().is_empty());
    assert_eq!(lab.read(), START);
}

// PowerShell: 'a VALUE with trailing spaces is refused (exit 2)'
#[test]
fn env_set_refuses_a_value_with_edge_spaces() {
    let lab = Lab::new();
    lab.seed(START);
    let message =
        "VALUE must not start or end with spaces (labd.env lines are trimmed when read)\n";
    let trailing = lab.run(&["set", "PAD_KEY", "val  "]);
    assert_eq!(trailing.code, 2);
    assert_eq!(trailing.err, message);
    let leading = lab.run(&["set", "PAD_KEY", "  val"]);
    assert_eq!(leading.code, 2);
    assert_eq!(leading.err, message);
    assert_eq!(lab.read(), START);
    assert!(lab.backups().is_empty());
}

// PowerShell: 'a VALUE starting with - is set with -Value:'; packet: SOME_FLAGS -Value:-x
#[test]
fn env_set_takes_a_dash_value_in_the_powershell_spelling() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["set", "DASH_KEY", "-Value:-Xmx512m"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert!(
        lab.read().contains("\r\nDASH_KEY=-Xmx512m\r\n"),
        "{}",
        lab.read()
    );
    let run = lab.run(&["set", "SOME_FLAGS", "-Value:-x"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert!(
        lab.read().ends_with("\r\nSOME_FLAGS=-x\r\n"),
        "{}",
        lab.read()
    );
    let run = lab.run(&["get", "SOME_FLAGS"]);
    assert_eq!(run.out, "-x\n");
}

// PowerShell: 'the token never appears in env output'
#[test]
fn env_output_never_holds_the_token() {
    let lab = Lab::new();
    lab.seed(START);
    let listed = lab.run(&[]);
    let got = lab.run(&["get", "CIMMERIA_LAB_DAEMON_TOKEN"]);
    for text in [&listed.out, &listed.err, &got.out, &got.err] {
        assert!(!text.contains("test-token-not-real"), "{text}");
    }
}

// New (the PowerShell had no check): get of an unset key.
#[test]
fn env_get_of_an_unset_key_exits_1() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&["get", "NOPE"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.out, "");
    assert_eq!(run.err, "NOPE is not set\n");
}

// New: no labd.env at all.
#[test]
fn env_listing_without_a_labd_env_exits_1() {
    let lab = Lab::new();
    let run = lab.run(&[]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.err,
        format!("no labd.env at {}\n", lab.file().display())
    );
}

// New: set creates the file; there is nothing to back up, so no `backup:` line.
#[test]
fn env_set_creates_labd_env_without_a_backup() {
    let lab = Lab::new();
    let run = lab.run(&["set", "A", "1"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(run.out, format!("{RESTART}\n"));
    assert_eq!(lab.read(), "A=1\r\n");
    assert!(lab.backups().is_empty());
}

// New: an unknown verb.
#[test]
fn env_unknown_verb_exits_2() {
    let lab = Lab::new();
    let run = lab.run(&["frob", "X"]);
    assert_eq!(run.code, 2);
    assert_eq!(run.err, format!("unknown verb 'frob'\n{USAGE}\n"));
}

// New: PowerShell's -notin and the daemon's key map both ignore case.
#[test]
fn env_verb_and_file_keys_ignore_case() {
    let lab = Lab::new();
    lab.seed("foo=1\r\n");
    let run = lab.run(&["GET", "FOO"]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(run.out, "1\n");
}

// New: a value with a line break.
#[test]
fn env_set_refuses_a_multi_line_value() {
    let lab = Lab::new();
    let run = lab.run(&["set", "A", "x\ny"]);
    assert_eq!(run.code, 2);
    assert_eq!(run.err, "VALUE must be one line\n");
    assert!(!lab.file().exists());
}

// New: an empty value is allowed, as it was in PowerShell.
#[test]
fn env_set_accepts_an_empty_value() {
    let lab = Lab::new();
    let run = lab.run(&["set", "EMPTY", ""]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert_eq!(lab.read(), "EMPTY=\r\n");
}

// New: a positional VALUE and --value together are a clap usage error.
#[test]
fn env_value_and_value_flag_conflict() {
    let lab = Lab::new();
    let run = lab.run(&["set", "A", "1", "-Value:2"]);
    assert_eq!(run.code, 2);
    assert!(!lab.file().exists());
}

// New: other lines stay byte-identical through set and unset (odd spacing, LF
// endings, a stray line, a lower-case key, no final newline).
#[test]
fn env_set_and_unset_leave_other_lines_byte_identical() {
    let lab = Lab::new();
    lab.seed("# c\n  spaced = 1  \nfoo=low\nstray-line\n\nTARGET=old\n#end");
    assert_eq!(lab.run(&["set", "TARGET", "new"]).code, 0);
    assert_eq!(
        lab.read(),
        "# c\r\n  spaced = 1  \r\nfoo=low\r\nstray-line\r\n\r\nTARGET=new\r\n#end\r\n"
    );
    assert_eq!(lab.run(&["unset", "TARGET"]).code, 0);
    assert_eq!(
        lab.read(),
        "# c\r\n  spaced = 1  \r\nfoo=low\r\nstray-line\r\n\r\n#end\r\n"
    );
}

// New: the listing and get never print a bearer token, a 64-hex token, URL
// userinfo or host, or a stray pasted line.
#[test]
fn env_masks_bearer_hex_url_and_stray_lines() {
    let lab = Lab::new();
    let hex = "0123456789abcdef".repeat(4);
    lab.seed(&format!(
        "U=http://user:p/ss@host.example:8779/x\r\nN=Authorization Bearer abc123def\r\nH={hex}\r\n# note TOKEN=abc\r\npasted-secret-line\r\n"
    ));
    let listed = lab.run(&[]);
    assert_eq!(listed.code, 0, "{}", listed.err);
    assert_eq!(
        listed.out,
        "U=http://<host>:8779/x\nN=Authorization <redacted>\nH=<redacted>\n# note TOKEN=<redacted>\n<unparsed line>\n"
    );
    let got = lab.run(&["get", "U"]);
    assert_eq!(got.out, "http://<host>:8779/x\n");
    for text in [&listed.out, &got.out] {
        for secret in ["user", "host.example", "abc123def", hex.as_str(), "pasted"] {
            assert!(!text.contains(secret), "{secret} leaked in {text}");
        }
    }
}

// New: an empty verb lists, as `-not $Verb` did in the PowerShell.
#[test]
fn env_empty_verb_lists() {
    let lab = Lab::new();
    lab.seed(START);
    let run = lab.run(&[""]);
    assert_eq!(run.code, 0, "{}", run.err);
    assert!(run.out.starts_with("# head\n"), "{}", run.out);
}

// New: a labd.env that exists but cannot be read (a folder stands in for it)
// is `cannot read`, exit 1, and set writes nothing.
#[test]
fn env_unreadable_labd_env_exits_1() {
    let lab = Lab::new();
    std::fs::create_dir(lab.file()).expect("labd.env folder");
    for args in [&[][..], &["get", "A"][..], &["set", "A", "1"][..]] {
        let run = lab.run(args);
        assert_eq!(run.code, 1, "{args:?}: {}", run.err);
        assert!(
            run.err
                .starts_with(&format!("cannot read {}: ", lab.file().display())),
            "{}",
            run.err
        );
    }
    assert!(lab.backups().is_empty());
}
