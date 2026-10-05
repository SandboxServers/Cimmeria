//! The four exposed OrganizationMember **base** methods and their decoders.
//!
//! Argument order and widths are `OrganizationMember.def:418-449`. The
//! base-method wire id is `0xC0 + index` in the SGWPlayer base table, where
//! OrganizationMember holds indices 15-18
//! (`docs/protocol/sgwplayer-base-method-dispatch-table.md`).

use crate::cell::cell_methods::organization::{ArgReader, OrgDecodeError};

/// `organizationInvite(INT32 aOrganizationId, WSTRING aPlayerName)`.
pub const ORGANIZATION_INVITE: u8 = 0xCF;
/// `organizationInviteByType(UINT8 aOrganizationType, WSTRING aPlayerName)`.
pub const ORGANIZATION_INVITE_BY_TYPE: u8 = 0xD0;
/// `organizationKick(INT32 aOrganizationId, WSTRING aPlayerName)`.
pub const ORGANIZATION_KICK: u8 = 0xD1;
/// `organizationRankChange(INT32 aOrganizationId, WSTRING aPlayerName,
/// UINT8 aRank)`.
pub const ORGANIZATION_RANK_CHANGE: u8 = 0xD2;

/// One decoded organization base method. Values are raw: the type and rank
/// bytes are range-checked by the handlers (ORG-03, ORG-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgBaseCall {
    /// 0xCF: invite `player_name` into `org_id`.
    Invite { org_id: i32, player_name: String },
    /// 0xD0: invite `player_name` into the caller's organization of
    /// `org_type`. Type 0 may create a squad; 1 and 2 never create (CAT-M-02).
    InviteByType { org_type: u8, player_name: String },
    /// 0xD1: kick `player_name` from `org_id`.
    Kick { org_id: i32, player_name: String },
    /// 0xD2: set `player_name`'s rank in `org_id`.
    RankChange {
        org_id: i32,
        player_name: String,
        rank: u8,
    },
}

impl OrgBaseCall {
    /// The base-method message id the call arrived as (`0xC0 + index`).
    pub fn msg_id(&self) -> u8 {
        match self {
            OrgBaseCall::Invite { .. } => ORGANIZATION_INVITE,
            OrgBaseCall::InviteByType { .. } => ORGANIZATION_INVITE_BY_TYPE,
            OrgBaseCall::Kick { .. } => ORGANIZATION_KICK,
            OrgBaseCall::RankChange { .. } => ORGANIZATION_RANK_CHANGE,
        }
    }

    /// The `.def` method name, for logs, from the generated table
    /// (`crate::names`).
    pub fn method_name(&self) -> &'static str {
        crate::names::player_base_method(u16::from(self.msg_id() - 0xC0)).unwrap_or("unknown")
    }
}

/// `true` for the four organization base-method ids.
pub fn is_org_base_method(msg_id: u8) -> bool {
    (ORGANIZATION_INVITE..=ORGANIZATION_RANK_CHANGE).contains(&msg_id)
}

