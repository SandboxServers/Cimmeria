use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

/// Empty content engine for `dispatch`'s `engine` parameter.
///
/// Only `gmDHD` reads it today (the `stargate_dialed` content trigger),
/// and with no chains registered every `fire_*` is a no-op — so every
/// other arm's assertions are unaffected by it.
pub(super) fn test_engine() -> cimmeria_content_engine::chain::ChainEngine {
    cimmeria_content_engine::chain::ChainEngine::new()
}

/// The three GM indices seen on the wire in a pcap, independent of any
/// `.def` counting. Every GM constant is derived from `SGWGmPlayer.def` by
/// `cimmeria-wire`'s `mercury::def_conformance` (#801), which replaced the
/// hand-counted `109 + N` literals this test used to hold; these three stay
/// because they are evidence from the client, not from the defs.
#[test]
fn pcap_anchored_gm_indices() {
    assert_eq!(GM_GIVE_ITEM, 133, "gmGiveItem, pcap-anchored");
    assert_eq!(GM_GOTO_XYZ, 163, "gmGotoXYZ, pcap-anchored");
    assert_eq!(GM_KILL_TARGET, 190, "gmKillTarget, pcap-anchored");
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
        GM_GIVE_TRAINING_POINTS,
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
        GM_RELOAD_ORGANIZATIONS,
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
        GM_PHYSICS,
        GM_SEND_GM_SHOUT,
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
    assert!(!dispatch(1, 142, &[], &tx, &mut mgr, &test_engine()).await);
}

mod give;
mod give_training_points;
mod missions;
mod organizations;
mod physics;
mod query;
mod shout;
mod spawn;
mod stats;
mod travel;
mod travel_entry_point;
mod travel_pets;
mod world;
