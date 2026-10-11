use super::*;

fn ov(file: &str, section: &str, key: &str, value: &str) -> IniOverride {
    IniOverride {
        file: file.into(),
        section: section.into(),
        key: key.into(),
        value: value.into(),
    }
}

fn entry_toml(file: &str, section: &str, key: &str, value: &str) -> String {
    format!(
        "[[override]]\nfile = {file:?}\nsection = {section:?}\nkey = {key:?}\nvalue = {value}\n"
    )
}

#[test]
fn an_override_replaces_the_key_in_its_section_only() {
    // The other section's key comes first: dropping the section match
    // would replace that one instead.
    let text =
        "[Engine.GameEngine]\nHysteresisLimit=20\n\n[TextureStreaming]\nHysteresisLimit=10\n";
    let (out, applied) = apply(
        text,
        &[ov(
            "SGWEngine.ini",
            "TextureStreaming",
            "HysteresisLimit",
            "99",
        )],
    );
    assert_eq!(
        out,
        "[Engine.GameEngine]\nHysteresisLimit=20\n\n[TextureStreaming]\nHysteresisLimit=99\n"
    );
    assert_eq!(
        applied,
        vec![("HysteresisLimit".to_string(), Some("10".to_string()))]
    );
}

#[test]
fn a_missing_key_is_added_to_its_section() {
    let text = "[B]\ny=2\n\n[C]\nq=1\n";
    let (out, applied) = apply(text, &[ov("f.ini", "B", "z", "3")]);
    assert_eq!(out, "[B]\ny=2\nz=3\n\n[C]\nq=1\n");
    assert_eq!(applied, vec![("z".to_string(), None)]);

    // No final line break: one is added before the new line.
    let (out, _) = apply("[B]\ny=2", &[ov("f.ini", "B", "z", "3")]);
    assert_eq!(out, "[B]\ny=2\nz=3\n");
}

#[test]
fn a_missing_section_is_appended() {
    let (out, applied) = apply("[A]\nx=1\n", &[ov("f.ini", "B", "k", "v")]);
    assert_eq!(out, "[A]\nx=1\n\n[B]\nk=v\n");
    assert_eq!(applied, vec![("k".to_string(), None)]);
}

#[test]
fn crlf_is_kept() {
    let text = "[A]\r\nx=1\r\n[B]\r\ny=2\r\n";
    let (out, _) = apply(
        text,
        &[
            ov("f.ini", "A", "z", "3"),
            ov("f.ini", "B", "y", "5"),
            ov("f.ini", "C", "k", "v"),
        ],
    );
    assert_eq!(out, "[A]\r\nx=1\r\nz=3\r\n[B]\r\ny=5\r\n\r\n[C]\r\nk=v\r\n");
    assert!(!out.replace("\r\n", "").contains('\n'));
}

#[test]
fn matching_ignores_case() {
    let (out, applied) = apply(
        "[TextureStreaming]\nHysteresisLimit=10\n",
        &[ov("f.ini", "texturestreaming", "hysteresislimit", "7")],
    );
    assert_eq!(out, "[TextureStreaming]\nHysteresisLimit=7\n");
    assert_eq!(
        applied,
        vec![("hysteresislimit".to_string(), Some("10".to_string()))]
    );
}

/// A trimmed copy of the stock user `SGWEngine.ini`'s shape: CRLF, `;`
/// comments, repeated `Package=` lines, UE3 `+Key=` array edits, a key on
/// the first line of the first section, and no line break at the end.
const STOCK_SHAPE: &str = "[Engine.ScriptPackages]\r\n\
    NativePackages=Core\r\n\
    ;Package=Commented\r\n\
    +Package=Plus\r\n\
    Package=Engine\r\n\
    Package=SGWGame\r\n\
    \r\n\
    [TextureStreaming]\r\n\
    ;HysteresisLimit=5\r\n\
    +HysteresisLimit=6\r\n\
    HysteresisLimit=20\r\n\
    PoolSize=100";

