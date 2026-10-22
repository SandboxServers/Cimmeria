//! LF-19: the game's debug and session log lines, read from real files.
use super::*;
use std::fs;

fn binaries(game: &Path) -> PathBuf {
    let dir = game.join("Working").join("Binaries");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn utf16(text: &str, bom: bool) -> Vec<u8> {
    let mut out = if bom { vec![0xFF, 0xFE] } else { Vec::new() };
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

fn read(game: &Path, offsets: &Offsets) -> GameCollected {
    collect(game, offsets, &CancellationToken::new()).unwrap()
}

/// What the flow does after a send: `advance_game` for every line.
fn sent(offsets: &mut Offsets, got: &GameCollected) {
    for line in &got.collected.lines {
        offsets.advance_game(line, &got.files);
    }
}

fn messages(got: &GameCollected) -> Vec<&str> {
    got.collected
        .lines
        .iter()
        .map(|l| l.message.as_str())
        .collect()
}

#[test]
fn a_utf16le_bom_log_decodes_and_resumes_at_the_byte_offset() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    let first = utf16("alpha\r\nbeta\r\n", true);
    fs::write(&path, &first).unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(messages(&got), ["alpha", "beta"]);
    let mut offsets = Offsets::default();
    sent(&mut offsets, &got);
    assert_eq!(
        offsets.files.get("game/SGWDebugLog.log").copied(),
        Some(first.len() as u64),
        "the offset counts file bytes, BOM and two-byte units included"
    );
    let mut more = first.clone();
    more.extend(utf16("gamma\r\npartial", false));
    fs::write(&path, &more).unwrap();
    let got = read(game.path(), &offsets);
    assert_eq!(
        messages(&got),
        ["gamma"],
        "resumes after beta; the half-written last line waits for its newline"
    );
    assert!(!got.files["game/SGWDebugLog.log"].restarted);
}

#[test]
fn a_bomless_utf16le_log_is_found_by_its_nul_bytes() {
    let game = tempfile::tempdir().unwrap();
    fs::write(
        binaries(game.path()).join("SGWDebugLog.log"),
        utf16("one\ntwo\n", false),
    )
    .unwrap();
    assert_eq!(
        messages(&read(game.path(), &Offsets::default())),
        ["one", "two"]
    );
}

#[test]
fn a_utf8_log_resumes_at_its_byte_offset_and_keeps_multibyte_text() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    let text = "caf\u{e9} ready\r\nsecond\r\n";
    fs::write(&path, text).unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(messages(&got), ["caf\u{e9} ready", "second"]);
    let mut offsets = Offsets::default();
    sent(&mut offsets, &got);
    assert_eq!(
        offsets.files["game/SGWDebugLog.log"],
        text.len() as u64,
        "bytes, not characters"
    );
    assert!(read(game.path(), &offsets).collected.lines.is_empty());
}

#[test]
fn a_rewritten_log_with_a_different_head_restarts_at_zero() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    fs::write(&path, "old run line one\nold run line two\n").unwrap();
    let mut offsets = Offsets::default();
    let first = read(game.path(), &offsets);
    sent(&mut offsets, &first);
    // The client's next launch rewrites the file, and this one is LONGER than
    // the saved offset, so only the head fingerprint can tell it is new.
    fs::write(
        &path,
        "new run line one\nnew run line two\nnew run line three\n",
    )
    .unwrap();
    let got = read(game.path(), &offsets);
    assert!(got.files["game/SGWDebugLog.log"].restarted);
    assert_eq!(
        messages(&got),
        ["new run line one", "new run line two", "new run line three"]
    );
    sent(&mut offsets, &got);
    assert!(
        read(game.path(), &offsets).collected.lines.is_empty(),
        "after the restart is sent the file is caught up"
    );
}

#[test]
fn a_shorter_file_than_the_saved_offset_restarts_even_without_a_head() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    fs::write(&path, "x\n").unwrap();
    let mut offsets = Offsets::default();
    offsets.files.insert("game/SGWDebugLog.log".into(), 9_999);
    let got = read(game.path(), &offsets);
    assert_eq!(messages(&got), ["x"]);
    sent(&mut offsets, &got);
    assert_eq!(offsets.files["game/SGWDebugLog.log"], 2);
}

