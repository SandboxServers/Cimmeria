use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Re-assert the document-order count of `<Exposed/>` CellMethods in
/// `SGWGmPlayer.def`. The first own exposed method (gmMissionAssign, def
/// line 65) is index 109; counting forward in document order — skipping
/// `gmSetCallback` (def line 312, which has NO `<Exposed/>`) — lands each
/// implemented method at the constant below. If this drifts, the client's
/// method table and our dispatch disagree and gm* commands silently route
/// to the wrong handler. The three pcap-anchored DONE indices
/// (133/163/190) are the alignment proof for the whole tail.
#[test]
fn gm_indices_match_def_document_order() {
    // index = 109 + zero-based document-order position among exposed methods.
    assert_eq!(GM_MISSION_CLEAR, 109 + 1, "gmMissionClear (def line 71)");
    assert_eq!(
        GM_MISSION_ADVANCE,
        109 + 7,
        "gmMissionAdvance (def line 97)"
    );
    assert_eq!(
        GM_MISSION_ABANDON,
        109 + 11,
        "gmMissionAbandon (def line 120)"
    );
    assert_eq!(GM_GIVE_XP, 109 + 23, "gmGiveXp (def line 180)");
    assert_eq!(GM_GIVE_ITEM, 109 + 24, "gmGiveItem (def line 185)");
    assert_eq!(GM_GIVE_CASH, 109 + 25, "gmGiveCash (def line 191)");
    assert_eq!(GM_REMOVE_ITEM, 109 + 26, "gmRemoveItem (def line 196)");
    assert_eq!(GM_GIVE_EXPERTISE, 109 + 30, "gmGiveExpertise (offset 30)");
    assert_eq!(
        GM_GIVE_APPLIED_SCIENCE_POINTS,
        109 + 31,
        "gmGiveAppliedSciencePoints (offset 31)"
    );
    assert_eq!(GM_SPAWN_BY_CMD, 109 + 76, "gmSpawnByCmd (offset 76)");
    assert_eq!(GM_SET_HEALTH, 109 + 38, "gmSetHealth (def line 259)");
    assert_eq!(GM_SET_HEALTH_MAX, 109 + 39, "gmSetHealthMax (def line 265)");
    assert_eq!(GM_SET_FOCUS, 109 + 40, "gmSetFocus (def line 271)");
    assert_eq!(GM_SET_FOCUS_MAX, 109 + 41, "gmSetFocusMax (def line 277)");
    assert_eq!(GM_SET_TARGET, 109 + 47, "gmSetTarget (def line 302)");
    assert_eq!(GM_DHD, 109 + 50, "gmDHD (def line 325)");
    assert_eq!(GM_USERS, 109 + 57, "gmUsers (def line 363)");
    assert_eq!(TEST_LOS, 109 + 107, "testLOS (def line 619)");
    assert_eq!(
        GM_SHOW_TARGET_LOCATION,
        109 + 12,
        "gmShowTargetLocation (def line 127)"
    );
    assert_eq!(GM_SHOW_ROTATION, 109 + 13, "gmShowRotation (def line 131)");
    assert_eq!(GM_SHOW_PLAYER, 109 + 22, "gmShowPlayer (def line 174)");
    assert_eq!(GM_MISSION_ASSIGN, 109, "gmMissionAssign (def line 65)");
    assert_eq!(GM_MISSION_LIST, 109 + 4, "gmMissionList (def line 84)");
    assert_eq!(
        GM_MISSION_LIST_FULL,
        109 + 5,
        "gmMissionListFull (def line 88)"
    );
    assert_eq!(
        GM_MISSION_DETAILS,
        109 + 6,
        "gmMissionDetails (def line 92)"
    );
    assert_eq!(LIST_ABILITIES, 109 + 14, "listAbilities (def line 135)");
    assert_eq!(GM_SHOW_FLAG, 109 + 16, "gmShowFlag (def line 144)");
    assert_eq!(
        GM_GET_MOB_ATTRIBUTE,
        109 + 18,
        "gmGetMobAttribute (def line 153)"
    );
    assert_eq!(GM_SHOW_MOB_COUNT, 109 + 19, "gmShowMobCount (def line 159)");
    assert_eq!(GM_GOTO, 109 + 51, "gmGoto (def line 330)");
    assert_eq!(GM_SUMMON, 109 + 52, "gmSummon (def line 335)");
    assert_eq!(GM_DEBUG_MOB_DATA, 109 + 71, "gmDebugMobData (def line 427)");
    assert_eq!(GM_GOTO_LOCATION, 109 + 53, "gmGotoLocation (def line 340)");
    assert_eq!(GM_GOTO_XYZ, 109 + 54, "gmGotoXYZ (def line 348)");
    assert_eq!(GM_DESPAWN_BY_CMD, 109 + 77, "gmDespawnByCmd (def line 461)");
    assert_eq!(GM_RESPAWN, 109 + 80, "gmRespawn (def line 478)");
    assert_eq!(GM_KILL_TARGET, 109 + 81, "gmKillTarget (def line 482)");
    assert_eq!(DESPAWN_MOB, 109 + 104, "despawnMob (def line 605)");
}