#[test]
fn the_stock_shape_changes_only_the_first_bare_key() {
    let (out, applied) = apply(
        STOCK_SHAPE,
        &[
            ov(
                "SGWEngine.ini",
                "Engine.ScriptPackages",
                "NativePackages",
                "X",
            ),
            ov("SGWEngine.ini", "Engine.ScriptPackages", "Package", "P"),
            ov("SGWEngine.ini", "TextureStreaming", "HysteresisLimit", "99"),
            ov("SGWEngine.ini", "TextureStreaming", "PoolSize", "200"),
        ],
    );
    assert_eq!(
        out,
        "[Engine.ScriptPackages]\r\n\
         NativePackages=X\r\n\
         ;Package=Commented\r\n\
         +Package=Plus\r\n\
         Package=P\r\n\
         Package=SGWGame\r\n\
         \r\n\
         [TextureStreaming]\r\n\
         ;HysteresisLimit=5\r\n\
         +HysteresisLimit=6\r\n\
         HysteresisLimit=99\r\n\
         PoolSize=200"
    );
    let olds: Vec<_> = applied.into_iter().map(|(_, old)| old).collect();
    assert_eq!(
        olds,
        ["Core", "Engine", "20", "100"].map(|s| Some(s.to_string()))
    );
}

#[test]
fn comment_and_array_edit_lines_are_never_matched() {
    // Only `;Key` and `+Key` lines: the key counts as absent and is added.
    let text = "[A]\r\n;Key=1\r\n+Key=2\r\n-Key=3\r\n";
    let (out, applied) = apply(text, &[ov("f.ini", "A", "Key", "9")]);
    assert_eq!(out, "[A]\r\n;Key=1\r\n+Key=2\r\n-Key=3\r\nKey=9\r\n");
    assert_eq!(applied, vec![("Key".to_string(), None)]);
}

#[test]
fn the_space_after_equals_is_kept() {
    let (out, applied) = apply("[A]\nKey= 1\n", &[ov("f.ini", "A", "Key", "2")]);
    assert_eq!(out, "[A]\nKey= 2\n");
    assert_eq!(applied, vec![("Key".to_string(), Some("1".to_string()))]);
}

#[test]
fn iniversion_and_path_escapes_are_refused() {
    let ok = entry_toml("SGWEngine.ini", "A", "K", "\"1\"");
    assert_eq!(parse(&ok).unwrap().len(), 1);
    for text in [
        entry_toml("SGWEngine.ini", "IniVersion", "Stamp", "\"1\""),
        entry_toml("SGWEngine.ini", " iniversion ", "Stamp", "\"1\""),
        entry_toml("..\\x.ini", "A", "K", "\"1\""),
        entry_toml("a/b.ini", "A", "K", "\"1\""),
        entry_toml("C:x.ini", "A", "K", "\"1\""),
        entry_toml("x.ini:stream", "A", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "  ", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "A", "", "\"1\""),
        entry_toml("SGWEngine.ini", "A", " ", "\"1\""),
        entry_toml("SGWEngine.ini", "A]\n[B", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "A]", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "[A", "K", "\"1\""),
        entry_toml("SGWEngine.ini", "A", "K", "\"1\\n[B]\""),
    ] {
        assert!(parse(&text).is_err(), "{text}");
    }
}

#[test]
fn windows_device_names_are_refused() {
    for name in [
        "CON", "con.ini", "PRN.ini", "aux", "NUL.txt", "COM1", "com9.ini", "LPT1.ini", "lpt9",
        "NUL .ini",
    ] {
        let text = entry_toml(name, "A", "K", "\"1\"");
        assert!(parse(&text).is_err(), "{name}");
    }
    for name in [
        "CONSOLE.ini",
        "COM0.ini",
        "COM10.ini",
        "LPT.ini",
        "NULL.ini",
        "SGWEngine.ini",
    ] {
        let text = entry_toml(name, "A", "K", "\"1\"");
        assert!(parse(&text).is_ok(), "{name}");
    }
}