#[test]
fn a_growing_short_file_is_not_mistaken_for_a_rewrite() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    fs::write(&path, "hi\n").unwrap();
    let mut offsets = Offsets::default();
    let first = read(game.path(), &offsets);
    sent(&mut offsets, &first);
    fs::write(&path, "hi\nthere\n").unwrap();
    let got = read(game.path(), &offsets);
    assert!(!got.files["game/SGWDebugLog.log"].restarted);
    assert_eq!(messages(&got), ["there"]);
}

#[test]
fn key_dumps_and_the_session_json_are_never_read() {
    let game = tempfile::tempdir().unwrap();
    let session = binaries(game.path()).join("sessions").join("x");
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("a-keys.txt"), "SECRET-KEY-DUMP\n").unwrap();
    fs::write(session.join("current-session.json"), "SECRET-TOKEN\n").unwrap();
    fs::write(session.join("client.log"), "session line\n").unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(messages(&got), ["session line"]);
    assert_eq!(
        got.collected.lines[0].file, "game/sessions/x/client.log",
        "the offsets key keeps the path under the binaries folder"
    );
    assert_eq!(
        row_lines(&got.collected.lines)[0].file,
        "client.log",
        "the row's file field is a bare name (C-LF5)"
    );
}

#[test]
fn chat_lines_are_sent_unchanged() {
    let game = tempfile::tempdir().unwrap();
    fs::write(
        binaries(game.path()).join("SGWDebugLog.log"),
        "[Say] Bob: hello\n",
    )
    .unwrap();
    assert_eq!(
        messages(&read(game.path(), &Offsets::default())),
        ["[Say] Bob: hello"],
        "D-LU3: no content filter"
    );
}

#[test]
fn addresses_are_still_scrubbed_and_long_lines_cut_on_a_char_boundary() {
    let game = tempfile::tempdir().unwrap();
    let long = "\u{e9}".repeat(1_500);
    fs::write(
        binaries(game.path()).join("SGWDebugLog.log"),
        format!("connect to 203.0.113.9 now\n{long}\n"),
    )
    .unwrap();
    let got = read(game.path(), &Offsets::default());
    assert!(!got.collected.lines[0].message.contains("203.0.113.9"));
    let cut = &got.collected.lines[1];
    assert!(cut.truncated);
    assert_eq!(cut.message.len(), 2_048, "the floor is 2,048 bytes (D-LU4)");
}

#[test]
fn lines_total_counts_past_the_cap_and_the_newest_lines_are_kept() {
    let game = tempfile::tempdir().unwrap();
    let text: String = (0..MAX_LINES + 5).map(|i| format!("line {i}\n")).collect();
    fs::write(binaries(game.path()).join("SGWDebugLog.log"), text).unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(got.collected.lines_total, (MAX_LINES + 5) as u64);
    assert_eq!(got.collected.lines.len(), MAX_LINES);
    assert_eq!(got.collected.lines[0].message, "line 5");
}

#[test]
fn a_missing_game_folder_is_no_lines_and_no_files() {
    let game = tempfile::tempdir().unwrap();
    let got = read(&game.path().join("absent"), &Offsets::default());
    assert!(got.collected.lines.is_empty());
    assert!(!got.has_files);
}

#[test]
fn a_cancel_before_the_read_is_collect_failed() {
    let game = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        collect(game.path(), &Offsets::default(), &cancel).unwrap_err(),
        LogUploadError::CollectFailed
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_log_is_skipped() {
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    let elsewhere = game.path().join("elsewhere.txt");
    fs::write(&elsewhere, "not a game log\n").unwrap();
    std::os::unix::fs::symlink(&elsewhere, dir.join("SGWDebugLog.log")).unwrap();
    assert!(read(game.path(), &Offsets::default())
        .collected
        .lines
        .is_empty());
}

/// A session folder the process cannot list must not cost the launcher's own
/// lines: the game's files degrade to none. Skipped when the process can list
/// it anyway (root, or a platform without the permission).
#[cfg(unix)]
#[test]
fn an_unlistable_session_folder_is_no_game_lines_not_a_failure() {
    use std::os::unix::fs::PermissionsExt;
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    let locked = dir.join("sessions").join("x");
    fs::create_dir_all(&locked).unwrap();
    fs::write(locked.join("a.log"), "line\n").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&locked).is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("skipped: this process can list a chmod 000 folder");
        return;
    }
    let got = collect(game.path(), &Offsets::default(), &CancellationToken::new());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let got = got.unwrap();
    assert!(got.collected.lines.is_empty());
}

