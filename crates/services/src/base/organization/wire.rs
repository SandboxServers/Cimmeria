//! Server → Client wire serializers for the OrganizationMember interface.
//!
//! All builders return a `Vec<u8>` of the raw args payload (no method-index
//! byte, no length prefix).  Method indices live in `crate::mercury::method_idx`.
//!
//! Wire formats verified against:
//! - `entities/defs/interfaces/OrganizationMember.def`
//! - `docs/reverse-engineering/findings/organization-wire-formats.md`

use crate::cell::social::squad_manager::RosterEntry;

// ── WSTRING helpers ───────────────────────────────────────────────────────────

/// Append a WSTRING (u32 char-count + N×2B UTF-16LE) to `buf`.
fn push_wstring(buf: &mut Vec<u8>, s: &str) {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
    for &ch in &utf16 {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
}

// ── CM 34: onOrganizationInvite ───────────────────────────────────────────────

/// `onOrganizationInvite(aInviterName WSTRING, aOrganizationType UINT8,
///                       aRequestID INT32, aName WSTRING, aIsStrikeTeam UINT8)`
///
/// Sent to the *target* of an invite.
///
/// Called from the Phase 3 `organizationInviteByType` base-method handler.
#[allow(dead_code)] // Phase 3: used by organizationInviteByType base handler
pub fn build_on_organization_invite(
    inviter_name: &str,
    org_type: u8,
    request_id: i32,
    org_name: &str,
    is_strike_team: bool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    push_wstring(&mut buf, inviter_name);
    buf.push(org_type);
    buf.extend_from_slice(&request_id.to_le_bytes());
    push_wstring(&mut buf, org_name);
    buf.push(u8::from(is_strike_team));
    buf
}

// ── CM 35: onOrganizationJoined ───────────────────────────────────────────────

/// `onOrganizationJoined(aOrganizationId INT32, aOrganizationType UINT8,
///                       aRank UINT8, aNewMember UINT8)`
///
/// Sent to the player who just joined.
pub fn build_on_organization_joined(
    org_id: i32,
    org_type: u8,
    rank: u8,
    is_new_member: bool,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(7);
    buf.extend_from_slice(&org_id.to_le_bytes());
    buf.push(org_type);
    buf.push(rank);
    buf.push(u8::from(is_new_member));
    buf
}

// ── CM 36: onOrganizationLeft ────────────────────────────────────────────────

/// `onOrganizationLeft(aReason UINT8, aOrganizationId INT32)`
///
/// Sent to the player who left/was kicked.
///
/// UNCONFIRMED reason enum values — x64dbg needed.
/// Using: 0=Disbanded, 1=Left, 2=Kicked (see `e_reasons` in squad_manager).
pub fn build_on_organization_left(reason: u8, org_id: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    buf.push(reason);
    buf.extend_from_slice(&org_id.to_le_bytes());
    buf
}

// ── CM 37: onMemberJoinedOrganization ────────────────────────────────────────

/// `onMemberJoinedOrganization(aMemberName WSTRING, aMember INT32,
///                              aOrganizationId INT32, aRank UINT8,
///                              aNewMember UINT8)`
///
/// Fan out to *existing* members when someone new joins.
pub fn build_on_member_joined_organization(
    member_name: &str,
    member_player_id: i32,
    org_id: i32,
    rank: u8,
    is_new_member: bool,
) -> Vec<u8> {
    let mut buf = Vec::new();
    push_wstring(&mut buf, member_name);
    buf.extend_from_slice(&member_player_id.to_le_bytes());
    buf.extend_from_slice(&org_id.to_le_bytes());
    buf.push(rank);
    buf.push(u8::from(is_new_member));
    buf
}

// ── CM 38: onOrganizationRosterInfo ──────────────────────────────────────────

/// `onOrganizationRosterInfo(aOrganizationId INT32,
///                            aRosterInfo ARRAY<RosterInfo>)`
///
/// Sent to a player on join to give them the full roster.
///
/// `RosterInfo` FIXED_DICT layout (per wire-formats.md):
/// - name:         WSTRING
/// - level:        UINT8
/// - archetype:    UINT8
/// - rank:         UINT8
/// - note:         WSTRING
/// - officerNote:  WSTRING
///
/// Open Q: does `RosterInfo` include an `isOnline` bool?  Not in the .def.
/// Using the .def layout; needs x64dbg confirmation.
pub fn build_on_organization_roster_info(org_id: i32, roster: &[RosterEntry]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&org_id.to_le_bytes());
    buf.extend_from_slice(&(roster.len() as u32).to_le_bytes());
    for entry in roster {
        push_wstring(&mut buf, &entry.name);
        buf.push(entry.level);
        buf.push(entry.archetype);
        buf.push(entry.rank);
        push_wstring(&mut buf, &entry.note);
        push_wstring(&mut buf, &entry.officer_note);
    }
    buf
}

