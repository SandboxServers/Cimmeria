//! Wire tests for the mail serializers and enum values.

use super::*;

#[test]
fn serialize_empty_mail_headers() {
    let args = serialize_on_mail_header_info(0, &[], &[]);
    // ResetCategory(1) + bArchive(1) + headers count(4) + attachments count(4)
    assert_eq!(args.len(), 1 + 1 + 4 + 4);
    assert_eq!(args[0], 0); // ResetCategory
    assert_eq!(args[1], 0); // bArchive
                            // Headers count = 0
    assert_eq!(u32::from_le_bytes([args[2], args[3], args[4], args[5]]), 0);
    // Attachments count = 0
    assert_eq!(u32::from_le_bytes([args[6], args[7], args[8], args[9]]), 0);
}

#[test]
fn serialize_one_mail_header() {
    let headers = vec![MailHeader {
        id: 42,
        from_text: "Bob".to_string(),
        from_id: 7,
        subject_text: "Hi".to_string(),
        cash: 100,
        sent_time: 1000.0,
        read_time: 0.0,
        flags: 0,
    }];
    let args = serialize_on_mail_header_info(1, &headers, &[]);

    // Verify basic structure
    assert_eq!(args[0], 0); // ResetCategory
    assert_eq!(args[1], 1); // bArchive

    // Headers count = 1
    let count = u32::from_le_bytes([args[2], args[3], args[4], args[5]]);
    assert_eq!(count, 1);

    // First header starts at offset 6
    let offset = 6;
    let id = i32::from_le_bytes([
        args[offset],
        args[offset + 1],
        args[offset + 2],
        args[offset + 3],
    ]);
    assert_eq!(id, 42);
}

/// Byte-exact `onMailHeaderInfo` with one header and its attachment:
/// the attachment array follows every header, each entry is the five
/// `alias.xml:103-111` INT32s in order, and `id` is the mail id.
#[test]
fn message_attachment_bytes_are_alias_ordered() {
    let headers = vec![MailHeader {
        id: 42,
        from_text: "Al".to_string(),
        from_id: 7,
        subject_text: "S".to_string(),
        cash: 500,
        sent_time: 2.0,
        read_time: 0.0,
        flags: 2,
    }];
    let attachments = [MailAttachment {
        id: 42,
        item_id: 1_234,
        stack_size: 5,
        durability: -1,
        charges: 3,
    }];
    let args = serialize_on_mail_header_info(0, &headers, &attachments);

    let mut want = vec![0u8, 0];
    want.extend_from_slice(&1u32.to_le_bytes());
    want.extend_from_slice(&42i32.to_le_bytes());
    want.extend_from_slice(&2u32.to_le_bytes());
    want.extend_from_slice(&[b'A', 0, b'l', 0]);
    want.extend_from_slice(&7i32.to_le_bytes());
    want.extend_from_slice(&1u32.to_le_bytes());
    want.extend_from_slice(&[b'S', 0]);
    want.extend_from_slice(&0i32.to_le_bytes()); // subjectId
    want.extend_from_slice(&500i32.to_le_bytes());
    want.extend_from_slice(&2.0f32.to_le_bytes());
    want.extend_from_slice(&0.0f32.to_le_bytes());
    want.extend_from_slice(&2i32.to_le_bytes());
    // MessageAttachments: count, then id, itemId, stackSize, durability, charges.
    want.extend_from_slice(&1u32.to_le_bytes());
    for v in [42i32, 1_234, 5, -1, 3] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(args, want);

    let mut one = Vec::new();
    attachments[0].serialize(&mut one);
    assert_eq!(one.len(), 20, "MessageAttachment is five INT32s");
}

#[test]
fn serialize_mail_read() {
    let args = serialize_on_mail_read(42, "Hello world", "Alice");

    // MailId
    let mail_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    assert_eq!(mail_id, 42);

    // BodyText WSTRING: char_count
    let text_len = u32::from_le_bytes([args[4], args[5], args[6], args[7]]);
    assert_eq!(text_len, 11); // "Hello world" = 11 chars
}

#[test]
fn serialize_mail_header_remove() {
    let args = serialize_on_mail_header_remove(99);
    assert_eq!(args.len(), 4);
    assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), 99);
}

// ── sendMailResult (SS-M1) ───────────────────────────────────────────────────

use super::codes::{flags, MailResult};