/// A 4 MiB line: the path past the first 32 KiB never reaches the scrub or the
/// message, and the message is the 2,048-byte cut, flagged as truncated.
#[test]
fn a_huge_line_is_cut_before_the_scrub() {
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    let mut line = "a".repeat(3 << 20);
    line.push_str(" C:\\Users\\bob\\x.dll ");
    line.push_str(&"b".repeat(1 << 20));
    line.push('\n');
    fs::write(dir.join("SGWDebugLog.log"), line).unwrap();
    let got = read(game.path(), &Offsets::default());
    let [only] = got.collected.lines.as_slice() else {
        panic!("expected one line, got {}", got.collected.lines.len());
    };
    assert_eq!(only.message.len(), MAX_MESSAGE_BYTES);
    assert!(only.truncated);
    assert!(!only.message.contains("C:"));
}

/// A NUL run is kept as U+FFFD, so it cannot inflate the row's escaping.
#[test]
fn control_characters_become_replacement_characters() {
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    // Plain text first, so the encoding check reads the file as UTF-8.
    let text = format!("{}{}\n", "a".repeat(600), "\0".repeat(2_048));
    fs::write(dir.join("SGWDebugLog.log"), text).unwrap();
    let got = read(game.path(), &Offsets::default());
    let [only] = got.collected.lines.as_slice() else {
        panic!("expected one line");
    };
    assert!(!only.message.contains('\0'));
    assert!(only.message.contains('\u{FFFD}'));
    assert!(only.truncated);
}

#[test]
fn a_missing_file_is_read_failed_for_telemetry() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone.log");
    let got = read_file(
        "game/gone.log",
        &missing,
        0,
        (None, None),
        &Redaction::from_env(),
    );
    assert!(matches!(got, Err("read_failed")));
}

/// A 64 MiB file is streamed: every line is counted, only the newest `MAX_LINES`
/// are kept, and `line_no` is the number a whole-file read would give.
#[test]
fn a_64_mib_file_is_streamed_and_keeps_the_newest_lines() {
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    let mut text = Vec::with_capacity(64 << 20);
    let mut n = 0u64;
    while text.len() < 64 << 20 {
        n += 1;
        text.extend_from_slice(format!("line {n:08}\n").as_bytes());
    }
    fs::write(dir.join("SGWDebugLog.log"), &text).unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(got.collected.lines_total, n);
    assert_eq!(got.collected.lines.len(), MAX_LINES);
    assert_eq!(got.collected.lines[0].line_no, n - MAX_LINES as u64 + 1);
    let last = got.collected.lines.last().unwrap();
    assert_eq!(last.line_no, n);
    assert_eq!(last.message, format!("line {n:08}"));
}

/// The per-word scrub: a path word and a long key word are replaced, the rest of
/// the line stays (A5, A9). The launcher's whole-value scrub still blanks the line.
#[test]
fn a_path_or_key_word_is_scrubbed_and_the_rest_of_the_line_kept() {
    let game = tempfile::tempdir().unwrap();
    let dir = binaries(game.path());
    let text = "loaded C:\\Users\\bob\\x.dll ok\nkey=QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVowMTIzNDU2Nzg5 done\n";
    fs::write(dir.join("SGWDebugLog.log"), text).unwrap();
    let got = read(game.path(), &Offsets::default());
    assert_eq!(
        messages(&got),
        vec!["loaded <path removed> ok", "<removed> done"]
    );
    let redaction = Redaction {
        home: None,
        user: None,
    };
    assert_eq!(
        redaction.scrub("loaded C:\\Users\\bob\\x.dll ok"),
        "<path removed>"
    );
}

#[test]
fn a_version_one_offsets_file_loads_with_no_heads() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("uploaded-log-offsets.json"),
        r#"{"version":1,"files":{"launcher.2026-10-22.log":120}}"#,
    )
    .unwrap();
    let offsets = Offsets::load(dir.path());
    assert_eq!(offsets.files["launcher.2026-10-22.log"], 120);
    assert!(offsets.heads.is_empty());
    offsets.save(dir.path()).unwrap();
    let again = Offsets::load(dir.path());
    assert_eq!(again.files, offsets.files, "saves as version 2 and reloads");
}