// ── CM 39: onMemberLeftOrganization ──────────────────────────────────────────

/// `onMemberLeftOrganization(aMember INT32, aReason UINT8,
///                            aOrganizationId INT32, aMemberName WSTRING)`
///
/// Fan out to remaining members when someone leaves/is kicked.
pub fn build_on_member_left_organization(
    member_player_id: i32,
    reason: u8,
    org_id: i32,
    member_name: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&member_player_id.to_le_bytes());
    buf.push(reason);
    buf.extend_from_slice(&org_id.to_le_bytes());
    push_wstring(&mut buf, member_name);
    buf
}

// ── CM 51: onSquadLootType ────────────────────────────────────────────────────

/// `onSquadLootType(aOrganizationId INT32, aLootType INT32)`
///
/// Fan out to all squad members when loot mode changes.
pub fn build_on_squad_loot_type(org_id: i32, loot_type: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.extend_from_slice(&org_id.to_le_bytes());
    buf.extend_from_slice(&loot_type.to_le_bytes());
    buf
}

// ── CM 40: onMemberRankChangedOrganization ────────────────────────────────────

/// `onMemberRankChangedOrganization(aMember INT32, aRank UINT8,
///                                   aOrganizationId INT32,
///                                   aMemberName WSTRING)`
///
/// Broadcast to all online org members when a rank changes.
pub fn build_on_member_rank_changed_organization(
    member_player_id: i32,
    rank: u8,
    org_id: i32,
    member_name: &str,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&member_player_id.to_le_bytes());
    buf.push(rank);
    buf.extend_from_slice(&org_id.to_le_bytes());
    push_wstring(&mut buf, member_name);
    buf
}

// ── CM 45: onOrganizationMOTDUpdate ──────────────────────────────────────────

/// `onOrganizationMOTDUpdate(aOrganizationId INT32, aMOTD WSTRING)`
///
/// Broadcast to all online org members when the MOTD changes.
pub fn build_on_organization_motd_update(org_id: i32, motd: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&org_id.to_le_bytes());
    push_wstring(&mut buf, motd);
    buf
}

// ── SGWPlayer CM 134: onOrganizationCreationResult ───────────────────────────