/// All implemented indices sit in the GM tail (109 or above), so the
/// dispatch-layer gate (`gm_gate::requires_gm`, which gates every index from
/// 109 up) covers them. A constant that slipped below 109 would be reachable
/// by a non-GM — this pins the invariant.
#[test]
fn implemented_indices_are_in_gm_tail() {
    const GM_TAIL_BASE: u16 = 109;
    for idx in [
        GM_MISSION_CLEAR,
        GM_MISSION_ADVANCE,
        GM_MISSION_ABANDON,
        GM_GIVE_XP,
        GM_GIVE_ITEM,
        GM_GIVE_CASH,
        GM_REMOVE_ITEM,
        GM_GIVE_EXPERTISE,
        GM_GIVE_APPLIED_SCIENCE_POINTS,
        GM_SPAWN_BY_CMD,
        GM_SET_HEALTH,
        GM_SET_HEALTH_MAX,
        GM_SET_FOCUS,
        GM_SET_FOCUS_MAX,
        GM_SET_TARGET,
        GM_DHD,
        GM_GOTO_LOCATION,
        GM_GOTO_XYZ,
        GM_DESPAWN_BY_CMD,
        GM_RESPAWN,
        GM_KILL_TARGET,
        DESPAWN_MOB,
        GM_USERS,
        TEST_LOS,
        GM_SHOW_TARGET_LOCATION,
        GM_SHOW_ROTATION,
        GM_SHOW_PLAYER,
        GM_MISSION_ASSIGN,
        GM_MISSION_LIST,
        GM_MISSION_LIST_FULL,
        GM_MISSION_DETAILS,
        LIST_ABILITIES,
        GM_SHOW_FLAG,
        GM_GET_MOB_ATTRIBUTE,
        GM_SHOW_MOB_COUNT,
        GM_GOTO,
        GM_SUMMON,
        GM_DEBUG_MOB_DATA,
    ] {
        assert!(
            idx >= GM_TAIL_BASE,
            "implemented gm* index {idx} must be in the GM-gated tail (>= 109)"
        );
    }
}

fn mgr_with_player(eid: u32, world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
    );
    mgr.parse_spaces_xml(&xml).unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_entity(eid, world, [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(eid) {
        e.is_player = true;
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }
    mgr
}

/// Drain all currently-queued messages from the channel.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn write_wstring_arg(buf: &mut Vec<u8>, s: &str) {
    crate::mercury::write_wstring(buf, s);
}

fn give_item_args(design_id: &str, qty: i32) -> Vec<u8> {
    let mut args = Vec::new();
    write_wstring_arg(&mut args, design_id);
    args.extend_from_slice(&qty.to_le_bytes());
    args
}

/// Build `(INT32 Amount, INT64 TargetId)`.
fn set_stat_args(amount: i32, target: i64) -> Vec<u8> {
    let mut args = amount.to_le_bytes().to_vec();
    args.extend_from_slice(&target.to_le_bytes());
    args
}

/// Pull the decoded text of the first `onPlayerCommunication` feedback line
/// addressed to `entity_id` (method index 28), if any.
fn feedback_text(msgs: &[CellToBaseMsg], entity_id: u32) -> Option<String> {
    msgs.iter().find_map(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: e,
            method_index: 28,
            args,
        } if *e == entity_id => {
            // Skip speaker WSTRING (u32 len + len*2) + flags + channel, then read text WSTRING.
            let spk = u32::from_le_bytes(args[0..4].try_into().ok()?) as usize;
            let off = 4 + spk * 2 + 2; // + flags + channel
            let tlen = u32::from_le_bytes(args[off..off + 4].try_into().ok()?) as usize;
            let units: Vec<u16> = (0..tlen)
                .map(|i| u16::from_le_bytes([args[off + 4 + i * 2], args[off + 5 + i * 2]]))
                .collect();
            Some(String::from_utf16_lossy(&units))
        }
        _ => None,
    })
}

/// An unimplemented 109+ index returns `false` so the router falls through to
/// its (already-authorized) warn arm — no panic.
#[tokio::test]
async fn unimplemented_gm_index_returns_false() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, _rx) = mpsc::channel(8);
    // 142 = gmSetGodMode — in the tail, not implemented here.
    assert!(!dispatch(1, 142, &[], &tx, &mut mgr).await);
}

mod give;
mod missions;
mod query;
mod spawn;
mod stats;
mod travel;
mod world;
