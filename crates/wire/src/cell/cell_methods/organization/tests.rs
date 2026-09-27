//! Decoder tests for the organization cell methods, one per method, each fed
//! the literal argument bytes a client sends (the payload after the 4-byte
//! entity id the base strips). CM 13, 14, 15, 17 and 94 used to stop after
//! the numeric fields and drop their `WSTRING`s (audit A-02); these tests
//! assert the text.

use super::*;

/// `WSTRING "Hi"`.
const WS_HI: [u8; 8] = [2, 0, 0, 0, 0x48, 0, 0x69, 0];

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn decode(idx: u16, args: &[u8]) -> OrgCellCall {
    decode_org_cell_method(idx, args).unwrap_or_else(|e| panic!("CM {idx}: {e}"))
}

#[test]
fn cm8_invite_response() {
    assert_eq!(
        decode(8, &[0x01, 0, 0, 0x20, 1]),
        OrgCellCall::InviteResponse {
            request_id: 0x2000_0001,
            response: 1
        }
    );
}

#[test]
fn cm9_leave() {
    assert_eq!(
        decode(9, &[1, 0, 0, 0x40]),
        OrgCellCall::Leave {
            org_id: 0x4000_0001
        }
    );
}

#[test]
fn cm10_broadcast_minimap_ping() {
    let args = cat(&[
        &[5, 0, 0, 0],
        &1.5f32.to_le_bytes(),
        &(-2.0f32).to_le_bytes(),
        &300.25f32.to_le_bytes(),
    ]);
    assert_eq!(
        decode(10, &args),
        OrgCellCall::BroadcastMinimapPing {
            org_id: 5,
            location: [1.5, -2.0, 300.25]
        }
    );
}

#[test]
fn cm11_strike_team_response() {
    assert_eq!(
        decode(11, &[5, 0, 0, 0, 1]),
        OrgCellCall::StrikeTeamResponse {
            org_id: 5,
            response: 1
        }
    );
}

#[test]
fn cm12_pvp_leave_response() {
    assert_eq!(
        decode(12, &[5, 0, 0, 0, 0]),
        OrgCellCall::PvpLeaveResponse {
            org_id: 5,
            response: 0
        }
    );
}

#[test]
fn cm13_motd_reads_its_wstring() {
    assert_eq!(
        decode(13, &cat(&[&[7, 0, 0, 0], &WS_HI])),
        OrgCellCall::Motd {
            org_id: 7,
            motd: "Hi".into()
        }
    );
}

#[test]
fn cm14_note_reads_its_wstring() {
    assert_eq!(
        decode(14, &cat(&[&[7, 0, 0, 0], &WS_HI])),
        OrgCellCall::Note {
            org_id: 7,
            note: "Hi".into()
        }
    );
}

#[test]
fn cm15_officer_note_reads_both_wstrings() {
    let args = cat(&[&[7, 0, 0, 0], &[1, 0, 0, 0, 0x42, 0], &WS_HI]);
    assert_eq!(
        decode(15, &args),
        OrgCellCall::OfficerNote {
            org_id: 7,
            name: "B".into(),
            note: "Hi".into()
        }
    );
}

#[test]
fn cm16_set_rank_permissions() {
    let args = cat(&[&[7, 0, 0, 0], &[6, 0, 0, 0], &[0x02, 0x04, 0, 0]]);
    assert_eq!(
        decode(16, &args),
        OrgCellCall::SetRankPermissions {
            org_id: 7,
            rank: 6,
            permissions: 0x402
        }
    );
}

#[test]
fn cm17_set_rank_name_reads_its_wstring() {
    let args = cat(&[&[7, 0, 0, 0], &[3, 0, 0, 0], &WS_HI]);
    assert_eq!(
        decode(17, &args),
        OrgCellCall::SetRankName {
            org_id: 7,
            rank: 3,
            name: "Hi".into()
        }
    );
}

#[test]
fn cm18_squad_set_loot_mode() {
    assert_eq!(
        decode(18, &[1, 0, 0, 0]),
        OrgCellCall::SquadSetLootMode { loot_mode: 1 }
    );
}

#[test]
fn cm19_transfer_cash_is_signed() {
    assert_eq!(
        decode(19, &[7, 0, 0, 0, 0x9C, 0xFF, 0xFF, 0xFF]),
        OrgCellCall::TransferCash {
            org_id: 7,
            amount: -100
        }
    );
}