/// `onOrganizationCreationResult(Result UINT8, RetCode UINT8)`
///
/// Sent to the player who attempted to create an org.
/// `result` 0 = success, non-zero = failure. `ret_code` is a more specific
/// reason code (values UNCONFIRMED — needs x64dbg).
///
/// Method 134 is a SGWPlayer own method (base offset 98 + offset within own
/// list), not an OrganizationMember interface method. It uses the SGWPlayer
/// extended-encoding path.
pub fn build_on_organization_creation_result(result: u8, ret_code: u8) -> Vec<u8> {
    vec![result, ret_code]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::social::squad_manager::RosterEntry;

    // Helper: decode a WSTRING starting at `offset`, return (string, next_offset).
    fn decode_wstring(buf: &[u8], offset: usize) -> (String, usize) {
        let char_count = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]) as usize;
        let byte_start = offset + 4;
        let byte_end = byte_start + char_count * 2;
        let utf16: Vec<u16> = (byte_start..byte_end)
            .step_by(2)
            .map(|i| u16::from_le_bytes([buf[i], buf[i + 1]]))
            .collect();
        (String::from_utf16_lossy(&utf16).to_owned(), byte_end)
    }

    // ── CM 34: onOrganizationInvite ───────────────────────────────────────────

    #[test]
    fn on_organization_invite_layout() {
        let buf = build_on_organization_invite("Alice", 0, 42, "Alpha Squad", false);

        let (inviter_name, off) = decode_wstring(&buf, 0);
        assert_eq!(inviter_name, "Alice");

        let org_type = buf[off];
        // OrgType::Squad = 0
        assert_eq!(org_type, 0, "org_type wire pin");
        let off = off + 1;

        let request_id = i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(request_id, 42, "request_id literal pin");
        let off = off + 4;

        let (org_name, off) = decode_wstring(&buf, off);
        assert_eq!(org_name, "Alpha Squad");

        let is_strike_team = buf[off];
        assert_eq!(is_strike_team, 0, "is_strike_team wire pin (false=0)");
        assert_eq!(off + 1, buf.len(), "no trailing bytes");
    }

    #[test]
    fn on_organization_invite_strike_team_flag() {
        let buf = build_on_organization_invite("Bob", 0, 1, "Strike", true);
        // is_strike_team is the last byte
        assert_eq!(*buf.last().unwrap(), 1, "is_strike_team true=1 literal pin");
    }

    // ── CM 35: onOrganizationJoined ──────────────────────────────────────────

    #[test]
    fn on_organization_joined_layout_and_size() {
        let buf = build_on_organization_joined(7, 0, 2, true);
        // INT32 + UINT8 + UINT8 + UINT8 = 7 bytes
        assert_eq!(buf.len(), 7, "total size literal pin");

        let org_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(org_id, 7, "org_id literal pin");
        assert_eq!(buf[4], 0, "org_type Squad=0 literal pin");
        assert_eq!(buf[5], 2, "rank Member=2 literal pin");
        assert_eq!(buf[6], 1, "is_new_member true=1 literal pin");
    }

    // ── CM 36: onOrganizationLeft ─────────────────────────────────────────────

    #[test]
    fn on_organization_left_layout_and_size() {
        let buf = build_on_organization_left(1, 99);
        // UINT8 + INT32 = 5 bytes
        assert_eq!(buf.len(), 5, "total size literal pin");
        assert_eq!(buf[0], 1, "reason LEFT=1 literal pin");
        let org_id = i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);
        assert_eq!(org_id, 99, "org_id literal pin");
    }

    #[test]
    fn on_organization_left_disbanded() {
        let buf = build_on_organization_left(0, 5);
        assert_eq!(buf[0], 0, "reason DISBANDED=0 literal pin");
    }

    // ── CM 37: onMemberJoinedOrganization ────────────────────────────────────

    #[test]
    fn on_member_joined_organization_layout() {
        let buf = build_on_member_joined_organization("Bob", 200, 7, 2, true);

        let (name, off) = decode_wstring(&buf, 0);
        assert_eq!(name, "Bob");

        let member_id = i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(member_id, 200, "member_id literal pin");
        let off = off + 4;

        let org_id = i32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(org_id, 7, "org_id literal pin");
        let off = off + 4;

        assert_eq!(buf[off], 2, "rank Member=2 literal pin");
        assert_eq!(buf[off + 1], 1, "is_new_member true=1 literal pin");
        assert_eq!(off + 2, buf.len(), "no trailing bytes");
    }

    // ── CM 38: onOrganizationRosterInfo ──────────────────────────────────────

    #[test]
    fn on_organization_roster_info_empty_roster() {
        let buf = build_on_organization_roster_info(3, &[]);
        // INT32 org_id + u32 count(0) = 8 bytes
        assert_eq!(buf.len(), 8, "empty roster size literal pin");
        let org_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(org_id, 3, "org_id literal pin");
        let count = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(count, 0, "count=0 literal pin");
    }

    #[test]
    fn on_organization_roster_info_one_member() {
        let entry = RosterEntry {
            name: "Alice".into(),
            level: 10,
            archetype: 1,
            rank: 8,
            note: String::new(),
            officer_note: String::new(),
        };
        let buf = build_on_organization_roster_info(5, &[entry]);

        let org_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(org_id, 5, "org_id literal pin");
        let count = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(count, 1, "count=1 literal pin");

        let off = 8;
        let (name, off) = decode_wstring(&buf, off);
        assert_eq!(name, "Alice");
        assert_eq!(buf[off], 10, "level literal pin");
        assert_eq!(buf[off + 1], 1, "archetype literal pin");
        assert_eq!(buf[off + 2], 8, "rank Leader=8 literal pin");
        let off = off + 3;
        // note (empty): 4-byte count = 0
        let note_len = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(note_len, 0, "note empty literal pin");
        let off = off + 4;
        let officer_note_len =
            u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
        assert_eq!(officer_note_len, 0, "officer_note empty literal pin");
        assert_eq!(off + 4, buf.len(), "no trailing bytes");
    }

    // ── CM 39: onMemberLeftOrganization ──────────────────────────────────────

    #[test]
    fn on_member_left_organization_layout() {
        let buf = build_on_member_left_organization(300, 1, 7, "Carol");

        let member_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(member_id, 300, "member_id literal pin");
        assert_eq!(buf[4], 1, "reason LEFT=1 literal pin");
        let org_id = i32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]);
        assert_eq!(org_id, 7, "org_id literal pin");
        let (name, end) = decode_wstring(&buf, 9);
        assert_eq!(name, "Carol");
        assert_eq!(end, buf.len(), "no trailing bytes");
    }

    // ── CM 51: onSquadLootType ────────────────────────────────────────────────

    #[test]
    fn on_squad_loot_type_layout_and_size() {
        let buf = build_on_squad_loot_type(7, 2);
        // INT32 + INT32 = 8 bytes
        assert_eq!(buf.len(), 8, "total size literal pin");
        let org_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(org_id, 7, "org_id literal pin");
        let loot_type = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(loot_type, 2, "loot_type literal pin");
    }

    // ── C→S parser tests (CM 8/9/10/18) ─────────────────────────────────────

    /// CM 8 — organizationInviteResponse: INT32 request_id + UINT8 response.
    #[test]
    fn parse_cm8_invite_response_accept() {
        let mut args = Vec::new();
        args.extend_from_slice(&42i32.to_le_bytes()); // request_id
        args.push(1u8); // accept

        assert_eq!(args.len(), 5);
        let request_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let accepted = args[4] != 0;
        assert_eq!(request_id, 42, "request_id literal pin");
        assert!(accepted, "non-zero = accept");
    }

    #[test]
    fn parse_cm8_invite_response_decline() {
        let mut args = Vec::new();
        args.extend_from_slice(&99i32.to_le_bytes());
        args.push(0u8); // decline

        let request_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let accepted = args[4] != 0;
        assert_eq!(request_id, 99, "request_id literal pin");
        assert!(!accepted, "zero = decline");
    }

    /// CM 9 — organizationLeave: INT32 org_id.
    #[test]
    fn parse_cm9_organization_leave() {
        let args = 77i32.to_le_bytes();
        assert_eq!(args.len(), 4);
        let org_id = i32::from_le_bytes(args);
        assert_eq!(org_id, 77, "org_id literal pin");
    }

    /// CM 10 — BroadcastMinimapPing: INT32 org_id + VECTOR3 (3×f32 LE).
    #[test]
    fn parse_cm10_minimap_ping() {
        let mut args = Vec::new();
        args.extend_from_slice(&5i32.to_le_bytes()); // org_id
        args.extend_from_slice(&100.0f32.to_le_bytes()); // x
        args.extend_from_slice(&200.0f32.to_le_bytes()); // y
        args.extend_from_slice(&300.0f32.to_le_bytes()); // z

        assert_eq!(args.len(), 16, "total size literal pin");
        let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        assert_eq!(org_id, 5, "org_id literal pin");
        let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        assert!((x - 100.0).abs() < f32::EPSILON, "x literal pin");
        let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
        assert!((y - 200.0).abs() < f32::EPSILON, "y literal pin");
        let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
        assert!((z - 300.0).abs() < f32::EPSILON, "z literal pin");
    }

    /// CM 18 — squadSetLootMode: INT32 loot_mode.
    #[test]
    fn parse_cm18_squad_set_loot_mode() {
        let args = 3i32.to_le_bytes();
        assert_eq!(args.len(), 4, "size literal pin");
        let loot_mode = i32::from_le_bytes(args);
        assert_eq!(loot_mode, 3, "loot_mode literal pin");
    }

    // ── Phase 3 S→C wire tests ─────────────────────────────────────────────

    /// CM 40 — onMemberRankChangedOrganization:
    /// INT32 member_id + UINT8 rank + INT32 org_id + WSTRING name.
    #[test]
    fn on_member_rank_changed_organization_layout() {
        let buf = super::build_on_member_rank_changed_organization(500, 6, 12, "Officer");

        let member_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(member_id, 500, "member_id literal pin");
        assert_eq!(buf[4], 6, "rank Officer=6 literal pin");
        let org_id = i32::from_le_bytes([buf[5], buf[6], buf[7], buf[8]]);
        assert_eq!(org_id, 12, "org_id literal pin");
        let (name, end) = decode_wstring(&buf, 9);
        assert_eq!(name, "Officer", "name literal pin");
        assert_eq!(end, buf.len(), "no trailing bytes");
    }

    /// CM 45 — onOrganizationMOTDUpdate: INT32 org_id + WSTRING motd.
    #[test]
    fn on_organization_motd_update_layout() {
        let buf = super::build_on_organization_motd_update(7, "Welcome");

        let org_id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(org_id, 7, "org_id literal pin");
        let (motd, end) = decode_wstring(&buf, 4);
        assert_eq!(motd, "Welcome", "motd literal pin");
        // decode_wstring returns the offset *after* the wstring — must equal buf.len()
        assert_eq!(end, buf.len(), "no trailing bytes");
    }

    /// CM 45 — empty MOTD (clear) encodes as zero-length WSTRING.
    #[test]
    fn on_organization_motd_update_empty_motd() {
        let buf = super::build_on_organization_motd_update(3, "");
        // INT32 (4) + u32 char_count=0 (4) = 8 bytes total.
        assert_eq!(buf.len(), 8, "empty MOTD must be 8 bytes");
        let char_count = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(char_count, 0, "empty MOTD char_count must be 0");
    }

    /// SGWPlayer CM 134 — onOrganizationCreationResult: UINT8 result + UINT8 ret_code.
    #[test]
    fn on_organization_creation_result_layout() {
        let buf = super::build_on_organization_creation_result(0, 0);
        assert_eq!(buf.len(), 2, "size literal pin: 2 bytes");
        assert_eq!(buf[0], 0, "result=0 (success) literal pin");
        assert_eq!(buf[1], 0, "ret_code=0 literal pin");
    }

    #[test]
    fn on_organization_creation_result_failure() {
        let buf = super::build_on_organization_creation_result(1, 2);
        assert_eq!(buf[0], 1, "result=1 (failure) literal pin");
        assert_eq!(buf[1], 2, "ret_code=2 literal pin");
    }

    /// C→S CM 13 — organizationMOTD: INT32 org_id + WSTRING motd.
    #[test]
    fn parse_cm13_organization_motd() {
        let mut args = Vec::new();
        args.extend_from_slice(&42i32.to_le_bytes()); // org_id
        let motd = "Hello world";
        let utf16: Vec<u16> = motd.encode_utf16().collect();
        args.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
        for ch in &utf16 {
            args.extend_from_slice(&ch.to_le_bytes());
        }

        assert_eq!(args.len(), 4 + 4 + motd.len() * 2, "size literal pin");
        let org_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        assert_eq!(org_id, 42, "org_id literal pin");

        let (parsed_motd, _) = decode_wstring(&args, 4);
        assert_eq!(parsed_motd, motd, "MOTD round-trip literal pin");
    }
}