#[test]
fn section_and_key_are_stored_trimmed() {
    let text = entry_toml(
        "SGWEngine.ini",
        " TextureStreaming ",
        " HysteresisLimit ",
        "\"9\"",
    );
    assert_eq!(
        parse(&text).unwrap(),
        vec![ov(
            "SGWEngine.ini",
            "TextureStreaming",
            "HysteresisLimit",
            "9"
        )]
    );
}

#[test]
fn integer_float_and_boolean_values_become_ini_text() {
    for (toml_value, ini) in [
        ("99", "99"),
        ("-3", "-3"),
        ("0.5", "0.5"),
        ("30.0", "30"),
        ("true", "TRUE"),
        ("false", "FALSE"),
        ("\"text\"", "text"),
    ] {
        let parsed = parse(&entry_toml("SGWEngine.ini", "A", "K", toml_value)).unwrap();
        assert_eq!(parsed[0].value, ini, "{toml_value}");
    }
    for bad in ["[1, 2]", "{ a = 1 }", "1979-05-27"] {
        assert!(
            parse(&entry_toml("SGWEngine.ini", "A", "K", bad)).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn the_search_prefers_the_variable_then_the_state_dir_then_the_walk_up() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("state");
    let checkout = tmp.path().join("checkout");
    let deep = checkout.join("target").join("debug");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::create_dir_all(checkout.join("tools/lab")).unwrap();
    let starts = [deep.clone()];

    assert_eq!(overrides_path_from(None, &state, &starts), None);

    let walked = checkout.join("tools/lab").join(OVERRIDES_FILE);
    std::fs::write(&walked, "").unwrap();
    assert_eq!(
        overrides_path_from(None, &state, &starts),
        Some(walked.clone())
    );

    let in_state = state.join(OVERRIDES_FILE);
    std::fs::write(&in_state, "").unwrap();
    assert_eq!(
        overrides_path_from(Some(""), &state, &starts),
        Some(in_state)
    );

    assert_eq!(
        overrides_path_from(Some(r" D:\lab\mine.toml "), &state, &starts),
        Some(PathBuf::from(r"D:\lab\mine.toml"))
    );
}

#[test]
fn a_missing_target_file_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    apply_overrides(
        tmp.path(),
        "p2",
        &[ov(
            "SGWEngine.ini",
            "TextureStreaming",
            "HysteresisLimit",
            "99",
        )],
    );
    assert!(!tmp.path().join("SGWEngine.ini").exists());
}

#[test]
fn a_bom_is_set_aside_for_matching_and_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("SGWEngine.ini");
    // The section is the first line: with the BOM left on, its header would
    // not match and a second [A] would be appended.
    std::fs::write(&path, "\u{FEFF}[A]\r\nK=1\r\n").unwrap();
    apply_overrides(tmp.path(), "p2", &[ov("SGWEngine.ini", "A", "K", "2")]);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        b"\xEF\xBB\xBF[A]\r\nK=2\r\n".to_vec()
    );
}

#[test]
fn a_read_only_ini_is_made_writable_and_written_without_a_temp_left() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("SGWEngine.ini");
    std::fs::write(&path, "[A]\r\nK=1\r\n").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();

    apply_overrides(tmp.path(), "p2", &[ov("SGWEngine.ini", "A", "K", "2")]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[A]\r\nK=2\r\n");
    assert!(!std::fs::metadata(&path).unwrap().permissions().readonly());
    assert!(!tmp.path().join("SGWEngine.ini.tmp").exists());
}

#[test]
fn the_example_file_parses_to_nothing() {
    let example = include_str!("../../../../tools/lab/ini-overrides.example.toml");
    assert_eq!(parse(example).unwrap(), Vec::new());
}