/// Byte-exact: `sendMailResult(Sent, ["Bob", "Al"], 0)` in `.def` order:
/// the result byte, the name array, then the flags.
#[test]
fn send_mail_result_bytes_are_def_ordered() {
    let args =
        serialize_send_mail_result(MailResult::Sent, &["Bob".to_string(), "Al".to_string()], 0);
    let expected: Vec<u8> = vec![
        0x00, // ResultCode = MAILRESULT_Sent
        0x02, 0x00, 0x00, 0x00, // FailedRecipients count
        0x03, 0x00, 0x00, 0x00, b'B', 0x00, b'o', 0x00, b'b', 0x00, // "Bob"
        0x02, 0x00, 0x00, 0x00, b'A', 0x00, b'l', 0x00, // "Al"
        0x00, 0x00, 0x00, 0x00, // FailedRecipientFlags
    ];
    assert_eq!(args, expected);
}

/// Byte-exact: a refusal with no names and the vault flag echoed back
/// (D-SS07's alias refusal shape).
#[test]
fn send_mail_result_bytes_with_flags_and_no_names() {
    let args = serialize_send_mail_result(MailResult::NoRecipients, &[], flags::MAIL_TO_VAULT);
    assert_eq!(
        args,
        vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]
    );
}

/// `name -> value` for one `<EnumName>ENUMERATION` block of
/// `entities/defs/enumerations.xml`, read at test time.
fn xml_enum(enum_name: &str) -> std::collections::BTreeMap<String, i64> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../entities/defs/enumerations.xml"
    );
    let xml = std::fs::read_to_string(path).expect("read enumerations.xml");
    let open = format!("<{enum_name}>");
    let close = format!("</{enum_name}>");
    let start = xml
        .find(&open)
        .unwrap_or_else(|| panic!("{enum_name} missing"));
    let end = start + xml[start..].find(&close).expect("unterminated enum");
    let mut out = std::collections::BTreeMap::new();
    for token in xml[start..end].split("<Token>").skip(1) {
        let field = |tag: &str| -> String {
            let a = token.find(&format!("<{tag}>")).unwrap() + tag.len() + 2;
            let b = token.find(&format!("</{tag}>")).unwrap();
            token[a..b].trim().to_owned()
        };
        out.insert(
            field("Name"),
            field("Value").parse().expect("numeric value"),
        );
    }
    out
}

/// Every `EMailFlags` constant equals the client's token, including the two
/// anomalies (4092 and 8196, audit A-12). A drifted constant, or a "fixed"
/// 4096, fails here.
#[test]
fn mail_flags_match_enumerations_xml() {
    let xml = xml_enum("EMailFlags");
    let ours = [
        ("MAIL_Archive", flags::MAIL_ARCHIVE),
        ("MAIL_COD", flags::MAIL_COD),
        ("MAIL_ToVault", flags::MAIL_TO_VAULT),
        ("MAIL_ToTeam", flags::MAIL_TO_TEAM),
        ("MAIL_ToCommand", flags::MAIL_TO_COMMAND),
        ("MAIL_ToCommandOfficers", flags::MAIL_TO_COMMAND_OFFICERS),
        ("MAIL_ToCommandRank0", flags::MAIL_TO_COMMAND_RANK0),
        ("MAIL_ToCommandRank1", flags::MAIL_TO_COMMAND_RANK1),
        ("MAIL_ToCommandRank2", flags::MAIL_TO_COMMAND_RANK2),
        ("MAIL_ToCommandRank3", flags::MAIL_TO_COMMAND_RANK3),
        ("MAIL_ToCommandRank4", flags::MAIL_TO_COMMAND_RANK4),
        ("MAIL_ToCommandRank5", flags::MAIL_TO_COMMAND_RANK5),
        ("MAIL_ToCommandRank6", flags::MAIL_TO_COMMAND_RANK6),
        ("MAIL_ToCommandRank7", flags::MAIL_TO_COMMAND_RANK7),
    ];
    assert_eq!(xml.len(), ours.len(), "EMailFlags gained or lost a token");
    for (name, value) in ours {
        assert_eq!(xml[name], i64::from(value), "{name}");
    }
    // Every `MAIL_To*` token is covered by one of the two alias masks.
    for (name, value) in &xml {
        if name.starts_with("MAIL_To") {
            let v = *value as i32;
            assert_eq!(
                v & (flags::VAULT_ALIASES | flags::ORGANIZATION_ALIASES),
                v,
                "{name} escapes the alias masks"
            );
        }
    }
}

/// Every `EMailResultCodes` value equals the client's token, and the enum
/// has no value the client does not.
#[test]
fn mail_result_codes_match_enumerations_xml() {
    let xml = xml_enum("EMailResultCodes");
    assert_eq!(xml.len(), MailResult::ALL.len());
    for r in MailResult::ALL {
        assert_eq!(xml[r.token()], i64::from(r.code()), "{}", r.token());
        assert_eq!(MailResult::try_from(r.code()), Ok(r));
    }
    assert_eq!(MailResult::try_from(8), Err(8));
}
