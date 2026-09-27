//! Literal pins for the organization models.
//!
//! Every enum value is checked twice: against a literal number here, and
//! against the token parsed out of `entities/defs/enumerations.xml` at test
//! time, so neither a typo in the model nor a drifted copy of the XML can
//! pass by agreeing with itself.

use std::collections::BTreeMap;

use super::org_text::{name_key, validate};
use super::*;

/// `name -> value` for one `<EnumName>ENUMERATION` block of
/// `entities/defs/enumerations.xml`.
fn xml_enum(enum_name: &str) -> BTreeMap<String, i64> {
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
    let block = &xml[start..end];
    let mut out = BTreeMap::new();
    for token in block.split("<Token>").skip(1) {
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

#[test]
fn org_type_matches_enumerations_xml() {
    assert_eq!(OrgType::Squad as u8, 0);
    assert_eq!(OrgType::Team as u8, 1);
    assert_eq!(OrgType::Command as u8, 2);
    let xml = xml_enum("EOrganizationType");
    assert_eq!(xml.len(), 3);
    assert_eq!(xml["EORG_TYPE_Squad"], OrgType::Squad as i64);
    assert_eq!(xml["EORG_TYPE_Team"], OrgType::Team as i64);
    assert_eq!(xml["EORG_TYPE_Command"], OrgType::Command as i64);
    // EPersistentOrganizationType agrees on the two persistent types.
    let pot = xml_enum("EPersistentOrganizationType");
    assert_eq!(pot["POT_Team"], 1);
    assert_eq!(pot["POT_Command"], 2);
}

#[test]
fn org_type_parse_and_persistence() {
    assert_eq!(OrgType::try_from(0u8), Ok(OrgType::Squad));
    assert_eq!(OrgType::try_from(2u8), Ok(OrgType::Command));
    assert_eq!(OrgType::try_from(3u8), Err(UnknownValue(3)));
    assert!(!OrgType::Squad.is_persistent());
    assert!(OrgType::Team.is_persistent());
    assert!(OrgType::Command.is_persistent());
}

#[test]
fn org_rank_matches_enumerations_xml() {
    let pins = [
        (OrgRank::NONE, 0u8, "EORG_RANK_None"),
        (OrgRank::INITIATE, 1, "EORG_RANK_Initiate"),
        (OrgRank::MEMBER, 2, "EORG_RANK_Member"),
        (OrgRank::SENIOR_MEMBER, 3, "EORG_RANK_SeniorMember"),
        (OrgRank::VETERAN, 4, "EORG_RANK_Veteran"),
        (OrgRank::SENIOR_VETERAN, 5, "EORG_RANK_SeniorVeteran"),
        (OrgRank::OFFICER, 6, "EORG_RANK_Officer"),
        (OrgRank::SENIOR_OFFICER, 7, "EORG_RANK_SeniorOfficer"),
        (OrgRank::LEADER, 8, "EORG_RANK_Leader"),
    ];
    let xml = xml_enum("EOrganizationRank");
    assert_eq!(xml.len(), pins.len());
    for (rank, literal, token) in pins {
        assert_eq!(rank.as_u8(), literal, "{token}");
        assert_eq!(xml[token], i64::from(literal), "{token}");
    }
}

#[test]
fn org_rank_parse_rejects_out_of_range() {
    assert_eq!(OrgRank::try_from(8u8), Ok(OrgRank::LEADER));
    assert_eq!(OrgRank::try_from(9u8), Err(UnknownValue(9)));
    assert_eq!(OrgRank::try_from(6i32), Ok(OrgRank::OFFICER));
    assert_eq!(OrgRank::try_from(-1i32), Err(UnknownValue(-1)));
    assert_eq!(OrgRank::try_from(264i32), Err(UnknownValue(264)));
}

/// D-ORG07. The literal ranks, not `OrgRank::for_type` compared to itself.
#[test]
fn ranks_for_type_follow_d_org07() {
    let raw = |t| {
        OrgRank::for_type(t)
            .iter()
            .map(|r| r.as_u8())
            .collect::<Vec<_>>()
    };
    assert_eq!(raw(OrgType::Squad), [2, 8]);
    assert_eq!(raw(OrgType::Team), [2, 3, 8]);
    assert_eq!(raw(OrgType::Command), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(OrgRank::entry_for(OrgType::Team).as_u8(), 2);
    assert_eq!(OrgRank::entry_for(OrgType::Command).as_u8(), 1);
    assert_eq!(OrgRank::entry_for(OrgType::Squad).as_u8(), 2);
    // D-ORG09 (5): 0 is never assignable, and Team has no rank 5.
    for t in OrgType::ALL {
        assert!(!OrgRank::NONE.is_valid_for(t));
    }
    assert!(!OrgRank::SENIOR_VETERAN.is_valid_for(OrgType::Team));
    assert!(OrgRank::LEADER > OrgRank::SENIOR_OFFICER);
}

#[test]
fn leave_reason_matches_enumerations_xml() {
    // PR #584 had disbanded 0 / left 1 / kicked 2 (audit A-30).
    assert_eq!(OrgLeaveReason::Requested.as_u8(), 0);
    assert_eq!(OrgLeaveReason::Kicked.as_u8(), 1);
    assert_eq!(OrgLeaveReason::Disbanded.as_u8(), 2);
    assert_eq!(OrgLeaveReason::Logout.as_u8(), 3);
    let xml = xml_enum("EReasons");
    assert_eq!(xml.len(), 4);
    assert_eq!(xml["REAS_requested"], 0);
    assert_eq!(xml["REAS_kicked"], 1);
    assert_eq!(xml["REAS_disbanded"], 2);
    assert_eq!(xml["REAS_logout"], 3);
    assert_eq!(OrgLeaveReason::try_from(3u8), Ok(OrgLeaveReason::Logout));
    assert_eq!(OrgLeaveReason::try_from(4u8), Err(UnknownValue(4)));
}

#[test]
fn squad_loot_type_matches_enumerations_xml() {
    assert_eq!(SquadLootType::RoundRobin.as_i32(), 0);
    assert_eq!(SquadLootType::FreeForAll.as_i32(), 1);
    let xml = xml_enum("EGroupLootType");
    assert_eq!(xml.len(), 2);
    assert_eq!(xml["GROUP_LOOT_RoundRobin"], 0);
    assert_eq!(xml["GROUP_LOOT_FreeForAll"], 1);
    assert_eq!(SquadLootType::try_from(1), Ok(SquadLootType::FreeForAll));
    assert_eq!(SquadLootType::try_from(2), Err(UnknownValue(2)));
    assert_eq!(SquadLootType::try_from(-1), Err(UnknownValue(-1)));
}

#[test]
fn permission_bits_match_enumerations_xml() {
    let pins = [
        (OrgPermission::DO_NOT_USE, 1u32, "EORG_PERM_DoNotUse"),
        (OrgPermission::INVITE, 2, "EORG_PERM_Invite"),
        (OrgPermission::PROMOTE, 4, "EORG_PERM_Promote"),
        (OrgPermission::DEMOTE, 8, "EORG_PERM_Demote"),
        (OrgPermission::EJECT, 16, "EORG_PERM_Eject"),
        (OrgPermission::ROSTER_NOTES, 32, "EORG_PERM_RosterNotes"),
        (OrgPermission::OFFICER_NOTES, 64, "EORG_PERM_OfficerNotes"),
        (OrgPermission::RANK_NAMES, 128, "EORG_PERM_RankNames"),
        (OrgPermission::OFFICER_CHAT, 256, "EORG_PERM_OfficerChat"),
        (OrgPermission::EMAIL_LISTS, 512, "EORG_PERM_EmailLists"),
        (OrgPermission::MOTD, 1024, "EORG_PERM_MOTD"),
        (OrgPermission::HISTORY_LOG, 2048, "EORG_PERM_HistoryLog"),
        (OrgPermission::CALENDAR, 4096, "EORG_PERM_Calendar"),
        (OrgPermission::RECRUIT_DESC, 8192, "EORG_PERM_RecruitDesc"),
        (OrgPermission::ADJECTIVES, 16384, "EORG_PERM_Adjectives"),
        (OrgPermission::INSIGNIA, 32768, "EORG_PERM_Insignia"),
        (OrgPermission::DEPOSIT_BANK, 65536, "EORG_PERM_DepositBank"),
        (
            OrgPermission::WITHDRAW_BANK,
            131072,
            "EORG_PERM_WithdrawBank",
        ),
        (OrgPermission::DEPOSIT_CASH, 262144, "EORG_PERM_DepositCash"),
        (
            OrgPermission::WITHDRAW_CASH,
            524288,
            "EORG_PERM_WithdrawCash",
        ),
        (
            OrgPermission::VIEW_BANK_LOGS,
            1048576,
            "EORG_PERM_ViewBankLogs",
        ),
        (OrgPermission::LEADER_CHAT, 2097152, "EORG_PERM_LeaderChat"),
        (
            OrgPermission::ALLIANCE_CHAT,
            4194304,
            "EORG_PERM_AllianceChat",
        ),
        (OrgPermission::ALTER_PERMS, 8388608, "EORG_PERM_AlterPerms"),
        (
            OrgPermission::TRANSFER_LEADER,
            16777216,
            "EORG_PERM_TransferLeader",
        ),
        (
            OrgPermission::ALLIANCE_CMDS,
            33554432,
            "EORG_PERM_AllianceCmds",
        ),
    ];
    let xml = xml_enum("EOrganizationPermission");
    assert_eq!(xml.len(), 26);
    let mut union = 0u32;
    for (perm, literal, token) in pins {
        assert_eq!(perm.bits(), literal, "{token}");
        assert_eq!(xml[token], i64::from(literal), "{token}");
        union |= literal;
    }
    assert_eq!(OrgPermission::ALL.bits(), 0x3FF_FFFF);
    assert_eq!(union, OrgPermission::ALL.bits());
}

#[test]
fn permission_wire_mask_drops_undefined_bits() {
    assert_eq!(OrgPermission::from_wire(-1), OrgPermission::ALL);
    assert_eq!(OrgPermission::from_wire(0x0400_0002).bits(), 2);
    assert_eq!(OrgPermission::ALL.to_wire(), 0x3FF_FFFF);
    assert_eq!(!OrgPermission::ALL, OrgPermission::NONE);
}

/// Audit A-12: the bits `Team.lua` and `Command.lua` put in their editors,
/// as literals. `TransferLeader` is in neither (D-ORG08).
#[test]
fn editable_bits_match_the_client_editors() {
    // Invite 2, Promote 4, Demote 8, Eject 16, OfficerNotes 64, RankNames
    // 128, MOTD 1024, DepositBank 65536, WithdrawBank 131072, DepositCash
    // 262144, WithdrawCash 524288, AlterPerms 8388608.
    let team = 2 + 4 + 8 + 16 + 64 + 128 + 1024 + 65536 + 131072 + 262144 + 524288 + 8388608;
    // Plus OfficerChat 256 and EmailLists 512.
    let command = team + 256 + 512;
    assert_eq!(OrgPermission::editable_for(OrgType::Team).bits(), team);
    assert_eq!(
        OrgPermission::editable_for(OrgType::Command).bits(),
        command
    );
    assert_eq!(
        OrgPermission::editable_for(OrgType::Team)
            .bits()
            .count_ones(),
        12
    );
    assert_eq!(
        OrgPermission::editable_for(OrgType::Command)
            .bits()
            .count_ones(),
        14
    );
    assert!(OrgPermission::editable_for(OrgType::Squad).is_empty());
    for t in OrgType::ALL {
        assert!(!OrgPermission::editable_for(t).contains(OrgPermission::TRANSFER_LEADER));
    }
}

/// D-ORG08 as amended by D-ORG21, as literal masks.
#[test]
fn default_rank_permissions_follow_d_org08_and_d_org21() {
    // D-ORG21: DepositBank 65536, DepositCash 262144, ViewBankLogs 1048576
    // on every rank below Leader.
    let bank = 65536 + 262144 + 1048576;
    assert_eq!(bank, 1_376_256);
    // Officer: bank + Invite 2, Eject 16, RosterNotes 32, OfficerNotes 64,
    // OfficerChat 256, MOTD 1024.
    let officer = 1_377_650;
    // Plus Promote 4, Demote 8, RankNames 128, AlterPerms 8388608.
    let senior_officer = 9_766_398;
    // bank + RosterNotes 32.
    let member = 1_376_288;
    let all = 0x3FF_FFFF;

    let rows = |t| {
        default_rank_permissions(t)
            .into_iter()
            .map(|(r, p)| (r.as_u8(), p.bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        rows(OrgType::Command),
        [
            (1, bank),
            (2, member),
            (3, member),
            (4, member),
            (5, member),
            (6, officer),
            (7, senior_officer),
            (8, all),
        ]
    );
    assert_eq!(rows(OrgType::Team), [(2, member), (3, officer), (8, all)]);
    assert!(rows(OrgType::Squad).is_empty());

    // Withdrawing is opt-in: WithdrawBank 131072 and WithdrawCash 524288
    // are on no default row below Leader.
    for t in [OrgType::Team, OrgType::Command] {
        for (rank, bits) in rows(t) {
            if rank < 8 {
                assert_eq!(bits & (131072 | 524288), 0, "{t:?} rank {rank}");
            }
        }
    }
}

// ── D-ORG10 text rules ──────────────────────────────────────────────────

#[test]
fn name_is_normalised_and_keyed() {
    assert_eq!(
        validate(TextField::Name, "  Tau'ri   Command  ").unwrap(),
        "Tau'ri Command"
    );
    assert_eq!(validate(TextField::Name, "SG-1.").unwrap(), "SG-1.");
    assert_eq!(name_key("Tau'ri Command"), "tau'ri command");
    // Two spellings of one name share a key.
    let a = validate(TextField::Name, "TAU'RI  command").unwrap();
    assert_eq!(name_key(&a), name_key("Tau'ri Command"));
}

#[test]
fn name_length_is_checked_after_normalisation() {
    assert_eq!(
        validate(TextField::Name, ""),
        Err(TextReject::TooShort { units: 0, min: 1 })
    );
    assert_eq!(
        validate(TextField::Name, "     "),
        Err(TextReject::TooShort { units: 0, min: 1 })
    );
    let sixty = "a".repeat(60);
    assert_eq!(validate(TextField::Name, &sixty).unwrap(), sixty);
    assert_eq!(
        validate(TextField::Name, &"a".repeat(61)),
        Err(TextReject::TooLong { units: 61, max: 60 })
    );
    // 61 raw characters that normalise to 60 are accepted.
    let padded = format!(" {sixty}");
    assert_eq!(validate(TextField::Name, &padded).unwrap(), sixty);
}

#[test]
fn name_rejects_outside_the_charset() {
    for (text, bad) in [
        ("Tau\u{2019}ri", '\u{2019}'), // typographic apostrophe
        ("Caf\u{E9}", '\u{E9}'),       // Latin-1 letter: ASCII only
        ("\u{212A}elvin", '\u{212A}'), // KELVIN SIGN, NFC would make it K
        ("\u{0421}G", '\u{0421}'),     // Cyrillic ES, a C homoglyph
        ("a_b", '_'),
        ("a\u{A0}b", '\u{A0}'), // no-break space
        ("<b>", '<'),
    ] {
        assert_eq!(
            validate(TextField::Name, text),
            Err(TextReject::NameCharset(bad)),
            "{text:?}"
        );
    }
}

#[test]
fn every_field_rejects_controls_bidi_and_zero_width() {
    let fields = [
        TextField::Name,
        TextField::Motd,
        TextField::Note,
        TextField::OfficerNote,
        TextField::RankName,
    ];
    for field in fields {
        for (text, want) in [
            ("a\u{0}b", TextReject::Control('\u{0}')),
            ("a\tb", TextReject::Control('\t')),
            ("a\rb", TextReject::Control('\r')),
            ("a\u{7F}b", TextReject::Control('\u{7F}')),
            ("a\u{85}b", TextReject::Control('\u{85}')),
            ("a\u{9F}b", TextReject::Control('\u{9F}')),
            ("a\u{202A}b", TextReject::Bidi('\u{202A}')),
            ("a\u{202E}b", TextReject::Bidi('\u{202E}')),
            ("a\u{2066}b", TextReject::Bidi('\u{2066}')),
            ("a\u{2069}b", TextReject::Bidi('\u{2069}')),
            ("a\u{200B}b", TextReject::ZeroWidth('\u{200B}')),
            ("a\u{200D}b", TextReject::ZeroWidth('\u{200D}')),
            ("a\u{FEFF}b", TextReject::ZeroWidth('\u{FEFF}')),
        ] {
            assert_eq!(validate(field, text), Err(want), "{field:?} {text:?}");
        }
    }
}

#[test]
fn newline_only_in_motd_and_notes() {
    for field in [TextField::Motd, TextField::Note, TextField::OfficerNote] {
        assert_eq!(validate(field, "line 1\nline 2").unwrap(), "line 1\nline 2");
    }
    for field in [TextField::Name, TextField::RankName] {
        assert_eq!(
            validate(field, "a\nb"),
            Err(TextReject::Control('\n')),
            "{field:?}"
        );
    }
}

#[test]
fn non_name_fields_keep_their_text_and_enforce_caps() {
    // Non-Latin text is fine outside names, and nothing is normalised.
    assert_eq!(
        validate(TextField::Motd, "  Chaa'ka  ÿ ").unwrap(),
        "  Chaa'ka  ÿ "
    );
    assert_eq!(validate(TextField::Motd, "").unwrap(), "");
    assert_eq!(validate(TextField::Note, "").unwrap(), "");
    for (field, max) in [
        (TextField::Motd, 255),
        (TextField::Note, 128),
        (TextField::OfficerNote, 128),
        (TextField::RankName, 32),
    ] {
        assert!(validate(field, &"x".repeat(max)).is_ok(), "{field:?}");
        assert_eq!(
            validate(field, &"x".repeat(max + 1)),
            Err(TextReject::TooLong {
                units: max + 1,
                max
            }),
            "{field:?}"
        );
    }
    assert_eq!(
        validate(TextField::RankName, ""),
        Err(TextReject::TooShort { units: 0, min: 1 })
    );
}

/// Caps count UTF-16 units, not chars: a supplementary-plane character is
/// two units.
#[test]
fn caps_count_utf16_units() {
    let emoji = "\u{1F680}"; // 2 UTF-16 units
    assert_eq!(
        validate(TextField::RankName, &emoji.repeat(17)),
        Err(TextReject::TooLong { units: 34, max: 32 })
    );
    assert!(validate(TextField::RankName, &emoji.repeat(16)).is_ok());
}

#[test]
fn id_constants_are_disjoint_bits() {
    assert_eq!(SQUAD_ORG_ID_MIN, 0x4000_0000);
    assert_eq!(BASE_INVITE_REQUEST_FLAG, 0x2000_0000);
    assert_eq!(MAX_ORG_ID, 0x3FFF_FFFF);
    assert_eq!(MAX_ORG_ID + 1, SQUAD_ORG_ID_MIN);
    assert_eq!(SQUAD_ORG_ID_MIN & BASE_INVITE_REQUEST_FLAG, 0);
    assert_eq!(MAX_SQUAD_SIZE, 6);
    assert_eq!(
        (MIN_NAME_UNITS, MAX_NAME_UNITS, MAX_MOTD_UNITS),
        (1, 60, 255)
    );
    assert_eq!(
        (MAX_NOTE_UNITS, MAX_OFFICER_NOTE_UNITS, MAX_RANK_NAME_UNITS),
        (128, 128, 32)
    );
}