#[test]
fn cm94_on_organization_creation_reads_the_name() {
    // "SG-1" as the client sends it: count 4, then UTF-16LE.
    let args = [4, 0, 0, 0, 0x53, 0, 0x47, 0, 0x2D, 0, 0x31, 0];
    assert_eq!(decode_on_organization_creation(&args).unwrap(), "SG-1");
    assert_eq!(decode_on_organization_creation(&[0, 0, 0, 0]).unwrap(), "");
}

// ── Rejections ───────────────────────────────────────────────────────────

/// A forged count is bounded by the bytes left before anything is
/// allocated: `0xFFFF_FFFF` units on a 2-byte tail is a truncation, not an
/// 8 GiB allocation.
#[test]
fn wstring_count_is_bounded_by_the_payload() {
    let args = cat(&[&[7, 0, 0, 0], &[0xFF, 0xFF, 0xFF, 0xFF], &[0x41, 0]]);
    assert_eq!(
        decode_org_cell_method(13, &args),
        Err(OrgDecodeError::Truncated {
            field: "aMOTD",
            need: 0x1_FFFF_FFFE,
            have: 2
        })
    );
    // One unit short.
    let args = cat(&[&[7, 0, 0, 0], &[2, 0, 0, 0], &[0x41, 0]]);
    assert!(matches!(
        decode_org_cell_method(14, &args),
        Err(OrgDecodeError::Truncated { field: "aNote", .. })
    ));
    // The officer note's second string is checked too.
    let args = cat(&[&[7, 0, 0, 0], &WS_HI, &[9, 0, 0, 0]]);
    assert!(matches!(
        decode_org_cell_method(15, &args),
        Err(OrgDecodeError::Truncated { field: "aNote", .. })
    ));
    assert!(matches!(
        decode_on_organization_creation(&[5, 0, 0, 0, 0x41, 0]),
        Err(OrgDecodeError::Truncated {
            field: "aOrganizationName",
            ..
        })
    ));
}

#[test]
fn numeric_fields_reject_short_payloads() {
    assert_eq!(
        decode_org_cell_method(8, &[1, 0, 0, 0]),
        Err(OrgDecodeError::Truncated {
            field: "aResponse",
            need: 1,
            have: 0
        })
    );
    assert!(matches!(
        decode_org_cell_method(10, &[5, 0, 0, 0, 0, 0, 0x80, 0x3F]),
        Err(OrgDecodeError::Truncated {
            field: "aLocation.y",
            ..
        })
    ));
    assert!(decode_org_cell_method(16, &[7, 0, 0, 0, 6, 0, 0, 0]).is_err());
}

#[test]
fn trailing_bytes_are_rejected() {
    assert_eq!(
        decode_org_cell_method(9, &[7, 0, 0, 0, 0xAA]),
        Err(OrgDecodeError::TrailingBytes { extra: 1 })
    );
    assert_eq!(
        decode_on_organization_creation(&cat(&[&WS_HI, &[0, 0]])),
        Err(OrgDecodeError::TrailingBytes { extra: 2 })
    );
}

/// An unpaired surrogate is a D-ORG10 text rejection, reported so the
/// handler can answer the player rather than drop the call.
#[test]
fn lone_surrogate_is_a_text_reject() {
    // A lone high surrogate U+D83D.
    let args = cat(&[&[7, 0, 0, 0], &[1, 0, 0, 0, 0x3D, 0xD8]]);
    let err = decode_org_cell_method(13, &args).unwrap_err();
    assert_eq!(err, OrgDecodeError::LoneSurrogate { field: "aMOTD" });
    assert_eq!(
        err.text_reject(),
        Some(cimmeria_entity::organization::TextReject::LoneSurrogate)
    );
    assert_eq!(
        OrgDecodeError::TrailingBytes { extra: 1 }.text_reject(),
        None
    );
}

#[test]
fn non_org_index_is_unknown() {
    assert_eq!(
        decode_org_cell_method(20, &[]),
        Err(OrgDecodeError::UnknownMethod(20))
    );
}

#[test]
fn org_id_accessor() {
    assert_eq!(decode(9, &[7, 0, 0, 0]).org_id(), Some(7));
    assert_eq!(decode(8, &[7, 0, 0, 0, 1]).org_id(), None);
    assert_eq!(decode(18, &[0, 0, 0, 0]).org_id(), None);
}