/// Decode an organization base method. `msg_id` outside 0xCF-0xD2 is
/// [`OrgDecodeError::UnknownMethod`].
pub fn decode_org_base_method(msg_id: u8, payload: &[u8]) -> Result<OrgBaseCall, OrgDecodeError> {
    let mut r = ArgReader::new(payload);
    let call = match msg_id {
        ORGANIZATION_INVITE => OrgBaseCall::Invite {
            org_id: r.i32("aOrganizationId")?,
            player_name: r.wstring("aPlayerName")?,
        },
        ORGANIZATION_INVITE_BY_TYPE => OrgBaseCall::InviteByType {
            org_type: r.u8("aOrganizationType")?,
            player_name: r.wstring("aPlayerName")?,
        },
        ORGANIZATION_KICK => OrgBaseCall::Kick {
            org_id: r.i32("aOrganizationId")?,
            player_name: r.wstring("aPlayerName")?,
        },
        ORGANIZATION_RANK_CHANGE => OrgBaseCall::RankChange {
            org_id: r.i32("aOrganizationId")?,
            player_name: r.wstring("aPlayerName")?,
            rank: r.u8("aRank")?,
        },
        other => return Err(OrgDecodeError::UnknownMethod(u16::from(other))),
    };
    r.finish()?;
    Ok(call)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `WSTRING "Bo"`.
    const WS_BO: [u8; 8] = [2, 0, 0, 0, 0x42, 0, 0x6F, 0];

    fn cat(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    /// Base-table indices 15-18 at `0xC0 + index`.
    #[test]
    fn ids_are_0xcf_to_0xd2() {
        assert_eq!(ORGANIZATION_INVITE, 0xC0 + 15);
        assert_eq!(ORGANIZATION_INVITE_BY_TYPE, 0xC0 + 16);
        assert_eq!(ORGANIZATION_KICK, 0xC0 + 17);
        assert_eq!(ORGANIZATION_RANK_CHANGE, 0xC0 + 18);
        assert!(is_org_base_method(0xCF) && is_org_base_method(0xD2));
        assert!(!is_org_base_method(0xCE) && !is_org_base_method(0xD3));
    }

    /// Id first, then the name (the old dispatch table had the name first).
    #[test]
    fn invite_decodes_id_then_name() {
        assert_eq!(
            decode_org_base_method(0xCF, &cat(&[&[9, 0, 0, 0], &WS_BO])),
            Ok(OrgBaseCall::Invite {
                org_id: 9,
                player_name: "Bo".into()
            })
        );
    }

    /// The type is one byte, not an `INT32`.
    #[test]
    fn invite_by_type_decodes_a_uint8_type() {
        assert_eq!(
            decode_org_base_method(0xD0, &cat(&[&[2], &WS_BO])),
            Ok(OrgBaseCall::InviteByType {
                org_type: 2,
                player_name: "Bo".into()
            })
        );
        // A 4-byte type would leave the name misaligned; it must not decode.
        assert!(decode_org_base_method(0xD0, &cat(&[&[2, 0, 0, 0], &WS_BO])).is_err());
    }

    #[test]
    fn kick_decodes_id_then_name() {
        assert_eq!(
            decode_org_base_method(0xD1, &cat(&[&[0, 0, 0, 0x40], &WS_BO])),
            Ok(OrgBaseCall::Kick {
                org_id: 0x4000_0000,
                player_name: "Bo".into()
            })
        );
    }

    #[test]
    fn rank_change_decodes_id_name_then_uint8_rank() {
        assert_eq!(
            decode_org_base_method(0xD2, &cat(&[&[9, 0, 0, 0], &WS_BO, &[6]])),
            Ok(OrgBaseCall::RankChange {
                org_id: 9,
                player_name: "Bo".into(),
                rank: 6
            })
        );
    }

    #[test]
    fn malformed_payloads_are_rejected() {
        // Forged name length.
        assert!(matches!(
            decode_org_base_method(0xCF, &cat(&[&[9, 0, 0, 0], &[0xFF, 0xFF, 0, 0]])),
            Err(OrgDecodeError::Truncated {
                field: "aPlayerName",
                ..
            })
        ));
        // Missing rank byte.
        assert_eq!(
            decode_org_base_method(0xD2, &cat(&[&[9, 0, 0, 0], &WS_BO])),
            Err(OrgDecodeError::Truncated {
                field: "aRank",
                need: 1,
                have: 0
            })
        );
        assert_eq!(
            decode_org_base_method(0xD1, &cat(&[&[9, 0, 0, 0], &WS_BO, &[0]])),
            Err(OrgDecodeError::TrailingBytes { extra: 1 })
        );
        assert_eq!(
            decode_org_base_method(0xD3, &[]),
            Err(OrgDecodeError::UnknownMethod(0xD3))
        );
    }
}