/// A rewrite whose first 256 bytes are the same as the old run's: only the tail
/// fingerprint at the saved offset tells it is new (A6). Without the tail check
/// the new run's early lines end before the saved offset and are never sent.
#[test]
fn a_rewritten_log_with_the_same_head_restarts_by_its_tail() {
    let game = tempfile::tempdir().unwrap();
    let path = binaries(game.path()).join("SGWDebugLog.log");
    // A 300-byte banner starts both runs, so the 256-byte head cannot tell them apart.
    let banner = format!("{}\n", "=".repeat(300));
    let old: String = (0..20).map(|n| format!("old run {n:02}\n")).collect();
    fs::write(&path, format!("{banner}{old}")).unwrap();
    let mut offsets = Offsets::default();
    let first = read(game.path(), &offsets);
    sent(&mut offsets, &first);
    assert!(offsets.tails.contains_key("game/SGWDebugLog.log"));
    // The new run is longer than the saved offset.
    let new: String = (0..30).map(|n| format!("new run {n:02}\n")).collect();
    fs::write(&path, format!("{banner}{new}")).unwrap();
    let got = read(game.path(), &offsets);
    assert!(got.files["game/SGWDebugLog.log"].restarted);
    assert_eq!(
        messages(&got).len(),
        31,
        "the banner and all 30 new lines are sent"
    );
    sent(&mut offsets, &got);
    assert!(
        read(game.path(), &offsets).collected.lines.is_empty(),
        "the saved tail matches the file, so the caught-up file sends nothing"
    );
}

/// A version-2 file with no `tails` key loads with no tail check, as before.
#[test]
fn a_version_two_offsets_file_without_tails_loads_with_none() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("uploaded-log-offsets.json"),
        r#"{"version":2,"files":{"game/SGWDebugLog.log":12},"heads":{}}"#,
    )
    .unwrap();
    let offsets = Offsets::load(dir.path());
    assert_eq!(offsets.files["game/SGWDebugLog.log"], 12);
    assert!(offsets.tails.is_empty());
}

/// Past the file limit, the oldest game file by modification time goes, but a
/// file with lines this press (busy) never does, and the newest survives (A7).
#[test]
fn past_the_file_limit_the_oldest_unbusy_file_goes_and_the_newest_stays() {
    let mut offsets = Offsets::default();
    let mut live = HashMap::new();
    for n in 0..600u64 {
        let key = format!("game/sessions/s{n:03}.log");
        offsets.files.insert(key.clone(), 10);
        let modified = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(n);
        live.insert(key, modified);
    }
    let busy = std::collections::HashSet::from(["game/sessions/s000.log".to_string()]);
    offsets.prune(&live, &busy);
    assert_eq!(offsets.files.len(), 512);
    assert!(offsets.files.contains_key("game/sessions/s000.log"));
    assert!(!offsets.files.contains_key("game/sessions/s001.log"));
    assert!(offsets.files.contains_key("game/sessions/s599.log"));
}

/// A game file that is gone is pruned with its head and tail; launcher keys stay.
#[test]
fn a_deleted_game_file_is_pruned_and_a_launcher_key_is_kept() {
    let mut offsets = Offsets::default();
    offsets.files.insert("game/gone.log".into(), 5);
    offsets.heads.insert("game/gone.log".into(), "5:x".into());
    offsets.tails.insert("game/gone.log".into(), "5:x".into());
    offsets.files.insert("launcher.2026-10-22.log".into(), 7);
    offsets.files.insert("game/here.log".into(), 9);
    let live = HashMap::from([(
        "game/here.log".to_string(),
        std::time::SystemTime::UNIX_EPOCH,
    )]);
    offsets.prune(&live, &std::collections::HashSet::new());
    assert!(!offsets.files.contains_key("game/gone.log"));
    assert!(!offsets.heads.contains_key("game/gone.log"));
    assert!(!offsets.tails.contains_key("game/gone.log"));
    assert_eq!(offsets.files["launcher.2026-10-22.log"], 7);
    assert_eq!(offsets.files["game/here.log"], 9);
}
